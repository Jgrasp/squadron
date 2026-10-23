import { models } from './catalogs.svelte'
import { families } from './commands'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ask } from '@tauri-apps/plugin-dialog'
import { branchOf, create, find, ofProject, persist } from './agents.svelte'
import { moveCard, type Card, type Column } from './board'
import { all, arrangeSlot, hidden, labelOf } from './layout-slots.svelte'
import { projects, setCards, type Project } from './projects.svelte'
import { settings } from './settings.svelte'
import { SLOTS, type Slot } from './slots'
import { reviews } from './pulls.svelte'
import { workdir, type Agent, type Options } from './stream'

type Call = { id: string; agent: string; tool: string; args: Record<string, any> }

const EFFORTS = ['low', 'medium', 'high', 'xhigh', 'max']
const MODES = ['default', 'plan', 'acceptEdits', 'auto']
const COLUMNS: Column[] = ['todo', 'progress', 'review', 'done']

function pick<T extends string>(value: unknown, allowed: string[], fallback: T, what: string): T {
  if (value === null || value === undefined || value === '') return fallback
  if (!allowed.includes(String(value))) throw new Error(`Unknown ${what} “${value}”. Use one of: ${allowed.join(', ')}.`)
  return value as T
}

function projectFor(caller: Agent, wanted: unknown): Project {
  const own = projects.list.find((p) => p.path === caller.project)!
  if (!wanted) return own
  const found = projects.list.find((p) => p.path === wanted || p.name.toLowerCase() === String(wanted).toLowerCase())
  if (!found) throw new Error(`No project “${wanted}”. Projects: ${projects.list.map((p) => p.name).join(', ')}.`)
  if (found !== own && !settings.pilot_all)
    throw new Error(`You can only act on your own project (${own.name}). The user can allow every project in Settings › Preferences.`)
  return found
}

function text(agent: Agent, last: number) {
  return agent.entries
    .flatMap((e) =>
      e.kind === 'user'
        ? [`User: ${e.text}`]
        : e.kind === 'agent'
          ? e.blocks.flatMap((b) => (b.type === 'text' && b.text.trim() ? [`${agent.name}: ${b.text.trim()}`] : []))
          : [],
    )
    .slice(-last)
}

const goalOf = (a: Agent) =>
  a.goal ? { done_when: a.goal.condition, state: a.goal.state, relaunches: a.goal.relaunches || undefined, last_reason: a.goal.reason || undefined } : undefined

