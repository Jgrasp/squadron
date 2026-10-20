export type Options = {
  model: 'opus' | 'sonnet' | 'haiku'
  effort: 'low' | 'medium' | 'high' | 'xhigh' | 'max'
  mode: 'default' | 'acceptEdits' | 'plan' | 'auto'
}

export type Status = 'working' | 'waiting' | 'done' | 'interrupted' | 'error'

export type ToolCall = {
  id: string
  name: string
  input: Record<string, unknown>
  result?: { text: string; error: boolean; data?: Record<string, unknown> }
}

export type Block =
  | { type: 'thinking'; ms?: number }
  | { type: 'text'; text: string; key: string }
  | { type: 'tools'; calls: ToolCall[] }

export type Ask = {
  id: string
  agent: string
  tool_name: string
  input: Record<string, any>
  tool_use_id: string | null
}

export type Usage = { cost: number; input: number; output: number; ms: number }

export type Entry =
  | { kind: 'user'; text: string; at: number }
  | { kind: 'agent'; blocks: Block[] }
  | ({ kind: 'turn'; index: number } & Usage)
  | ({ kind: 'interrupted'; at: number } & Usage)
  | { kind: 'reloaded'; at: number }
  | { kind: 'error'; at: number; message: string; login: boolean; missing: boolean }
  | { kind: 'decision'; at: number; tone: 'ok' | 'bad'; title: string; detail: string; note?: string }

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
  asks: Ask[]
  allowEdits: boolean
}

type Json = Record<string, any>

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
    action: 'Démarre…',
    since: Date.now(),
    live: { input: 0, output: 0, outputDone: 0, started: Date.now() },
    alive: false,
    processCost: 0,
    interrupting: false,
    asks: [],
    allowEdits: false,
  }
}

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

export function userMessage(agent: Agent, text: string) {
  agent.entries.push({ kind: 'user', text, at: Date.now() })
  if (!agent.alive) agent.processCost = 0
  agent.status = 'working'
  agent.action = 'Démarre…'
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

export function calls(agent: Agent) {
  return agent.entries.flatMap((e) =>
    e.kind === 'agent' ? e.blocks.flatMap((b) => (b.type === 'tools' ? b.calls : [])) : [],
  )
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

export function apply(agent: Agent, d: Json) {
  agent.alive = true
  switch (d.type) {
    case 'system':
      if (d.subtype === 'status' && d.status === 'requesting' && agent.status !== 'waiting')
        agent.status = 'working'
      return
    case 'stream_event':
      return stream(agent, d)
    case 'assistant':
      return assistant(agent, d)
    case 'user':
      return toolResults(agent, d)
    case 'result':
      return result(agent, d)
  }
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
      agent.action = 'Rédige la réponse…'
    } else if (block.type === 'thinking') {
      entry.blocks.push({ type: 'thinking' })
      agent.action = 'Réfléchit…'
    } else if (block.type === 'tool_use') {
      const call: ToolCall = { id: block.id, name: block.name, input: {} }
      const last = entry.blocks.at(-1)
      if (last?.type === 'tools') last.calls.push(call)
      else entry.blocks.push({ type: 'tools', calls: [call] })
      agent.action = block.name
    }
  } else if (e.type === 'content_block_delta' && e.delta.type === 'text_delta') {
    const entry = agentEntry(agent)
    const block = entry.blocks.findLast((b) => b.type === 'text' && b.key === key)
    if (block?.type === 'text') block.text += e.delta.text
  }
}

function assistant(agent: Agent, d: Json) {
  for (const c of d.message.content as Json[]) {
    if (c.type === 'thinking') {
      const entry = agentEntry(agent)
      const block = entry.blocks.findLast((b) => b.type === 'thinking' && b.ms === undefined)
      if (block?.type === 'thinking') block.ms = d.thinking_duration_ms ?? 0
    } else if (c.type === 'tool_use') {
      const call = calls(agent).find((x) => x.id === c.id)
      if (call) call.input = c.input
      agent.action = `${c.name} ${argument(agent, { name: c.name, input: c.input })}`.trim()
    } else if (c.type === 'text' && !agentEntry(agent).blocks.some((b) => b.type === 'text' && b.key.startsWith(`${d.message.id}:`))) {
      agentEntry(agent).blocks.push({ type: 'text', text: c.text, key: `${d.message.id}:full` })
    }
  }
}

function toolResults(agent: Agent, d: Json) {
  const content = d.message?.content
  if (!Array.isArray(content)) return
  for (const c of content as Json[]) {
    if (c.type !== 'tool_result') continue
    const call = calls(agent).find((x) => x.id === c.tool_use_id)
    if (call)
      call.result = {
        text: textOf(c.content),
        error: !!c.is_error,
        data: typeof d.tool_use_result === 'object' ? d.tool_use_result : undefined,
      }
  }
}

function result(agent: Agent, d: Json) {
  const total = d.total_cost_usd ?? 0
  const cost = Math.max(0, total - agent.processCost)
  agent.processCost = Math.max(agent.processCost, total)
  const used = usage(d, cost)
  agent.cost += cost
  agent.since = Date.now()
  if (agent.interrupting) {
    agent.interrupting = false
    agent.status = 'interrupted'
    agent.action = 'Interrompu · attend ta consigne'
    agent.entries.push({ kind: 'interrupted', at: Date.now(), ...used })
  } else if (d.is_error) {
    const message = String(d.result ?? ((d.errors ?? []).join('\n') || d.subtype))
    fail(agent, message || 'La session s’est arrêtée.')
  } else {
    agent.turns += 1
    agent.status = 'done'
    agent.action = ''
    agent.entries.push({ kind: 'turn', index: agent.turns, ...used })
  }
}

export function fail(agent: Agent, message: string) {
  agent.asks = []
  agent.status = 'error'
  agent.interrupting = false
  agent.since = Date.now()
  const missing = message === 'claude-introuvable'
  const login = /not logged in|\/login|invalid api key/i.test(message)
  agent.action = missing ? 'Claude Code introuvable' : login ? 'Claude Code non connecté' : message.split('\n')[0]
  agent.entries.push({ kind: 'error', at: Date.now(), message, login, missing })
}

export function exited(agent: Agent, stderr: string) {
  agent.alive = false
  agent.asks = []
  if (agent.interrupting) {
    agent.interrupting = false
    agent.status = 'interrupted'
    agent.action = 'Interrompu · attend ta consigne'
    agent.entries.push({ kind: 'interrupted', at: Date.now(), cost: 0, input: 0, output: 0, ms: 0 })
  } else if (agent.status === 'working' || agent.status === 'waiting') {
    fail(agent, stderr.trim() || 'La session s’est arrêtée pendant la réponse de l’agent.')
  }
}

export function reloaded(agent: Agent) {
  agent.asks = []
  agent.entries.push({ kind: 'reloaded', at: Date.now() })
  agent.processCost = 0
  agent.alive = true
  agent.interrupting = false
  if (agent.status !== 'done') {
    agent.status = 'done'
    agent.action = 'Session rechargée'
  }
}
