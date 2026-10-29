import { t } from './i18n/index.svelte'

export type Options = {
  model: string
  effort: 'low' | 'medium' | 'high' | 'xhigh' | 'max'
  mode: 'default' | 'acceptEdits' | 'plan' | 'auto'
  agent?: string
}

export type Status = 'working' | 'waiting' | 'done' | 'interrupted' | 'error'

export type ToolCall = {
  id: string
  name: string
  input: Record<string, unknown>
  result?: { text: string; error: boolean; data?: Record<string, unknown> }
  shots?: string[]
}

export type Block =
  | { type: 'thinking'; ms?: number; text?: string; at?: number }
  | { type: 'text'; text: string; key: string; at?: number }
  | { type: 'tools'; calls: ToolCall[]; at?: number }

export type Ask = {
  id: string
  agent: string
  tool_name: string
  input: Record<string, any>
  tool_use_id: string | null
}

export type Task = {
  id: string
  toolUseId: string
  subject: string
  activeForm?: string
  status: 'pending' | 'in_progress' | 'completed'
}

export type Subagent = {
  id: string
  task: string
  num: number
  name: string
  type: string
  prompt: string
  status: 'running' | 'done' | 'error' | 'stopped'
  last: string
  tokens: number
  tools: number
  ms: number
  started: number
  ended?: number
  result?: string
  model?: string
  background: boolean
  log: Block[]
}

export const INTERNAL = ['TaskCreate', 'TaskUpdate', 'TaskList', 'TaskGet', 'ToolSearch']

export const READ_ONLY = ['overview', 'read_agent', 'get_layout', 'set_worktree', 'add_review_comment', 'set_progress', 'set_pinned', 'set_info_panel', 'publish_artifact', 'ask_values', 'get_review_prompt', 'read_pull_comments'].map((tool) => `mcp__squadron-pilot__${tool}`)

export type Comment = { path: string; line: number; removed: boolean; code: string; note: string; pull?: number }

export function prompt(text: string, comments: Comment[] = []) {
  const blocks = comments.map((c) =>
    c.path ? `${c.path}:${c.line}${c.removed ? ' (removed line)' : ''}\n> ${c.code.trim()}\n${c.note}` : `On the whole pull request:\n${c.note}`,
  )
  return [...blocks, text].filter(Boolean).join('\n\n')
}

export type Usage = { cost: number; input: number; output: number; ms: number }

export const GOAL_RELAUNCHES = 5

export type Goal = {
  condition: string
  state: 'planned' | 'sent' | 'active' | 'met' | 'stopping' | 'partial' | 'cleared'
  relaunches: number
  reason: string
}

export type Entry =
  | { kind: 'user'; text: string; at: number; comments?: Comment[]; files?: string[]; remote?: boolean; from?: string; peer?: string; squadron?: true }
  | { kind: 'agent'; blocks: Block[]; uuid?: string }
  | ({ kind: 'turn'; index: number } & Usage)
  | ({ kind: 'interrupted'; at: number } & Usage)
  | { kind: 'reloaded'; at: number }
  | { kind: 'resumed'; at: number }
  | { kind: 'forked'; at: number; from: string }
  | { kind: 'launched'; at: number; by: string }
  | { kind: 'compacted'; at: number; before: number; after: number }
  | { kind: 'cleared'; at: number }
  | { kind: 'note'; at: number; command: string; text: string }
  | { kind: 'shell'; at: number; command: string; output: string; state: 'running' | 'done' | 'stopped'; code?: number | null; ms?: number; sent?: boolean; pty?: number }
  | { kind: 'error'; at: number; message: string; login: boolean; missing: boolean }
  | {
      kind: 'decision'
      at: number
      tone: 'ok' | 'bad'
      title: string
      detail: string
      note?: string
      questions?: { question: string; options: string[]; answer: string }[]
    }

