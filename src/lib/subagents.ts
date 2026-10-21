import { subagentOf, type Agent, type Subagent } from './stream'

export type Look = 'waiting' | 'error' | 'running' | 'done' | 'stopped'

export const LOOKS: Look[] = ['running', 'waiting', 'error', 'done', 'stopped']

export const LABEL: Record<Look, string> = {
  running: 'running',
  waiting: 'waiting for you',
  error: 'failed',
  done: 'done',
  stopped: 'stopped',
}

export function asking(agent: Agent) {
  return new Set(agent.asks.map((a) => subagentOf(agent, a.tool_use_id)?.id).filter(Boolean))
}

export function lookOf(sub: Subagent, waiting: Set<string | undefined>): Look {
  return waiting.has(sub.id) ? 'waiting' : sub.status
}

export function count(subs: Subagent[], waiting: Set<string | undefined>) {
  const out: Record<Look, number> = { running: 0, waiting: 0, error: 0, done: 0, stopped: 0 }
  for (const s of subs) out[lookOf(s, waiting)]++
  return out
}

export function summary(subs: Subagent[], waiting: Set<string | undefined>) {
  const c = count(subs, waiting)
  const parts = [`${subs.length} subagent${subs.length > 1 ? 's' : ''}`]
  if (c.done) parts.push(`${c.done} done`)
  else if (c.running) parts.push(`${c.running} running`)
  if (c.error) parts.push(`${c.error} failed`)
  return parts.join(' · ')
}
