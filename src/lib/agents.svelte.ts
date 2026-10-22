import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { remember } from './catalogs.svelte'
import { EDITS } from './diff'
import { addRule, editProject, projects, type Project } from './projects.svelte'
import { ask } from '@tauri-apps/plugin-dialog'
import { matches, ruleFor, ruleLabel } from './rules'
import {
  apply,
  argument,
  subagentOf,
  INTERNAL,
  READ_ONLY,
  exited,
  fail,
  newAgent,
  prompt,
  reloaded,
  userMessage,
  type Agent,
  type Ask,
  type Comment,
  type Entry,
  type Options,
} from './stream'

export const agents = $state({ list: [] as Agent[], current: {} as Record<string, string> })

export const clock = $state({ now: Date.now() })

const dayOf = (at: number) => new Date(at).toLocaleDateString('sv')

export const today = $state({ day: dayOf(Date.now()), usd: 0 })

export function todayCost() {
  return today.day === dayOf(clock.now) ? today.usd : 0
}

export async function loadToday() {
  today.day = dayOf(Date.now())
  today.usd = await invoke<number>('day_cost', { day: today.day })
}

async function spend(usd: number) {
  const day = dayOf(Date.now())
  const total = await invoke<number>('add_cost', { day, usd })
  today.day = day
  today.usd = total
}

export function waitingAgents() {
  return agents.list.filter((a) => a.attention).sort((a, b) => a.since - b.since)
}

export function show(agent: Agent) {
  projects.overview = false
  projects.board = false
  projects.current = agent.project
  agents.current[agent.project] = agent.id
}

export function jumpToWaiting() {
  const queue = waitingAgents()
  if (!queue.length) return false
  const shown = projects.overview ? undefined : selected(projects.current)
  const at = queue.findIndex((a) => a.id === shown?.id)
  show(queue[(at + 1) % queue.length])
  return true
}
setInterval(() => (clock.now = Date.now()), 1000)

export function find(id: string) {
  return agents.list.find((a) => a.id === id)
}

export function ofProject(project: string) {
  return agents.list.filter((a) => a.project === project)
}

export function waiting(project: string) {
  return agents.list.filter((a) => a.project === project && a.attention).length
}

export function selected(project: string) {
  const list = ofProject(project)
  return list.find((a) => a.id === agents.current[project]) ?? list[0]
}

listen<{ id: string; line: string }>('agent-line', ({ payload }) => {
  const agent = find(payload.id)
  if (!agent) return
  const d = JSON.parse(payload.line)
  const before = agent.cost
  apply(agent, d)
  if (agent.cost > before) spend(agent.cost - before)
  if (d.type === 'result') invoke('cancel_permissions', { agent: agent.id })
  if (d.type === 'result' || d.subtype === 'init') persist(agent)
  if (d.type === 'control_response' && d.response?.response?.commands) remember(agent.project, d.response.response)
})

const timers = new Map<string, number>()

export function persist(agent: Agent) {
  clearTimeout(timers.get(agent.id))
  timers.set(agent.id, window.setTimeout(() => save(agent), 600))
}

function save(agent: Agent, extra: Record<string, unknown> = {}) {
  clearTimeout(timers.get(agent.id))
  timers.delete(agent.id)
  const { live, alive, processCost, interrupting, asks, ...kept } = $state.snapshot(agent)
  const subagents = kept.subagents.map(({ log, ...s }) => ({ ...s, log: [] }))
  return invoke('save_agent', { id: agent.id, data: JSON.stringify({ ...kept, subagents, ...extra }) })
}

export function saveAll() {
  return Promise.all(agents.list.map((a) => save(a)))
}

export async function restore() {
  const known = new Set([...projects.list, ...projects.archived].map((p) => p.path))
  const saved = (await invoke<string[]>('load_agents')).flatMap((text) => {
    try {
      return [JSON.parse(text) as Agent]
    } catch {
      return []
    }
  })
  saved.sort((a, b) => (a.entries[0]?.kind === 'user' ? a.entries[0].at : 0) - (b.entries[0]?.kind === 'user' ? b.entries[0].at : 0))
  for (const data of saved) {
    if (find(data.id)) continue
    if (!known.has(data.project)) invoke('delete_agent_file', { id: data.id })
    else await adopt(data)
  }
}