export type Agent = {
  id: string
  project: string
  name: string
  options: Options
  status: Status
  entries: Entry[]
  cost: number
  tokens: number
  turns: number
  action: string
  since: number
  live: { input: number; output: number; outputDone: number; started: number }
  alive: boolean
  processCost: number
  interrupting: boolean
  held: boolean
  jobs: string[]
  asks: Ask[]
  allowEdits: boolean
  sessionId: string | null
  forkOf: string | null
  forkAt: string | null
  tasks: Task[]
  summary: string
  attention: string
  autoName: boolean
  commands: string[]
  skills: string[]
  tools?: string[]
  remote: { state: string; url: string; since?: number; last?: number } | null
  subagents: Subagent[]
  goal: Goal | null
  worktree: { path: string; branch: string } | null
  progress?: { done: number; total: number; message: string; place: 'above' | 'below' } | null
  pinned?: boolean
  unread?: boolean
  errorSeen?: boolean
  context?: { max: number; threshold: number | null }
  review?: { project: string; number: number; base?: string; title?: string }
  imported?: string
  unsent?: unknown[]
  resumes?: number
  infoPanel?: { html: string; height: number; hidden?: boolean }
  artifacts?: LegacyArtifact[]
  hiddenDocs?: string[]
  guess?: string
}

export type LegacyArtifact = { id: string; title: string; kind: 'html' | 'svg' | 'markdown'; content: string; version: number; at: number; link?: string }

type Json = Record<string, any>

export const workdir = (agent: Agent) => agent.worktree?.path ?? agent.project

export function newAgent(id: string, project: string, name: string, options: Options): Agent {
  return {
    id,
    project,
    name,
    options,
    status: 'working',
    entries: [],
    cost: 0,
    tokens: 0,
    turns: 0,
    action: t('conversation.starting'),
    since: Date.now(),
    live: { input: 0, output: 0, outputDone: 0, started: Date.now() },
    alive: false,
    processCost: 0,
    interrupting: false,
    held: false,
    jobs: [],
    asks: [],
    allowEdits: false,
    sessionId: null,
    forkOf: null,
    forkAt: null,
    tasks: [],
    summary: '',
    attention: '',
    autoName: false,
    commands: [],
    skills: [],
    remote: null,
    subagents: [],
    goal: null,
    worktree: null,
  }
}

export const known = { commands: [] as string[], skills: [] as string[], terminal: [] as string[] }

export function relative(agent: Agent, path: unknown) {
  const text = String(path ?? '')
  return text.startsWith(agent.project + '/') ? text.slice(agent.project.length + 1) : text
}

export function argument(agent: Agent, call: Pick<ToolCall, 'name' | 'input'>): string {
  const input = call.input
  switch (call.name) {
    case 'Bash':
      return String(input.command ?? '')
    case 'Read':
    case 'Edit':
    case 'MultiEdit':
    case 'Write':
      return relative(agent, input.file_path)
    case 'NotebookEdit':
      return relative(agent, input.notebook_path)
    case 'Grep':
    case 'Glob':
      return String(input.pattern ?? '')
    case 'WebFetch':
      return String(input.url ?? '')
    case 'WebSearch':
      return String(input.query ?? '')
    default: {
      const first = Object.values(input).find((v) => typeof v === 'string')
      return typeof first === 'string' ? first : ''
    }
  }
}

const file = (input: Record<string, unknown>) => String(input.file_path ?? input.notebook_path ?? '').split('/').at(-1)

