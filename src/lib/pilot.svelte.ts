import { models } from './catalogs.svelte'
import { families } from './commands'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ask } from './asking.svelte'
import { askValues, branchOf, create, find, importSession, knownSessions, ofProject, persist, pin, send, waitingFor, type Elsewhere } from './agents.svelte'
import { moveCard, type Card, type Column } from './board'
import { all, arrangeSlot, hidden, labelOf } from './layout-slots.svelte'
import { editProject, projects, setCards, type Project } from './projects.svelte'
import { saveSettings, settings } from './settings.svelte'
import { SLOTS, type Slot } from './slots'
import { REVIEW_PROMPT, reviewPrompt, reviews } from './pulls.svelte'
import { workdir, type Agent, type Options } from './stream'
import { english, t } from './i18n/index.svelte'
import { PANEL_MAX_HTML, panelHeight } from './infopanel'
import { ARTIFACT_MAX, ARTIFACT_KINDS, DOC_LINK } from './artifacts'

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

const waitingOf = (a: Agent) => (a.asks[0] ? waitingFor(a.asks[0], english) : a.attention) || undefined

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
        pinned: !!a.pinned,
        waiting_for_user: waitingOf(a),
        doing: a.status === 'working' ? a.action : undefined,
        cost_usd: Math.round(a.cost * 100) / 100,
        progress: a.progress ? `${a.progress.done}/${a.progress.total}${a.progress.message ? ` · ${a.progress.message}` : ''}` : undefined,
        tasks: a.tasks.length ? `${a.tasks.filter((t) => t.status === 'completed').length}/${a.tasks.length}` : undefined,
        goal: goalOf(a),
      })),
      cards: p.cards.map((c) => ({ id: c.id, title: c.title, prompt: c.prompt || undefined, goal: c.goal || undefined, column: c.column, agent: c.agent ?? undefined })),
    })),

  read_agent: (_, args) => {
    const agent = find(String(args.agent))
    if (!agent) throw new Error(`No agent with id ${args.agent}. Call overview to list them.`)
    return { name: agent.name, status: agent.status, waiting_for_user: waitingOf(agent), goal: goalOf(agent), messages: text(agent, Number(args.last) || 10) }
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
      !(await ask(t(name ? 'shell.pilotStartNamed' : 'shell.pilotStartUnnamed', { caller: caller.name, name, project: project.name, mode: options.mode }), {
        title: t('shell.pilotStartTitle'),
        kind: 'warning',
        okLabel: t('shell.pilotStartOk'),
        cancelLabel: t('shell.pilotStartRefuse'),
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

  set_pinned: (caller, args) => {
    const target = args.agent ? find(String(args.agent)) : caller
    if (!target) throw new Error(`No agent with id ${args.agent}. Call overview to list them.`)
    if (target.project !== caller.project) throw new Error(`${target.name} belongs to another project. You can only pin the agents of your own project.`)
    if (typeof args.pinned !== 'boolean') throw new Error('pinned must be true or false.')
    pin(target.id, args.pinned)
    const who = target === caller ? 'You are' : `${target.name} is`
    return args.pinned ? `${who} pinned at the top of the agent list.` : `${who} no longer pinned.`
  },

  set_progress: (caller, args) => {
    if (args.total === null || args.total === undefined) {
      caller.progress = null
      persist(caller)
      return 'Progress bar removed from your card.'
    }
    const total = Number(args.total)
    const done = Number(args.done ?? 0)
    if (!Number.isFinite(total) || total <= 0) throw new Error('total must be a number above 0.')
    if (!Number.isFinite(done) || done < 0 || done > total) throw new Error(`done must be between 0 and ${total}.`)
    const place = pick<'above' | 'below'>(args.place, ['above', 'below'], 'above', 'place')
    caller.progress = { done, total, message: String(args.message ?? '').trim(), place }
    persist(caller)
    return `Your card shows ${done}/${total}. Call set_progress again as the work moves on, or without total to remove it.`
  },

  message_agent: async (caller, args) => {
    const target = find(String(args.agent ?? ''))
    if (!target) throw new Error(`No agent with id ${args.agent}. Call overview to list them.`)
    if (target === caller) throw new Error('That agent is you.')
    projectFor(caller, target.project)
    const message = String(args.message ?? '').trim()
    if (!message) throw new Error('The message is empty.')
    if (target.asks.length) throw new Error(`${target.name} waits for the user to answer a question or a permission request. Only the user can answer it: tell them.`)
    await send(target.id, message, [], [], caller.name)
    return `Sent to ${target.name}. ${target.status === 'working' ? 'It reads it once its current action ends.' : 'It starts on it now.'} Read its answer later with read_agent.`
  },

  import_session: async (caller, args) => {
    const project = projectFor(caller, args.project)
    const [found, known] = await Promise.all([invoke<Elsewhere[]>('project_sessions', { project: project.path }), knownSessions()])
    const wanted = String(args.session ?? '').trim()
    if (!wanted)
      return found
        .filter((f) => !known.has(f.id))
        .slice(0, 50)
        .map((f) => ({ session: f.id, title: f.title, updated: new Date(f.updated).toISOString(), from: f.app ? 'Claude app' : 'Claude Code', archived: f.archived || undefined }))
    const pick = found.find((f) => f.id === wanted)
    if (!pick) throw new Error(`No conversation ${wanted} in ${project.name}. Call import_session without session to list them.`)
    if (known.has(pick.id)) throw new Error(`“${pick.title}” is already in Squadron.`)
    const agent = await importSession(project.path, pick)
    return `Imported “${agent.name}” in ${project.name} as agent ${agent.id}. It continues the conversation, in a fork, from its next message.`
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
    const count = `${reviews[key].length} comment${reviews[key].length > 1 ? 's' : ''}`
    return `Added to the user's draft review of pull request #${caller.review.number} (${count} so far). The user sees it in Squadron: Pull requests, then Review on #${caller.review.number}; your header chip leads there too.`
  },

  get_review_prompt: (caller, args) => {
    const project = projectFor(caller, args.project)
    return {
      global: settings.review_prompt || REVIEW_PROMPT,
      global_is_default: !settings.review_prompt || undefined,
      project: project.review_prompt || undefined,
      used_for_this_project: reviewPrompt(project.path),
    }
  },

  set_review_prompt: async (caller, args) => {
    const scope = pick<'global' | 'project'>(args.scope, ['global', 'project'], 'project', 'scope')
    const prompt = String(args.prompt ?? '').trim()
    if (scope === 'global') {
      await saveSettings({ ...$state.snapshot(settings), review_prompt: prompt === REVIEW_PROMPT ? '' : prompt })
      return prompt ? 'The global review prompt is set. Projects with their own prompt keep it.' : 'The global review prompt is back to Squadron’s default.'
    }
    const project = projectFor(caller, args.project)
    await editProject(project.path, { review_prompt: prompt })
    return prompt ? `${project.name} now has its own review prompt.` : `${project.name} follows the global review prompt again.`
  },

  set_info_panel: (caller, args) => {
    const html = String(args.html ?? '').trim()
    if (!html) {
      caller.infoPanel = undefined
      persist(caller)
      return 'Your panel is removed.'
    }
    if (html.length > PANEL_MAX_HTML) throw new Error(`The html is ${html.length} characters long; keep it under ${PANEL_MAX_HTML}.`)
    const height = panelHeight(args.height)
    const hidden = caller.infoPanel?.hidden
    caller.infoPanel = { html, height, hidden }
    persist(caller)
    return `Your panel shows under the header of your conversation, ${height}px high.${hidden ? ' The user has hidden it: they see it when they show it again.' : ''}`
  },

  publish_artifact: (caller, args) => {
    const id = String(args.id ?? '').trim()
    const content = String(args.content ?? '')
    const link = String(args.link ?? '').trim()
    const list = caller.artifacts ?? []
    const found = id ? list.find((a) => a.id === id) : undefined
    if (id && !found) throw new Error(`You have no artifact ${id}. Your artifacts: ${list.map((a) => `${a.id} “${a.title}”`).join(', ') || 'none'}.`)
    if (link && !DOC_LINK.test(link)) throw new Error(`${link} is not the link of a claude.ai doc.`)
    if (!content.trim()) {
      if (!found) throw new Error('The content is empty.')
      if (link) {
        caller.artifacts = list.map((a) => (a === found ? { ...a, link } : a))
        persist(caller)
        return `Artifact ${id} now links to ${link}: the user opens and shares it from the Artifacts panel.`
      }
      caller.artifacts = list.filter((a) => a !== found)
      persist(caller)
      return `Artifact ${id} deleted.`
    }
    if (content.length > ARTIFACT_MAX) throw new Error(`The content is ${content.length} characters long; keep it under ${ARTIFACT_MAX}.`)
    const kind = pick(args.kind, ARTIFACT_KINDS, found?.kind ?? 'markdown', 'kind')
    const title = String(args.title ?? '').trim() || found?.title || ''
    if (!title) throw new Error('The title is empty.')
    const artifact = { id: found?.id ?? crypto.randomUUID().slice(0, 8), title, kind, content, version: (found?.version ?? 0) + 1, at: Date.now(), link: link || found?.link }
    caller.artifacts = found ? list.map((a) => (a === found ? artifact : a)) : [...list, artifact]
    persist(caller)
    return `Artifact ${artifact.id} “${title}” published${found ? ` as version ${artifact.version}` : ''}. The user sees it in the Artifacts panel of your conversation. Call publish_artifact with id ${artifact.id} to update it.`
  },

  ask_values: async (caller, args) => {
    const path = String(args.path ?? '').trim()
    if (!path) throw new Error('The path is empty.')
    const fields = ((args.fields ?? []) as { key?: unknown; hint?: unknown; secret?: unknown; line?: unknown }[]).map((f) => ({
      key: String(f.key ?? '').trim(),
      hint: String(f.hint ?? '').trim() || undefined,
      secret: f.secret === true,
      line: String(f.line ?? '').trim() ? String(f.line) : '',
    }))
    if (!fields.length || fields.some((f) => !f.key)) throw new Error('Give each field a key.')
    const mark = (key: string) => `{{squadron:${key}}}`
    const bad = fields.find((f) => f.line && !f.line.includes(mark(f.key)))
    if (bad) throw new Error(`The line of ${bad.key} must hold ${mark(bad.key)} where the value goes.`)
    const root = workdir(caller)
    const read = () => invoke<string | null>('read_project_file', { project: root, path }).then((text) => text ?? '')
    await read()
    const values = await askValues(caller, path, fields)
    if (typeof values === 'string') return `The user closed the card without filling ${path}${values ? `, and wrote: ${values}` : ''}. The file is unchanged.`
    let text = await read()
    for (const f of fields.filter((f) => f.key in values)) {
      if (!text.includes(mark(f.key))) text += `${text && !text.endsWith('\n') ? '\n' : ''}${f.line || `${f.key}=${mark(f.key)}`}\n`
      text = text.split(mark(f.key)).join(values[f.key])
    }
    await invoke('write_project_file', { project: root, path, text })
    const left = fields.filter((f) => !(f.key in values)).map((f) => f.key)
    return `Filled ${Object.keys(values).join(', ')} in ${path}.${left.length ? ` The user left ${left.join(', ')} empty.` : ''} You never see these values: do not read them back, print them or copy them elsewhere.`
  },


  get_layout: () =>
    (Object.keys(SLOTS) as Slot[]).map((slot) => ({
      slot,
      title: english(SLOTS[slot].title),
      where: english(SLOTS[slot].hint),
      items: all(slot).map((id) => ({ id, label: labelOf(slot, id, english), hidden: hidden(slot, id) || undefined })),
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
