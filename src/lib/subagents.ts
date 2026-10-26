import { t, type Key } from './i18n/index.svelte'
import { subagentOf, type Agent, type Subagent } from './stream'

export type Look = 'waiting' | 'error' | 'running' | 'done' | 'stopped'

export const LOOKS: Look[] = ['running', 'waiting', 'error', 'done', 'stopped']

const LABELS: Record<Look, Key> = {
  running: 'conversation.stateRunning',
  waiting: 'conversation.stateWaiting',
  error: 'conversation.stateError',
  done: 'conversation.stateDone',
  stopped: 'conversation.stateStopped',
}

export const label = (look: Look) => t(LABELS[look])

const COUNTS: Record<Look, Key> = {
  running: 'conversation.countRunning',
  waiting: 'conversation.countWaiting',
  error: 'conversation.countError',
  done: 'conversation.countDone',
  stopped: 'conversation.countStopped',
}

export const counted = (look: Look, n: number) => t(COUNTS[look], { count: n })

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
  const parts = [t('conversation.subagentCount', { count: subs.length })]
  if (c.done) parts.push(counted('done', c.done))
  else if (c.running) parts.push(counted('running', c.running))
  if (c.error) parts.push(counted('error', c.error))
  return parts.join(' · ')
}
