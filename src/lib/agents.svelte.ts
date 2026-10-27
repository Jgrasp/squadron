import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { remember } from './catalogs.svelte'
import { EDITS } from './diff'
import { addRule, editProject, projects, type Project } from './projects.svelte'
import { ask } from './asking.svelte'
import { matches, ruleFor, ruleLabel } from './rules'
import { settings } from './settings.svelte'
import { entriesOf } from './transcript'
import { t } from './i18n/index.svelte'
import {
  apply,
  argument,
  captures,
  commandEcho,
  peerMessage,
  commandOutput,
  subagentOf,
  INTERNAL,
  READ_ONLY,
  exited,
  fail,
  flaggedWorktree,
  GOAL_RELAUNCHES,
  newAgent,
  notice,
  prompt,
  reloaded,
  running,
  userMessage,
  waitingOn,
  type Agent,
  type Ask,
  type Comment,
  type Entry,
  type Options,
} from './stream'

export const agents = $state({ list: [] as Agent[], current: {} as Record<string, string> })

export const clock = $state({ now: Date.now() })

export type Limit = { utilization: number; resetsAt: number }

export const limits: { five_hour?: Limit; seven_day?: Limit } = $state(stored('limits', {}))

function stored<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem(key) ?? '') ?? fallback
  } catch {
    return fallback
  }
}

function track(windows: typeof limits) {
  Object.assign(limits, windows)
  try {
    localStorage.setItem('limits', JSON.stringify(limits))
  } catch {}
}

export function branchOf(agent: Agent) {
  const worktree = agent.worktree
  if (!worktree) return
  invoke<string | null>('git_branch', { path: worktree.path })
    .then((branch) => (worktree.branch = branch ?? ''))
    .catch(() => {})
}

export const calling = (a: Agent) => !!a.attention && !(a.status === 'error' && a.errorSeen)

export function waitingAgents() {
  return agents.list.filter(calling).sort((a, b) => a.since - b.since)
}

export function show(agent: Agent) {
  projects.overview = false
  projects.board = false
  projects.pulls = false
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
  return agents.list.filter((a) => a.project === project && calling(a)).length
}

export function unread(project: string) {
  return agents.list.filter((a) => a.project === project && a.unread && !a.attention).length
}

export const onScreen = (agent: Agent) =>
  !projects.overview && !projects.board && !projects.pulls && projects.current === agent.project && selected(agent.project)?.id === agent.id

export function markUnread(id: string, value: boolean) {
  const agent = find(id)
  if (!agent || !!agent.unread === value) return
  agent.unread = value
  persist(agent)
}

export function selected(project: string) {
  const list = ofProject(project)
  return list.find((a) => a.id === agents.current[project]) ?? list[0]
}

captures.save = (agent, call, data) =>
  invoke<string>('save_capture', { data })
    .then((path) => {
      call.shots = [...(call.shots ?? []), path]
      persist(agent)
    })
    .catch(() => {})