async function adopt(data: Agent) {
  const wanted = data.remote
  const agent: Agent = {
    ...newAgent(data.id, data.project, data.name, data.options),
    ...data,
    asks: [],
    alive: false,
    remote: wanted ? { state: 'connecting', url: wanted.url } : null,
  }
  if (agent.status === 'working' || agent.status === 'waiting') {
    agent.status = 'interrupted'
    agent.action = 'Stopped when Squadron closed'
  }
  for (const s of agent.subagents) if (s.status === 'running') s.status = 'stopped'
  agents.list.push(agent)
  await invoke('restore_agent', {
    id: agent.id,
    project: agent.project,
    options: agent.options,
    sessionId: agent.sessionId,
    fork: !!agent.forkOf,
  }).catch((e) => fail(find(agent.id)!, String(e)))
  if (wanted) remote(agent.id, true)
}

export type Archived = Agent & { archived?: number; withProject?: boolean }

export async function archived() {
  return (await invoke<string[]>('list_archived'))
    .flatMap((text) => {
      try {
        return [JSON.parse(text) as Archived]
      } catch {
        return []
      }
    })
    .sort((a, b) => (b.archived ?? 0) - (a.archived ?? 0))
}

export async function unarchive(id: string) {
  const { archived, withProject, ...data } = JSON.parse(await invoke<string>('restore_archived', { id })) as Archived
  await adopt(data)
  agents.current[data.project] = data.id
}

export function forget(id: string) {
  return invoke('delete_archived', { id })
}

listen<{ id: string; stderr: string }>('agent-exit', ({ payload }) => {
  const agent = find(payload.id)
  if (!agent) return
  exited(agent, payload.stderr)
  invoke('cancel_permissions', { agent: agent.id })
  persist(agent)
})

listen<Ask>('permission-request', ({ payload: ask }) => {
  const agent = find(ask.agent)
  if (!agent) return decide(ask.id, { behavior: 'deny', message: 'Unknown Squadron agent.' })
  const rules = projects.list.find((p) => p.path === agent.project)?.rules ?? []
  if (
    INTERNAL.includes(ask.tool_name) ||
    READ_ONLY.includes(ask.tool_name) ||
    (agent.allowEdits && EDITS.includes(ask.tool_name)) ||
    rules.some((r) => matches(r, ask.tool_name, ask.input))
  )
    return decide(ask.id, { behavior: 'allow', updatedInput: ask.input })
  agent.asks.push(ask)
  waitFor(agent)
})

export function toolLabel(name: string) {
  const mcp = /^mcp__(.+?)__(.+)$/.exec(name)
  return mcp ? `${mcp[1]} · ${mcp[2]}` : name
}

function waitFor(agent: Agent) {
  const ask = agent.asks[0]
  if (!ask) {
    agent.status = 'working'
    agent.action = 'Resuming…'
    agent.attention = ''
    return
  }
  agent.status = 'waiting'
  agent.action =
    ask.tool_name === 'AskUserQuestion'
      ? 'Waiting for your answer · Question'
      : ask.tool_name === 'ExitPlanMode'
        ? 'Waiting for your approval · Plan'
        : `Waiting for your approval · ${toolLabel(ask.tool_name).split(' · ')[0]}`
  agent.attention = agent.action
}