export function doing(call: Pick<ToolCall, 'name' | 'input'>) {
  const input = call.input
  if (call.name === 'Bash') {
    const described = String(input.description ?? '').split('\n')[0].trim()
    return described && described.length <= 90 && !/&&|\|\||[|;`]|\$\(/.test(described) ? described : t('conversation.runningCommand')
  }
  const name = file(input)
  if (call.name === 'Read') return name ? t('conversation.readingFile', { file: name }) : t('conversation.readingAFile')
  if (['Edit', 'MultiEdit', 'Write', 'NotebookEdit'].includes(call.name)) return name ? t('conversation.editingFile', { file: name }) : t('conversation.editingAFile')
  if (['Grep', 'Glob', 'LS'].includes(call.name)) return t('conversation.searchingCode')
  if (['WebFetch', 'WebSearch'].includes(call.name)) return t('conversation.lookingUpWeb')
  if (call.name === 'Agent' || call.name === 'Task') return t('conversation.startingSubagent')
  return t('conversation.usingTool', { tool: call.name.split('__').at(-1) ?? '' })
}

export function activity(agent: Agent) {
  const last = agent.entries.at(-1)
  const block = last?.kind === 'agent' ? last.blocks.at(-1) : undefined
  const call = block?.type === 'tools' ? block.calls.at(-1) : undefined
  if (call && !call.result && !INTERNAL.includes(call.name)) return doing(call)
  if (block?.type === 'thinking' && block.ms === undefined) return t('conversation.thinking')
  if (block?.type === 'text') return t('conversation.writing')
  return agent.action
}

export function userMessage(agent: Agent, text: string, comments: Comment[] = [], files: string[] = [], from = '') {
  agent.entries.push({
    kind: 'user',
    text,
    at: Date.now(),
    ...(comments.length ? { comments } : {}),
    ...(files.length ? { files } : {}),
    ...(from ? { from } : {}),
  })
  starting(agent)
}

export function starting(agent: Agent) {
  agent.attention = ''
  if (!agent.alive) agent.processCost = 0
  agent.held = false
  agent.status = 'working'
  agent.action = t('conversation.starting')
  agent.since = Date.now()
  agent.live = { input: 0, output: 0, outputDone: 0, started: Date.now() }
}

function agentEntry(agent: Agent) {
  const last = agent.entries.at(-1)
  if (last?.kind === 'agent') return last
  const entry: Entry = { kind: 'agent', blocks: [] }
  agent.entries.push(entry)
  return entry
}

const callsIn = (blocks: Block[]) => blocks.flatMap((b) => (b.type === 'tools' ? b.calls : []))

export function calls(agent: Agent) {
  return [...agent.entries.flatMap((e) => (e.kind === 'agent' ? callsIn(e.blocks) : [])), ...agent.subagents.flatMap((s) => callsIn(s.log))]
}

export function subagentOf(agent: Agent, toolUseId: string | null) {
  return toolUseId ? agent.subagents.find((s) => callsIn(s.log).some((c) => c.id === toolUseId)) : undefined
}

const SETTLED: Record<string, Subagent['status']> = { completed: 'done', failed: 'error', killed: 'stopped', stopped: 'stopped' }

function subagentEvent(agent: Agent, d: Json) {
  if (d.subtype === 'task_started' && d.task_type === 'local_agent' && !agent.subagents.some((s) => s.id === d.tool_use_id))
    agent.subagents.push({
      id: d.tool_use_id,
      task: d.task_id,
      num: agent.subagents.length + 1,
      name: String(d.description ?? ''),
      type: String(d.subagent_type ?? ''),
      prompt: String(d.prompt ?? ''),
      status: 'running',
      last: t('conversation.starting'),
      tokens: 0,
      tools: 0,
      ms: 0,
      started: Date.now(),
      background: !!d.is_backgrounded,
      log: [],
    })
  const sub = agent.subagents.findLast((s) => s.task === d.task_id)
  if (!sub) return
  const u = d.usage
  if (u) {
    sub.tokens = u.total_tokens ?? sub.tokens
    sub.tools = u.tool_uses ?? sub.tools
    sub.ms = u.duration_ms ?? sub.ms
  }
  if (d.subtype === 'task_progress' && d.description) sub.last = String(d.description)
  const status = SETTLED[d.subtype === 'task_updated' ? d.patch?.status : d.subtype === 'task_notification' ? d.status : '']
  if (status && sub.status === 'running') {
    sub.status = status
    sub.ended ??= d.patch?.end_time ?? Date.now()
    if (agent.held) settled(agent, status)
  }
  if (d.subtype === 'task_notification' && d.summary) sub.result = String(d.summary)
}

const SCHEDULES = ['CronCreate', 'ScheduleWakeup']

function job(agent: Agent, d: Json) {
  const id = String(d.task_id ?? '')
  if (d.subtype === 'task_started' && id && !agent.jobs.includes(id)) agent.jobs.push(id)
  const ended = d.subtype === 'task_notification' || (d.subtype === 'task_updated' && SETTLED[d.patch?.status])
  if (ended) agent.jobs = agent.jobs.filter((j) => j !== id)
}

export const running = (agent: Agent) => agent.subagents.filter((s) => s.status === 'running').length

export const waitingOn = (n: number) => t('conversation.waitingOnSubagents', { count: n })

function settled(agent: Agent, status: Subagent['status']) {
  const left = running(agent)
  if (left) return act(agent, waitingOn(left))
  agent.held = false
  agent.since = Date.now()
  if (agent.interrupting) interrupted(agent)
  else if (status === 'stopped') {
    agent.status = 'done'
    agent.action = ''
  } else act(agent, t('conversation.readingSubagents'))
}

function interrupted(agent: Agent, used: Usage = { cost: 0, input: 0, output: 0, ms: 0 }) {
  agent.interrupting = false
  agent.status = 'interrupted'
  agent.action = t('conversation.interruptedWaiting')
  agent.entries.push({ kind: 'interrupted', at: Date.now(), ...used })
}

function textOf(content: unknown): string {
  if (typeof content === 'string') return content
  if (Array.isArray(content))
    return content
      .map((c: Json) => (c.type === 'text' ? c.text : ''))
      .filter(Boolean)
      .join('\n')
  return ''
}

function usage(d: Json, cost: number): Usage {
  const u = d.usage ?? {}
  return {
    cost,
    input: (u.input_tokens ?? 0) + (u.cache_creation_input_tokens ?? 0) + (u.cache_read_input_tokens ?? 0),
    output: u.output_tokens ?? 0,
    ms: d.duration_ms ?? 0,
  }
}

export const flaggedWorktree = new Set<string>()

export function apply(agent: Agent, d: Json) {
  agent.alive = true
  switch (d.type) {
    case 'system':
      if (d.subtype === 'init' && d.session_id) {
        if (d.session_id !== agent.sessionId) agent.forkOf = agent.forkAt = null
        agent.sessionId = d.session_id
      }
      if (d.subtype === 'init' && flaggedWorktree.delete(agent.id) && typeof d.cwd === 'string' && d.cwd.startsWith(agent.project + '/.claude/worktrees/'))
        agent.worktree = { path: d.cwd, branch: '' }
      if (d.subtype === 'init' && Array.isArray(d.slash_commands)) {
        agent.commands = d.slash_commands.filter((c: string) => !d.terminal_slash_commands?.includes(c))
        agent.skills = d.skills ?? []
        if (Array.isArray(d.tools)) agent.tools = d.tools
        known.commands = agent.commands
        known.skills = agent.skills
        known.terminal = d.terminal_slash_commands ?? []
      }
      if (d.subtype === 'bridge_state') agent.remote = { ...agent.remote, state: String(d.state), url: agent.remote?.url ?? '', since: agent.remote?.since ?? Date.now() }
      if (d.subtype === 'status' && d.status === 'compacting') agent.action = t('conversation.compacting')
      if (d.subtype === 'compact_boundary') {
        const meta = d.compact_metadata ?? {}
        agent.entries.push({ kind: 'compacted', at: Date.now(), before: meta.pre_tokens ?? 0, after: meta.post_tokens ?? 0 })
        if (meta.post_tokens) agent.tokens = meta.post_tokens
      }
      if (String(d.subtype).startsWith('task_')) {
        subagentEvent(agent, d)
        job(agent, d)
      }
      if (d.subtype === 'notification' && d.key === 'stop-hook-block-cap' && agent.goal?.state === 'active') agent.goal.state = 'stopping'
      if (d.subtype === 'post_turn_summary' && d.status_detail) agent.summary = d.status_detail
      if (d.subtype === 'post_turn_summary' && d.status_category === 'blocked')
        agent.attention = String(d.needs_action || d.status_detail || t('conversation.waitingForYou'))
      if (d.subtype === 'status' && d.status === 'requesting') {
        agent.guess = ''
        agent.held = false
        if (agent.status !== 'waiting') agent.status = 'working'
      }
      return
    case 'control_response':
      if (typeof d.response?.response?.maxTokens === 'number') {
        const r = d.response.response
        agent.context = { max: r.maxTokens, threshold: r.isAutoCompactEnabled && r.autoCompactThreshold ? r.autoCompactThreshold : null }
        agent.tokens = r.totalTokens ?? agent.tokens
      }
      if (d.response?.response?.session_url)
        agent.remote = { ...agent.remote, state: agent.remote?.state ?? 'ready', url: d.response.response.session_url, since: agent.remote?.since ?? Date.now() }
      return
    case 'prompt_suggestion':
      agent.guess = String(d.suggestion ?? '')
      return
    case 'conversation_reset':
      agent.entries.push({ kind: 'cleared', at: Date.now() })
      agent.tasks = []
      agent.subagents = []
      agent.tokens = 0
      return
    case 'stream_event':
      return d.parent_tool_use_id ? undefined : stream(agent, d)
    case 'assistant':
      return assistant(agent, d)
    case 'user':
      if (d.isSynthetic && textOf(d.message?.content).startsWith('Stop hook feedback')) return relaunched(agent, textOf(d.message.content))
      return d.isReplay && !d.parent_tool_use_id ? remoteMessage(agent, d) : toolResults(agent, d)
    case 'result':
      return result(agent, d)
  }
}

function act(agent: Agent, action: string) {
  if (agent.status !== 'waiting') agent.action = action
}

function stream(agent: Agent, d: Json) {
  const e = d.event
  const key = `${d.api_message_id}:${e.index}`
  if (e.type === 'message_start') {
    const u = e.message.usage ?? {}
    agent.live.outputDone += agent.live.output
    agent.live.output = u.output_tokens ?? 0
    agent.live.input = (u.input_tokens ?? 0) + (u.cache_creation_input_tokens ?? 0) + (u.cache_read_input_tokens ?? 0)
    agent.tokens = agent.live.input + agent.live.output
  } else if (e.type === 'message_delta') {
    agent.live.output = e.usage?.output_tokens ?? agent.live.output
    agent.tokens = agent.live.input + agent.live.output
  } else if (e.type === 'content_block_start') {
    const block = e.content_block
    const entry = agentEntry(agent)
    if (block.type === 'text') {
      entry.blocks.push({ type: 'text', text: '', key })
      act(agent, t('conversation.writingReply'))
    } else if (block.type === 'thinking') {
      entry.blocks.push({ type: 'thinking' })
      act(agent, t('conversation.thinking'))
    } else if (block.type === 'tool_use') {
      const call: ToolCall = { id: block.id, name: block.name, input: {} }
      const last = entry.blocks.at(-1)
      if (last?.type === 'tools') last.calls.push(call)
      else entry.blocks.push({ type: 'tools', calls: [call] })
      act(agent, block.name)
    }
  } else if (e.type === 'content_block_delta' && e.delta.type === 'thinking_delta') {
    const block = agentEntry(agent).blocks.findLast((b) => b.type === 'thinking' && b.ms === undefined)
    if (block?.type === 'thinking') block.text = (block.text ?? '') + e.delta.thinking
  } else if (e.type === 'content_block_delta' && e.delta.type === 'text_delta') {
    const entry = agentEntry(agent)
    const block = entry.blocks.findLast((b) => b.type === 'text' && b.key === key)
    if (block?.type === 'text') block.text += e.delta.text
  }
}

function assistant(agent: Agent, d: Json) {
  const sub = d.parent_tool_use_id && agent.subagents.find((s) => s.id === d.parent_tool_use_id)
  if (d.parent_tool_use_id && !sub) return
  if (sub) sub.model = d.message.model ?? sub.model
  const blocks = (): Block[] => (sub ? sub.log : agentEntry(agent).blocks)
  const at = sub ? { at: d.timestamp ? Date.parse(d.timestamp) : Date.now() } : {}
  if (!sub && d.message.model === '<synthetic>') goalNotice(agent, textOf(d.message.content))
  if (!sub && d.uuid && d.message.model !== '<synthetic>') agentEntry(agent).uuid = d.uuid
  for (const c of d.message.content as Json[]) {
    if (c.type === 'thinking') {
      const block = blocks().findLast((b) => b.type === 'thinking' && b.ms === undefined)
      if (block?.type === 'thinking') block.ms = d.thinking_duration_ms ?? 0
      else blocks().push({ type: 'thinking', ms: d.thinking_duration_ms ?? 0, ...(c.thinking ? { text: c.thinking } : {}), ...at })
      const done = blocks().findLast((b) => b.type === 'thinking')
      if (done?.type === 'thinking' && c.thinking) done.text = c.thinking
    } else if (c.type === 'tool_use') {
      const call = calls(agent).find((x) => x.id === c.id)
      if (call) call.input = c.input
      else {
        const last = blocks().at(-1)
        const added: ToolCall = { id: c.id, name: c.name, input: c.input }
        if (last?.type === 'tools' && !sub) last.calls.push(added)
        else blocks().push({ type: 'tools', calls: [added], ...at })
      }
      const action = doing({ name: c.name, input: c.input })
      if (sub) sub.last = action
      else {
        if (SCHEDULES.includes(c.name) && !c.input?.stop && !agent.jobs.includes(c.id)) agent.jobs.push(c.id)
        if (INTERNAL.includes(c.name)) track(agent, c)
        else act(agent, action)
      }
    } else if (c.type === 'text' && (sub || !blocks().some((b) => b.type === 'text' && b.key.startsWith(`${d.message.id}:`)))) {
      blocks().push({ type: 'text', text: c.text, key: `${d.message.id}:${sub ? d.uuid : 'full'}`, ...at })
    }
  }
}

export function replayLog(agent: Agent, sub: Subagent, transcript: string) {
  sub.log = []
  for (const line of transcript.split('\n')) {
    let d: Json
    try {
      d = JSON.parse(line)
    } catch {
      continue
    }
    const routed = { ...d, parent_tool_use_id: sub.id, tool_use_result: d.toolUseResult }
    if (d.type === 'assistant') assistant(agent, routed)
    else if (d.type === 'user') toolResults(agent, routed)
  }
}

function track(agent: Agent, c: Json) {
  const input = c.input ?? {}
  if (c.name === 'TaskCreate') {
    agent.tasks.push({ id: '', toolUseId: c.id, subject: String(input.subject ?? ''), activeForm: input.activeForm, status: 'pending' })
  } else if (c.name === 'TaskUpdate') {
    const task = agent.tasks.find((t) => t.id === String(input.taskId))
    if (!task) return
    if (input.status === 'deleted') agent.tasks = agent.tasks.filter((t) => t !== task)
    else {
      if (input.status) task.status = input.status
      if (input.subject) task.subject = input.subject
      if (input.activeForm) task.activeForm = input.activeForm
      if (task.status === 'in_progress') act(agent, task.activeForm ?? task.subject)
    }
  }
}

function goalNotice(agent: Agent, text: string) {
  const set = /^Goal set: ([\s\S]+)/.exec(text)
  if (set) {
    agent.goal = { condition: set[1].trim(), state: 'active', relaunches: 0, reason: '' }
    agent.status = 'working'
  }
  else if (text.startsWith('Goal cleared') && agent.goal && agent.goal.state !== 'partial') agent.goal.state = 'cleared'
}

function relaunched(agent: Agent, text: string) {
  const goal = agent.goal
  if (!goal) return
  const reason = text.replace('Stop hook feedback:', '').trim()
  const head = `[${goal.condition}]:`
  goal.relaunches += 1
  goal.reason = (reason.startsWith(head) ? reason.slice(head.length) : reason).trim()
  agent.entries.push({
    kind: 'decision',
    at: Date.now(),
    tone: 'bad',
    title: t('conversation.goalNotMet'),
    detail: t('conversation.relaunchOf', { n: goal.relaunches, total: GOAL_RELAUNCHES, reason: goal.reason }),
  })
}

export const notice = (text: string) => /^<task-notification>\s*([\s\S]*?)\s*<\/task-notification>$/.exec(text.trim())?.[1]

export function peerMessage(text: string) {
  const m = /<cross-session-message\b([^>]*)>([\s\S]*?)<\/cross-session-message>/.exec(text)
  if (!m) return
  const attr = (key: string) => new RegExp(`(?:^|\\s)${key}="([^"]*)"`).exec(m[1])?.[1]
  return { from: attr('from-name') || attr('name') || attr('from') || '', text: m[2].trim() }
}