listen<{ id: string; line: string }>('agent-line', ({ payload }) => {
  const agent = find(payload.id)
  if (!agent) return
  const d = JSON.parse(payload.line)
  const previous = agent.worktree
  const was = agent.status
  apply(agent, d)
  if (was !== 'done' && agent.status === 'done' && !onScreen(agent)) markUnread(agent.id, true)
  const next = agent.worktree
  if (next?.path !== previous?.path) {
    if (!next) invoke('set_workdir', { id: agent.id, dir: null }).catch(() => {})
    else
      invoke<string | null>('git_branch', { path: next.path })
        .then((branch) => {
          if (!next.branch) next.branch = branch ?? ''
          return invoke('set_workdir', { id: agent.id, dir: next.path })
        })
        .catch(() => {
          if (agent.worktree !== next) return
          agent.worktree = previous
          persist(agent)
        })
  }
  if (d.type === 'result') invoke('cancel_permissions', { agent: agent.id })
  if (agent.goal?.state === 'active' && agent.goal.relaunches >= GOAL_RELAUNCHES) {
    agent.goal.state = 'stopping'
    invoke('stop_turn', { id: agent.id })
  }
  if (d.type === 'result' && agent.goal?.state === 'stopping') {
    agent.goal.state = 'partial'
    agent.entries.push({ kind: 'decision', at: Date.now(), tone: 'bad', title: t('shell.goalStopped'), detail: t('shell.goalStoppedDetail', { count: GOAL_RELAUNCHES, reason: agent.goal.reason }) })
    invoke('send_message', { id: agent.id, text: '/goal clear' })
  }
  if (d.subtype === 'init' && agent.worktree && !agent.worktree.branch) branchOf(agent)
  if (d.type === 'result' || d.subtype === 'init' || d.subtype === 'compact_boundary') invoke('context_usage', { id: agent.id }).catch(() => {})
  if (d.type === 'result' || d.subtype === 'init' || d.response?.response?.maxTokens) persist(agent)
  if (d.type === 'rate_limit_event' && d.rate_limit_info?.unifiedWindows) track(d.rate_limit_info.unifiedWindows)
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
  const { live, alive, processCost, interrupting, held, asks, ...kept } = $state.snapshot(agent)
  const subagents = kept.subagents.map(({ log, ...s }) => ({ ...s, log: [] }))
  return invoke('save_agent', { id: agent.id, data: JSON.stringify({ ...kept, subagents, ...extra }) })
}

export function saveAll() {
  return Promise.all(agents.list.map((a) => save(a)))
}

export function archiveIdle() {
  const days = settings.archive_after_days
  if (!days) return
  const limit = Date.now() - days * 86_400_000
  const shown = projects.overview ? undefined : agents.current[projects.current]
  for (const agent of [...agents.list])
    if (['done', 'interrupted', 'error'].includes(agent.status) && !agent.asks.length && !agent.remote && !agent.pinned && agent.since < limit && agent.id !== shown)
      archive(agent.id)
}

let scanning = false
const elsewhere = new Set<string>()

export async function rescan() {
  if (scanning) return
  scanning = true
  try {
    const known = new Set(projects.list.map((p) => p.path))
    for (const id of await invoke<string[]>('agent_ids')) {
      if (find(id) || elsewhere.has(id)) continue
      const text = await invoke<string | null>('load_agent', { id })
      let data: Agent | null = null
      try {
        data = text ? JSON.parse(text) : null
      } catch {}
      if (data && known.has(data.project) && !find(data.id)) await adopt(data)
      else elsewhere.add(id)
    }
  } finally {
    scanning = false
  }
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
    agent.action = t('shell.stoppedOnClose')
  }
  for (const s of agent.subagents) if (s.status === 'running') s.status = 'stopped'
  if (agent.goal?.state === 'stopping') agent.goal.state = 'active'
  agent.entries = agent.entries.flatMap((e): Entry[] => {
    if (e.kind !== 'user') return [e]
    const peer = peerMessage(e.text)
    if (peer) return [{ ...e, text: peer.text, peer: peer.from, remote: undefined }]
    const text = commandEcho(e.text) ? commandOutput(e.text) : notice(e.text)
    return text === undefined ? [e] : text ? [{ kind: 'note', at: e.at, command: 'notice', text }] : []
  })
  agents.list.push(agent)
  await invoke('restore_agent', {
    id: agent.id,
    project: agent.project,
    options: agent.options,
    sessionId: agent.sessionId,
    fork: !!agent.forkOf,
    at: agent.forkOf ? agent.forkAt : null,
    dir: agent.worktree?.path ?? null,
  }).catch((e) => fail(find(agent.id)!, String(e)))
  if (wanted) remote(agent.id, true)
}

export type Elsewhere = {
  id: string
  title: string
  cwd: string
  here: boolean
  updated: number
  size: number
  app: boolean
  archived: boolean
  model: string | null
  effort: string | null
  mode: string | null
}

export async function knownSessions() {
  const all: Agent[] = [...agents.list, ...(await archived())]
  return new Set(all.flatMap((a) => [a.sessionId, a.imported].filter((id): id is string => !!id)))
}

export async function importSession(project: string, found: Elsewhere, name = '') {
  const transcript = await invoke<string>('read_session', { project, id: found.id })
  const from = found.app ? t('shell.theClaudeApp') : 'Claude Code'
  const pick = <T extends string>(value: string | null, allowed: readonly T[], fallback: T) => (allowed.includes(value as T) ? (value as T) : fallback)
  const options: Options = {
    model: found.model && /^[a-z0-9][a-z0-9.\-[\]]*$/i.test(found.model) ? found.model : settings.defaults.model,
    effort: pick(found.effort, ['low', 'medium', 'high', 'xhigh', 'max'] as const, settings.defaults.effort),
    mode: pick(found.mode, ['default', 'plan', 'acceptEdits', 'auto'] as const, settings.defaults.mode),
  }
  const entries = entriesOf(transcript)
  const data: Agent = {
    ...newAgent(crypto.randomUUID(), project, name || found.title || t('shell.importedConversation'), options),
    entries: [...entries, { kind: 'forked', at: Date.now(), from }],
    status: 'done',
    action: t('shell.importedFrom', { from }),
    since: found.updated || Date.now(),
    turns: entries.filter((e) => e.kind === 'user').length,
    sessionId: found.id,
    forkOf: from,
    forkAt: null,
    imported: found.id,
    worktree: found.here && found.cwd && found.cwd !== project ? { path: found.cwd, branch: '' } : null,
  }
  await adopt(data)
  const agent = find(data.id)!
  if (agent.worktree) branchOf(agent)
  persist(agent)
  return agent
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
    (READ_ONLY.includes(ask.tool_name) && !(ask.tool_name.endsWith('__set_worktree') && ask.input.agent && ask.input.agent !== agent.id)) ||
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
    agent.action = agent.held ? waitingOn(running(agent)) : t('shell.resuming')
    agent.attention = ''
    return
  }
  agent.status = 'waiting'
  agent.action = waitingFor(ask)
  agent.attention = agent.action
}

