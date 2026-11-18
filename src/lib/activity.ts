import { INTERNAL, type Agent } from './stream'
import type { Group, Project } from './projects.svelte'

export type ApiState = 'ok' | 'overloaded' | 'outage' | 'offline' | 'signed_out'

export type ClaudeApi = { state: ApiState; retries: number; since: number }

export type AgentActivity = {
  id: string
  project: string
  name: string
  status: Agent['status']
  waiting: 'permission' | 'question' | null
  tool: string | null
  error: 'crash' | 'api' | 'mcp' | null
  by: string | null
  archived: boolean
  since: number
  unread: boolean
}

export type ProjectActivity = { id: string; name: string; color: string | null; group: string | null; archived: boolean; active: number; week_agents: number; agents: number }

type Limit = { utilization: number; resetsAt: number }

const QUESTIONS = ['AskUserQuestion', 'ExitPlanMode']

const WEEK = 7 * 86_400_000

export const LOGIN = /not logged in|\/login|invalid api key|failed to authenticate|oauth session expired/i

type Line = Record<string, any>

export function apiSignal(d: Line): { state: ApiState; retries?: number } | null {
  if (d.type === 'system' && d.subtype === 'api_retry') {
    if (d.error === 'overloaded' || d.error_status === 529) return { state: 'overloaded', retries: Number(d.attempt) || 1 }
    if (d.error === 'authentication_failed') return { state: 'signed_out' }
    if (typeof d.error_status === 'number' && d.error_status >= 500) return { state: 'outage' }
    if (d.error_status == null && (d.error === 'unknown' || d.no_response)) return { state: 'offline' }
    return null
  }
  if (d.type === 'assistant' && d.error === 'authentication_failed') return { state: 'signed_out' }
  if (d.type === 'result') {
    if (!d.is_error) return { state: 'ok' }
    if (LOGIN.test(String(d.result ?? ''))) return { state: 'signed_out' }
    if (d.api_error_status === 529) return { state: 'overloaded', retries: 0 }
    if (typeof d.api_error_status === 'number' && d.api_error_status >= 500) return { state: 'outage' }
    return null
  }
  if (d.type === 'rate_limit_event' || (d.type === 'stream_event' && d.event?.type === 'message_start')) return { state: 'ok' }
  return null
}

export function noted(api: ClaudeApi, signal: { state: ApiState; retries?: number } | null, now = Date.now()): ClaudeApi {
  if (!signal) return api
  const retries = signal.state === 'overloaded' ? (signal.retries ?? 0) : 0
  if (signal.state === api.state && retries === api.retries) return api
  return { state: signal.state, retries, since: signal.state === api.state ? api.since : now }
}

export function runningTool(agent: Agent) {
  const last = agent.entries?.at(-1)
  const block = last?.kind === 'agent' ? last.blocks.at(-1) : undefined
  const call = block?.type === 'tools' ? block.calls.at(-1) : undefined
  return call && !call.result && !INTERNAL.includes(call.name) ? call.name : null
}

export const waitingKind = (agent: Agent) => {
  const ask = agent.asks?.[0]
  return agent.status === 'waiting' && ask ? (QUESTIONS.includes(ask.tool_name) ? 'question' : 'permission') : null
}

export function activityOf(agent: Agent, archived = false): AgentActivity {
  const live = !archived
  return {
    id: agent.id,
    project: agent.project,
    name: agent.autoName ? '' : agent.name,
    status: agent.status,
    waiting: live ? waitingKind(agent) : null,
    tool: live && agent.status === 'working' ? runningTool(agent) : null,
    error: !live ? null : agent.status === 'error' ? (agent.alive ? 'api' : 'crash') : agent.mcpFailed ? 'mcp' : null,
    by: agent.startedBy ?? null,
    archived,
    since: Math.round(agent.since),
    unread: live && !!agent.unread,
  }
}

export function projectOf(project: Project, all: AgentActivity[], now = Date.now()): ProjectActivity {
  const mine = all.filter((a) => a.project === project.path)
  return {
    id: project.path,
    name: project.name,
    color: project.color || null,
    group: project.group || null,
    archived: project.archived,
    active: mine.reduce((last, a) => Math.max(last, a.since), 0),
    week_agents: mine.filter((a) => a.since >= now - WEEK).length,
    agents: mine.length,
  }
}

export const groupOf = (group: Group) => ({ id: group.id, name: group.name, parent: group.parent || null })

export function usageOf(limits: { five_hour?: Limit; seven_day?: Limit }, api: ClaudeApi) {
  const window = (limit?: Limit) => (limit ? { utilization: limit.utilization, resets_at: limit.resetsAt ? Math.round(limit.resetsAt * 1000) : null } : null)
  return { five_hour: window(limits.five_hour), seven_day: window(limits.seven_day), api: api.state, retries: api.retries, since: api.since }
}

export type Fed = { name: string; data: unknown }

export function agentChanges(seen: Map<string, string>, live: AgentActivity[], archived: Set<string>, first: boolean) {
  const events: Fed[] = []
  const now = new Set(live.map((a) => a.id))
  for (const agent of live) {
    const json = JSON.stringify(agent)
    const before = seen.get(agent.id)
    seen.set(agent.id, json)
    if (first || before === json) continue
    events.push({ name: before === undefined && !archived.has(agent.id) ? 'agent.started' : 'agent.status', data: agent })
  }
  const gone = [...seen.keys()].filter((id) => !now.has(id))
  for (const id of gone) seen.delete(id)
  return { events, gone }
}

const shape = (p: ProjectActivity) => JSON.stringify([p.name, p.color, p.group, p.archived])

export function projectChanges(seen: Map<string, string>, list: ProjectActivity[], first: boolean) {
  const events: Fed[] = []
  const now = new Set(list.map((p) => p.id))
  for (const project of list) {
    const before = seen.get(project.id)
    const json = shape(project)
    seen.set(project.id, json)
    if (first || before === json) continue
    const name = before === undefined ? 'project.added' : project.archived && !JSON.parse(before)[3] ? 'project.archived' : 'project.changed'
    events.push({ name, data: project })
  }
  for (const id of [...seen.keys()].filter((id) => !now.has(id))) {
    seen.delete(id)
    if (!first) events.push({ name: 'project.removed', data: { id, name: '', color: null, group: null, archived: true, active: 0, week_agents: 0, agents: 0 } })
  }
  return events
}