export const commandEcho = (text: string) => /^<(local-command-|command-name>|command-message>)/.test(text.trim())

export function commandOutput(text: string) {
  const out = /^<local-command-(?:stdout|stderr)>([\s\S]*?)<\/local-command-(?:stdout|stderr)>$/.exec(text.trim())?.[1].trim()
  return out && !out.startsWith('Compacted') ? out : ''
}

function remoteMessage(agent: Agent, d: Json) {
  if (d.origin?.handback) {
    const sub = agent.subagents.findLast((s) => s.task === d.origin.senderTaskId)
    if (sub && d.origin.handbackReportText) sub.result = String(d.origin.handbackReportText)
    return
  }
  const text = textOf(d.message?.content).trim()
  if (!text) return
  const inside = commandEcho(text) ? commandOutput(text) : notice(text)
  if (inside) agent.entries.push({ kind: 'note', at: d.timestamp ? Date.parse(d.timestamp) : Date.now(), command: 'notice', text: inside })
  if (inside !== undefined) return
  const peer = peerMessage(text)
  userMessage(agent, peer?.text ?? text)
  const entry = agent.entries.at(-1)
  if (entry?.kind === 'user') {
    if (peer) entry.peer = peer.from
    else entry.remote = true
    entry.at = d.timestamp ? Date.parse(d.timestamp) : entry.at
  }
  if (agent.remote) agent.remote.last = Date.now()
}