export function waitingFor(ask: Ask, say = t) {
  if (ask.tool_name === 'AskUserQuestion') return say('shell.waitingAnswerQuestion')
  if (ask.tool_name === 'ExitPlanMode') return say('shell.waitingApprovalPlan')
  return say('shell.waitingApprovalTool', { tool: toolLabel(ask.tool_name).split(' · ')[0] })
}

export function waitingDetail(agent: Agent) {
  const ask = agent.asks[0]
  if (!ask) return ''
  if (ask.tool_name === 'AskUserQuestion') return String(ask.input.questions?.[0]?.question ?? '')
  if (ask.tool_name === 'ExitPlanMode')
    return (String(ask.input.plan ?? '').split('\n').find((l) => l.trim()) ?? '').replace(/^#+\s*/, '')
  const sub = subagentOf(agent, ask.tool_use_id)
  const what = ask.tool_name.startsWith('mcp__') ? toolLabel(ask.tool_name) : argument(agent, { name: ask.tool_name, input: ask.input }) || toolLabel(ask.tool_name)
  return sub ? t('shell.subagentDetail', { num: String(sub.num), what }) : what
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
    note = t('shell.editsWithoutAsking', { name: agent.name })
  } else if (always) {
    const rule = ruleFor(ask.tool_name, ask.input)
    await addRule(agent.project, rule)
    note = t('shell.ruleAdded', { project: projects.list.find((p) => p.path === agent.project)?.name ?? '', rule: ruleLabel(rule) })
  }
  return settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: always ? t('shell.alwaysAllowed') : t('shell.allowedOnce'),
    detail: detailOf(agent, ask),
    note,
  })
}

export function deny(agent: Agent, ask: Ask, message = '') {
  return settle(
    agent,
    ask,
    { behavior: 'deny', message: message || 'The user denied this.' },
    { tone: 'bad', title: t('shell.denied'), detail: detailOf(agent, ask), note: message ? t('shell.withInstruction', { message }) : undefined },
  )
}

export function answer(agent: Agent, ask: Ask, answers: Record<string, string>) {
  return settle(
    agent,
    ask,
    { behavior: 'allow', updatedInput: { ...ask.input, answers } },
    {
      tone: 'ok',
      title: t('shell.answered'),
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
      { tone: 'bad', title: t('shell.planToRevise'), detail: t('shell.keepPlanning'), note: remark ? t('shell.withFeedback', { remark }) : undefined },
    )
  if (choice === 'edits') agent.allowEdits = true
  const approved = settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: t('shell.planApproved'),
    detail: choice === 'edits' ? t('shell.startedAcceptEdits') : t('shell.startedAsking'),
  })
  arm(agent)
  return approved
}