export function waitingDetail(agent: Agent) {
  const ask = agent.asks[0]
  if (!ask) return ''
  if (ask.tool_name === 'AskUserQuestion') return String(ask.input.questions?.[0]?.question ?? '')
  if (ask.tool_name === 'ExitPlanMode')
    return (String(ask.input.plan ?? '').split('\n').find((l) => l.trim()) ?? '').replace(/^#+\s*/, '')
  const sub = subagentOf(agent, ask.tool_use_id)
  const what = ask.tool_name.startsWith('mcp__') ? toolLabel(ask.tool_name) : argument(agent, { name: ask.tool_name, input: ask.input }) || toolLabel(ask.tool_name)
  return sub ? `subagent #${sub.num} · ${what}` : what
}

function decide(id: string, decision: Record<string, unknown>) {
  return invoke('answer_permission', { id, decision })
}

function settle(agent: Agent, ask: Ask, decision: Record<string, unknown>, entry: Omit<Extract<Entry, { kind: 'decision' }>, 'kind' | 'at'>) {
  agent.asks = agent.asks.filter((a) => a.id !== ask.id)
  agent.entries.push({ kind: 'decision', at: Date.now(), ...entry })
  waitFor(agent)
  persist(agent)
  return decide(ask.id, decision).catch((e) => fail(agent, String(e)))
}

const detailOf = (agent: Agent, ask: Ask) =>
  `${toolLabel(ask.tool_name)} · ${argument(agent, { name: ask.tool_name, input: ask.input })}`.replace(/ · $/, '')

export async function allow(agent: Agent, ask: Ask, always = false) {
  let note: string | undefined
  if (always && EDITS.includes(ask.tool_name)) {
    agent.allowEdits = true
    note = `${agent.name} edits files without asking until its session ends`
  } else if (always) {
    const rule = ruleFor(ask.tool_name, ask.input)
    await addRule(agent.project, rule)
    note = `Rule added for ${projects.list.find((p) => p.path === agent.project)?.name}: ${ruleLabel(rule)}`
  }
  return settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: always ? 'Always allowed' : 'Allowed once',
    detail: detailOf(agent, ask),
    note,
  })
}

export function deny(agent: Agent, ask: Ask, message = '') {
  return settle(
    agent,
    ask,
    { behavior: 'deny', message: message || 'The user denied this.' },
    { tone: 'bad', title: 'Denied', detail: detailOf(agent, ask), note: message ? `With your instruction: “${message}”` : undefined },
  )
}

export function answer(agent: Agent, ask: Ask, answers: Record<string, string>) {
  return settle(
    agent,
    ask,
    { behavior: 'allow', updatedInput: { ...ask.input, answers } },
    {
      tone: 'ok',
      title: 'Answered',
      detail: Object.entries(answers)
        .map(([q, a]) => `${q} → ${a.replace(/\s*\((recommended|recommandé)\)/gi, '')}`)
        .join(' · '),
      questions: (ask.input.questions ?? []).map((q: { question: string; options?: { label: string }[] }) => ({
        question: q.question,
        options: (q.options ?? []).map((o) => o.label),
        answer: answers[q.question] ?? '',
      })),
    },
  )
}

export function plan(agent: Agent, ask: Ask, choice: 'edits' | 'ask' | 'continue', remark = '') {
  if (choice === 'continue')
    return settle(
      agent,
      ask,
      { behavior: 'deny', message: remark || 'Keep planning, do not change anything yet.' },
      { tone: 'bad', title: 'Plan to revise', detail: 'Keep planning', note: remark ? `With your feedback: “${remark}”` : undefined },
    )
  if (choice === 'edits') agent.allowEdits = true
  return settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: 'Plan approved',
    detail: choice === 'edits' ? 'Started in “accept edits”' : 'Started, asking for each edit',
  })
}

export function reply(agent: Agent, text: string) {
  const ask = agent.asks[0]
  if (!ask) return false
  if (ask.tool_name === 'AskUserQuestion') {
    const answers: Record<string, string> = {}
    for (const q of ask.input.questions ?? []) answers[q.question] = text
    answer(agent, ask, answers)
  } else if (ask.tool_name === 'ExitPlanMode') plan(agent, ask, 'continue', text)
  else deny(agent, ask, text)
  return true
}

export async function create(project: string, name: string, options: Options, text: string, from?: { by: string }) {
  const fallback = text.trim().split(/\s+/).slice(0, 5).join(' ')
  agents.list.push(newAgent(crypto.randomUUID(), project, name || fallback, options))
  const agent = agents.list.at(-1)!
  if (!name) {
    agent.autoName = true
    invoke<string>('suggest_title', { text })
      .then((title) => {
        if (!agent.autoName) return
        agent.name = title
        agent.autoName = false
        persist(agent)
      })
      .catch(() => {})
  }
  if (from) agent.entries.push({ kind: 'launched', at: Date.now(), by: from.by })
  else agents.current[project] = agent.id
  userMessage(agent, text)
  persist(agent)
  try {
    await invoke('create_agent', { id: agent.id, project, options, text })
  } catch (e) {
    fail(agent, String(e))
  }
  return agent.id
}