export const captures: { save?: (agent: Agent, call: ToolCall, data: string) => void } = {}

const imagesOf = (content: unknown) =>
  Array.isArray(content) ? (content as Json[]).filter((x) => x.type === 'image' && x.source?.type === 'base64').map((x) => String(x.source.data)) : []

function toolResults(agent: Agent, d: Json) {
  const content = d.message?.content
  if (!Array.isArray(content)) return
  for (const c of content as Json[]) {
    if (c.type !== 'tool_result') continue
    const created = agent.tasks.find((t) => t.toolUseId === c.tool_use_id && !t.id)
    if (created && d.tool_use_result?.task?.id) created.id = String(d.tool_use_result.task.id)
    const call = calls(agent).find((x) => x.id === c.tool_use_id)
    const result = d.tool_use_result
    if (!d.parent_tool_use_id && call?.name === 'EnterWorktree' && result?.worktreePath)
      agent.worktree = { path: String(result.worktreePath), branch: String(result.worktreeBranch ?? '') }
    if (!d.parent_tool_use_id && call?.name === 'ExitWorktree' && !c.is_error) agent.worktree = null
    if (call && !call.result) for (const data of imagesOf(c.content)) captures.save?.(agent, call, data)
    if (call)
      call.result = {
        text: textOf(c.content),
        error: !!c.is_error,
        data: typeof d.tool_use_result === 'object' ? d.tool_use_result : undefined,
      }
  }
}