async function arm(agent: Agent) {
  if (agent.goal?.state !== 'planned') return
  agent.goal.state = 'sent'
  persist(agent)
  await invoke('send_message', { id: agent.id, text: `/goal ${agent.goal.condition}` }).catch((e) => fail(agent, String(e)))
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

type Launch = { from?: { by: string }; goal?: string; worktree?: string | null; dir?: string; files?: string[] }

export async function create(project: string, name: string, options: Options, text: string, { from, goal = '', worktree = null, dir, files = [] }: Launch = {}) {
  const fallback = text.trim().split(/\s+/).slice(0, 5).join(' ')
  agents.list.push(newAgent(crypto.randomUUID(), project, name || fallback || t('common.newAgent'), options))
  const agent = agents.list.at(-1)!
  if (goal) agent.goal = { condition: goal, state: 'planned', relaunches: 0, reason: '' }
  if (dir) agent.worktree = { path: dir, branch: '' }
  if (worktree) flaggedWorktree.add(agent.id)
  if (!name) {
    agent.autoName = true
    if (text.trim()) entitle(agent, text)
  }
  if (from) agent.entries.push({ kind: 'launched', at: Date.now(), by: from.by })
  else agents.current[project] = agent.id
  if (text.trim()) userMessage(agent, text, [], files)
  else {
    agent.status = 'done'
    agent.since = Date.now()
  }
  persist(agent)
  try {
    await invoke('create_agent', { id: agent.id, project, options, text, worktree: worktree || null, dir: dir ?? null, files })
    if (dir) branchOf(agent)
    if (options.mode !== 'plan') await arm(agent)
  } catch (e) {
    fail(agent, String(e))
  }
  return agent.id
}

function entitle(agent: Agent, text: string) {
  invoke<string>('suggest_title', { text })
    .then((title) => {
      if (!agent.autoName) return
      agent.name = title
      agent.autoName = false
      persist(agent)
    })
    .catch(() => {})
}

export async function send(id: string, text: string, comments: Comment[] = [], files: string[] = [], from = '') {
  const agent = find(id)
  if (!agent) return
  if (agent.autoName && text.trim() && !agent.entries.some((e) => e.kind === 'user')) entitle(agent, text)
  userMessage(agent, text, comments, files, from)
  persist(agent)
  const said = from ? `Message from the agent “${from}”, sent with the user's approval:\n\n${prompt(text, comments)}` : prompt(text, comments)
  try {
    await invoke('send_message', { id, text: said, files })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function retry(id: string) {
  const agent = find(id)
  const last = agent?.entries.findLast((e) => e.kind === 'user')
  if (!agent || last?.kind !== 'user') return
  agent.status = 'working'
  agent.action = t('shell.starting')
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

export function pin(id: string, pinned: boolean) {
  const agent = find(id)
  if (!agent) return
  agent.pinned = pinned
  persist(agent)
}

export function rename(id: string, name: string) {
  const agent = find(id)
  if (!agent || !name.trim()) return
  agent.name = name.trim()
  agent.autoName = false
  persist(agent)
}

export type Draft = { text: string; files: string[]; comments: Comment[] }

export const drafts: Record<string, Draft> = (() => {
  try {
    return JSON.parse(localStorage.getItem('squadron.drafts') ?? '{}')
  } catch {
    return {}
  }
})()

export function keepDraft(id: string, draft: Draft) {
  if (draft.text || draft.files.length || draft.comments.length) drafts[id] = draft
  else delete drafts[id]
  try {
    localStorage.setItem('squadron.drafts', JSON.stringify(drafts))
  } catch {}
}

export function forkPoint(agent: Agent, upTo: number) {
  const before = agent.entries.slice(0, upTo).findLast((e) => e.kind === 'agent')
  if (!before) return { at: undefined }
  return before.kind === 'agent' && before.uuid && agent.sessionId ? { at: before.uuid } : null
}

export async function fork(id: string, upTo?: number) {
  const source = find(id)
  const point = upTo === undefined ? { at: undefined } : source && forkPoint(source, upTo)
  if (!source || !point || (upTo === undefined && !source.sessionId)) return
  const copy = newAgent(crypto.randomUUID(), source.project, t('shell.forkName', { name: source.name }), $state.snapshot(source.options))
  const snapshot = $state.snapshot(source)
  const resumes = upTo === undefined || !!point.at
  copy.entries = [...(upTo === undefined ? snapshot.entries : snapshot.entries.slice(0, upTo)), { kind: 'forked', at: Date.now(), from: source.name }]
  copy.tasks = upTo === undefined ? snapshot.tasks : []
  copy.subagents = upTo === undefined ? snapshot.subagents : []
  copy.sessionId = resumes ? source.sessionId : null
  copy.forkOf = resumes ? source.name : null
  copy.forkAt = point.at ?? null
  copy.worktree = resumes ? snapshot.worktree : null
  copy.status = 'done'
  copy.action = t('shell.forkedFrom', { name: source.name })
  const message = upTo === undefined ? undefined : snapshot.entries[upTo]
  if (message?.kind === 'user') keepDraft(copy.id, { text: message.text, files: message.files ?? [], comments: message.comments ?? [] })
  agents.list.push(copy)
  const agent = agents.list.at(-1)!
  agents.current[agent.project] = agent.id
  try {
    if (resumes) await invoke('fork_agent', { id: agent.id, from: id, project: agent.project, options: agent.options, at: point.at })
    else await invoke('restore_agent', { id: agent.id, project: agent.project, options: agent.options, sessionId: null, fork: false, at: null, dir: null })
    if (agent.worktree) await invoke('set_workdir', { id: agent.id, dir: agent.worktree.path })
  } catch (e) {
    fail(agent, String(e))
  }
  persist(agent)
}

export async function remove(id: string) {
  clearTimeout(timers.get(id))
  await invoke('archive_agent', { id })
  await invoke('cancel_permissions', { agent: id })
  await invoke('delete_agent_file', { id })
  agents.list = agents.list.filter((a) => a.id !== id)
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
    count ? t('shell.archiveProjectAskAgents', { name: project.name, count }) : t('shell.archiveProjectAsk', { name: project.name }),
    { title: t('shell.archiveProject'), kind: 'info', okLabel: t('common.archive'), cancelLabel: t('common.cancel') },
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