export async function send(id: string, text: string, comments: Comment[] = []) {
  const agent = find(id)
  if (!agent) return
  userMessage(agent, text, comments)
  persist(agent)
  try {
    await invoke('send_message', { id, text: prompt(text, comments) })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function retry(id: string) {
  const agent = find(id)
  const last = agent?.entries.findLast((e) => e.kind === 'user')
  if (!agent || last?.kind !== 'user') return
  agent.status = 'working'
  agent.action = 'Starting…'
  try {
    await invoke('send_message', { id, text: prompt(last.text, last.comments) })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function interrupt(id: string) {
  const agent = find(id)
  if (!agent || (agent.status !== 'working' && agent.status !== 'waiting')) return
  agent.interrupting = true
  agent.asks = []
  agent.attention = ''
  await invoke('interrupt_agent', { id })
  await invoke('cancel_permissions', { agent: id })
}

export async function reload(id: string) {
  const agent = find(id)
  if (!agent) return
  try {
    await invoke('reload_agent', { id })
    reloaded(agent)
    persist(agent)
  } catch (e) {
    fail(agent, String(e))
  }
}

export function note(id: string, command: string, text: string) {
  const agent = find(id)
  if (!agent) return
  agent.entries.push({ kind: 'note', at: Date.now(), command, text })
  persist(agent)
}

export async function setOptions(id: string, options: Options) {
  const agent = find(id)
  if (!agent) return
  agent.options = options
  persist(agent)
  await invoke('set_agent_options', { id, options }).catch((e) => fail(agent, String(e)))
}

export async function remote(id: string, enabled: boolean) {
  const agent = find(id)
  if (!agent) return
  if (!enabled) agent.remote = null
  await invoke('remote_control', { id, enabled }).catch((e) => fail(agent, String(e)))
}

export function rename(id: string, name: string) {
  const agent = find(id)
  if (!agent || !name.trim()) return
  agent.name = name.trim()
  agent.autoName = false
  persist(agent)
}

export async function fork(id: string) {
  const source = find(id)
  if (!source?.sessionId) return
  const copy = newAgent(crypto.randomUUID(), source.project, `${source.name} (fork)`, $state.snapshot(source.options))
  const snapshot = $state.snapshot(source)
  copy.entries = [...snapshot.entries, { kind: 'forked', at: Date.now(), from: source.name }]
  copy.tasks = snapshot.tasks
  copy.subagents = snapshot.subagents
  copy.sessionId = source.sessionId
  copy.forkOf = source.name
  copy.status = 'done'
  copy.action = `Forked from ${source.name}`
  agents.list.push(copy)
  const agent = agents.list.at(-1)!
  agents.current[agent.project] = agent.id
  try {
    await invoke('fork_agent', { id: agent.id, from: id, project: agent.project, options: agent.options })
  } catch (e) {
    fail(agent, String(e))
  }
  persist(agent)
}

export async function archive(id: string, extra: Record<string, unknown> = {}) {
  clearTimeout(timers.get(id))
  const agent = find(id)
  await invoke('archive_agent', { id })
  await invoke('cancel_permissions', { agent: id })
  if (agent) await save(agent, { archived: Date.now(), remote: null, ...extra })
  await invoke('archive_agent_file', { id }).catch(() => invoke('delete_agent_file', { id }))
  agents.list = agents.list.filter((a) => a.id !== id)
}

export async function archiveProject(project: string) {
  await Promise.all(ofProject(project).map((a) => archive(a.id)))
}

export async function shelve(project: Project) {
  const count = ofProject(project.path).length
  const sure = await ask(
    `Archive ${project.name}? ${count ? `Its ${count} agent${count > 1 ? 's' : ''} stop and stay with it. ` : ''}Unarchive it from Settings › Projects to get everything back.`,
    { title: 'Archive project', kind: 'info', okLabel: 'Archive', cancelLabel: 'Cancel' },
  )
  if (!sure) return
  await Promise.all(ofProject(project.path).map((a) => archive(a.id, { withProject: true })))
  await editProject(project.path, { archived: true })
}

export async function unshelve(path: string) {
  await editProject(path, { archived: false })
  for (const a of await archived()) if (a.withProject && a.project === path) await unarchive(a.id)
  projects.overview = false
  projects.board = false
  projects.current = path
}