function result(agent: Agent, d: Json) {
  agent.asks = []
  const total = d.total_cost_usd ?? 0
  const cost = Math.max(0, total - agent.processCost)
  agent.processCost = Math.max(agent.processCost, total)
  const used = usage(d, cost)
  agent.cost += cost
  agent.since = Date.now()
  if (agent.interrupting) interrupted(agent, used)
  else if (d.is_error) {
    const message = String(d.result ?? ((d.errors ?? []).join('\n') || d.subtype))
    fail(agent, message || t('conversation.sessionStopped'))
  } else {
    const left = running(agent)
    const goal = agent.goal?.state === 'sent'
    agent.resumes = undefined
    agent.held = !goal && left > 0
    agent.status = goal || left ? 'working' : 'done'
    agent.action = goal ? t('conversation.startingGoal') : left ? waitingOn(left) : ''
    if (agent.goal?.state === 'active') {
      agent.goal.state = 'met'
      agent.entries.push({ kind: 'decision', at: Date.now(), tone: 'ok', title: t('conversation.goalMet'), detail: agent.goal.condition })
    }
    if (d.num_turns === 0 && !cost) return
    agent.turns += 1
    agent.entries.push({ kind: 'turn', index: agent.turns, ...used })
  }
}

export function fail(agent: Agent, message: string) {
  agent.asks = []
  agent.status = 'error'
  agent.errorSeen = false
  agent.interrupting = false
  agent.held = false
  agent.attention = message.split('\n')[0] || t('conversation.error')
  agent.since = Date.now()
  const missing = message === 'claude-not-found'
  const login = /not logged in|\/login|invalid api key|failed to authenticate|oauth session expired/i.test(message)
  agent.action = missing ? t('conversation.claudeNotFound') : login ? t('conversation.claudeNotLoggedIn') : message.split('\n')[0]
  agent.entries.push({ kind: 'error', at: Date.now(), message, login, missing })
}

export function exited(agent: Agent, stderr: string) {
  agent.alive = false
  agent.remote = null
  agent.jobs = []
  agent.asks = []
  agent.held = false
  for (const s of agent.subagents) if (s.status === 'running') s.status = 'stopped'
  if (agent.interrupting) interrupted(agent)
  else if (agent.status === 'working' || agent.status === 'waiting') {
    fail(agent, stderr.trim() || t('conversation.sessionStoppedReplying'))
  }
}

export function reloaded(agent: Agent) {
  agent.asks = []
  agent.attention = ''
  agent.entries.push({ kind: 'reloaded', at: Date.now() })
  agent.processCost = 0
  agent.jobs = []
  agent.alive = true
  agent.interrupting = false
  agent.held = false
  for (const s of agent.subagents) if (s.status === 'running') s.status = 'stopped'
  if (agent.status !== 'done') {
    agent.status = 'done'
    agent.action = t('conversation.sessionReloaded')
  }
}