const tools: Record<string, (caller: Agent, args: Record<string, any>) => unknown> = {
  overview: () =>
    projects.list.map((p) => ({
      project: p.name,
      path: p.path,
      agents: ofProject(p.path).map((a) => ({
        id: a.id,
        name: a.name,
        status: a.status,
        waiting_for_user: a.attention || undefined,
        doing: a.status === 'working' ? a.action : undefined,
        cost_usd: Math.round(a.cost * 100) / 100,
        tasks: a.tasks.length ? `${a.tasks.filter((t) => t.status === 'completed').length}/${a.tasks.length}` : undefined,
        goal: goalOf(a),
      })),
      cards: p.cards.map((c) => ({ id: c.id, title: c.title, prompt: c.prompt || undefined, goal: c.goal || undefined, column: c.column, agent: c.agent ?? undefined })),
    })),

  read_agent: (_, args) => {
    const agent = find(String(args.agent))
    if (!agent) throw new Error(`No agent with id ${args.agent}. Call overview to list them.`)
    return { name: agent.name, status: agent.status, waiting_for_user: agent.attention || undefined, goal: goalOf(agent), messages: text(agent, Number(args.last) || 10) }
  },

  start_agent: async (caller, args) => {
    const project = projectFor(caller, args.project)
    const card = args.card ? project.cards.find((c) => c.id === args.card) : undefined
    if (args.card && !card) throw new Error(`No card ${args.card} in ${project.name}.`)
    const text = String(args.prompt ?? '').trim() || card?.prompt || card?.title || ''
    if (!text) throw new Error('The prompt is empty.')
    if (!project.exists) throw new Error(`The folder of ${project.name} cannot be found.`)
    const options: Options = {
      model: pick(args.model, [...families(models()), ...models().map((m) => m.value)].filter((v) => v !== 'default'), settings.defaults.model, 'model'),
      effort: pick(args.effort, EFFORTS, settings.defaults.effort, 'effort'),
      mode: pick(args.mode, MODES, settings.defaults.mode, 'mode'),
    }
    const name = String(args.name ?? '').trim() || card?.title || ''
    const goal = String(args.done_when ?? '').trim() || card?.goal || ''
    if (
      (options.mode === 'acceptEdits' || options.mode === 'auto') &&
      !(await ask(`${caller.name} wants to start ${name ? `“${name}”` : 'an agent'} in ${project.name} in ${options.mode} mode, which skips some permission requests.`, {
        title: 'Start an agent',
        kind: 'warning',
        okLabel: 'Start it',
        cancelLabel: 'Refuse',
      }))
    )
      return `The user refused to start this agent in ${options.mode} mode.`
    const dir = String(args.worktree ?? '').trim().replace(/\/+$/, '')
    if (dir) await invoke('git_branch', { path: dir }).catch(() => {
      throw new Error(`${dir} is not a git checkout.`)
    })
    const id = await create(project.path, name, options, text, { from: { by: caller.name }, goal, dir: dir || undefined })
    if (card) await setCards(project.path, project.cards.map((c) => (c.id === card.id ? { ...c, agent: id, column: 'progress' } : c)))
    const until = goal ? ` It works until this goal is met: ${goal}${options.mode === 'plan' ? ' (once the user approves its plan)' : ''}.` : ''
    return `Started agent ${id} in ${project.name}.${until} Its permission requests and questions go to the user.`
  },

  create_card: async (caller, args) => {
    if (!String(args.title ?? '').trim()) throw new Error('The title is empty.')
    const project = projectFor(caller, args.project)
    const card: Card = {
      id: crypto.randomUUID(),
      key: '',
      title: String(args.title).trim(),
      prompt: String(args.prompt ?? '').trim(),
      goal: String(args.done_when ?? '').trim(),
      column: 'todo',
      agent: null,
    }
    await setCards(project.path, [...project.cards, card])
    return `Card ${card.id} added to the To do column of ${project.name}.`
  },

  move_card: async (caller, args) => {
    const column = pick<Column>(args.column, COLUMNS, 'todo', 'column')
    const project = projects.list.find((p) => p.cards.some((c) => c.id === args.card))
    if (!project) throw new Error(`No card with id ${args.card}. Call overview to list them.`)
    projectFor(caller, project.path)
    await setCards(project.path, moveCard(project.cards, String(args.card), column))
    return `Card moved to ${column}.`
  },

  set_worktree: async (caller, args) => {
    const target = args.agent ? find(String(args.agent)) : caller
    if (!target) throw new Error(`No agent with id ${args.agent}. Call overview to list them.`)
    projectFor(caller, target.project)
    const who = target === caller ? 'you work' : `${target.name} works`
    const path = String(args.path ?? '').trim().replace(/\/+$/, '')
    if (!path || path === target.project) {
      target.worktree = null
      await invoke('set_workdir', { id: target.id, dir: null })
      persist(target)
      return `Squadron shows that ${who} in the main checkout again.`
    }
    const branch = await invoke<string | null>('git_branch', { path }).catch(() => {
      throw new Error(`${path} is not a git checkout.`)
    })
    target.worktree = { path, branch: branch ?? '' }
    await invoke('set_workdir', { id: target.id, dir: path })
    persist(target)
    return `Squadron now shows that ${who} in the worktree ${path}${branch ? ` on ${branch}` : ''}, and starts ${target === caller ? 'you' : 'it'} there next time.`
  },

  add_review_comment: async (caller, args) => {
    if (!caller.review) throw new Error('You were not asked to review a pull request.')
    const path = String(args.path ?? '').trim().replace(/^\/+/, '')
    const line = path ? Number(args.line) : 0
    const body = String(args.body ?? '').trim()
    if (!body) throw new Error('The comment is empty.')
    if (path && (!Number.isInteger(line) || line < 1)) throw new Error('A comment on a file needs the line it is about. Leave path and line out to comment on the whole pull request.')
    if (path && caller.review.base) {
      const files = await invoke<string[]>('changed_files', { dir: workdir(caller), base: caller.review.base }).catch(() => null)
      if (files && !files.includes(path))
        throw new Error(
          `${path} is not changed by the pull request. Put the comment on the changed line it relates to best, in one of these files: ${files.join(', ')}. Or leave path and line out to comment on the whole pull request.`,
        )
    }
    const removed = !!path && args.removed === true
    const text = !path || removed ? null : await invoke<string | null>('read_project_file', { project: workdir(caller), path }).catch(() => null)
    const key = `${caller.review.project}#${caller.review.number}`
    reviews[key] = [...(reviews[key] ?? []), { path, line, removed, code: text?.split('\n')[line - 1] ?? '', note: body, pull: caller.review.number }]
    return `Added to the user's review of pull request #${caller.review.number} (${reviews[key].length} comment${reviews[key].length > 1 ? 's' : ''} so far).`
  },

  get_layout: () =>
    (Object.keys(SLOTS) as Slot[]).map((slot) => ({
      slot,
      title: SLOTS[slot].title,
      where: SLOTS[slot].hint,
      items: all(slot).map((id) => ({ id, label: labelOf(slot, id), hidden: hidden(slot, id) || undefined })),
      cannot_hide: SLOTS[slot].locked,
    })),

  set_layout: (_, args) => {
    if (!(args.slot in SLOTS)) throw new Error(`Unknown slot “${args.slot}”. Slots: ${Object.keys(SLOTS).join(', ')}.`)
    arrangeSlot(args.slot as Slot, args.order, args.hidden)
    return `Layout of ${args.slot} updated: ${all(args.slot as Slot).join(', ')}.`
  },
}

listen<Call>('pilot-request', async ({ payload: call }) => {
  let result: unknown
  try {
    const caller = find(call.agent)
    if (!caller) throw new Error('Only agents started by Squadron can drive it.')
    if (!tools[call.tool]) throw new Error(`Unknown tool ${call.tool}.`)
    result = await tools[call.tool](caller, call.args ?? {})
  } catch (e) {
    result = `Error: ${e instanceof Error ? e.message : e}`
  }
  await invoke('pilot_reply', { id: call.id, result: typeof result === 'string' ? result : JSON.stringify(result, null, 2) })
})
