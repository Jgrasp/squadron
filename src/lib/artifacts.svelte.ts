import { invoke } from '@tauri-apps/api/core'
import { emit, emitTo, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { find, send, show } from './agents.svelte'
import type { ArtifactInfo } from './artifacts'
import { subagentOf, type Agent, type Subagent } from './stream'

export const shelf = $state<{ root: string; of: Record<string, ArtifactInfo[]> }>({ root: '', of: {} })

invoke<string>('artifacts_dir')
  .then((root) => (shelf.root = root))
  .catch(() => {})

export async function refresh(agent: string) {
  shelf.of[agent] = await invoke<ArtifactInfo[]>('list_artifacts', { agent }).catch(() => [])
}

listen<string>('artifacts-changed', ({ payload }) => refresh(payload))

export const pointed = $state<Record<string, string[]>>({})

export type Progress = { done: number; total: number; message: string }

export type Work = { destination: string; verb: 'share' | 'update' | 'unshare'; by: string; since: number; extension?: string; progress?: Progress; done?: number; after?: number }

export type Band = Work & { state: 'work' | 'wait' | 'done' | 'err'; reason?: string; asker?: string; queued?: boolean }

export const working = $state<Record<string, Work>>({})

export const bands = $state<Record<string, Band>>({})

export const workKey = (agent: string, artifact: string) => `${agent}/${artifact}`

const main = () => getCurrentWindow().label === 'main'

export function setWork(key: string, work: Work | null) {
  if (!main()) return void emitTo('main', 'artifact-work', { key, work })
  const agent = find(key.slice(0, key.indexOf('/')))
  if (work?.extension && work.after === undefined && agent) work = { ...work, after: agent.turns + (agent.status === 'working' || agent.status === 'waiting' ? 1 : 0) }
  if (work) working[key] = work
  else delete working[key]
}

export function finish(key: string) {
  if (!main()) return void emitTo('main', 'artifact-finish', key)
  const work = working[key]
  if (!work) return
  const done = Date.now()
  working[key] = { ...work, done }
  setTimeout(() => working[key]?.done === done && delete working[key], 5000)
}

export function shareFork(agent: Agent, work: Work): Subagent | undefined {
  if (!work.extension || work.verb !== 'share') return
  return agent.subagents.findLast((s) => s.started >= work.since - 2000 && s.name.startsWith(`/${work.extension}:`))
}

export function workOf(agent: Agent, sub: Subagent): [string, Work] | undefined {
  return Object.entries(working).find(([key, work]) => key.startsWith(`${agent.id}/`) && shareFork(agent, work) === sub)
}

const lastText = (sub: Subagent) => sub.log.findLast((b) => b.type === 'text')?.text.trim() ?? ''

function bandOf(key: string, work: Work): Band {
  if (work.done) return { ...work, state: 'done' }
  const agent = find(key.slice(0, key.indexOf('/')))
  if (!agent || !work.extension || work.verb !== 'share') return { ...work, state: 'work' }
  const sub = shareFork(agent, work)
  if (sub && agent.asks.some((a) => subagentOf(agent, a.tool_use_id) === sub)) return { ...work, state: 'wait', asker: sub.name.slice(sub.name.lastIndexOf(':') + 1) }
  if (sub && sub.status !== 'running') return { ...work, state: 'err', reason: lastText(sub) }
  if (!sub && work.after !== undefined && agent.turns > work.after) return { ...work, state: 'err' }
  return { ...work, state: 'work', queued: !sub && work.after !== undefined && agent.turns < work.after }
}

function replace(next: Record<string, Band>) {
  for (const key of Object.keys(bands)) if (!(key in next)) delete bands[key]
  Object.assign(bands, next)
}

if (main()) {
  listen<{ key: string; work: Work | null }>('artifact-work', ({ payload }) => setWork(payload.key, payload.work))
  listen<string>('artifact-finish', ({ payload }) => finish(payload))
  listen('artifact-bands-wanted', () => emit('artifact-bands', $state.snapshot(bands)))
  $effect.root(() => {
    $effect(() => {
      replace(Object.fromEntries(Object.entries(working).map(([key, work]) => [key, bandOf(key, work)])))
      emit('artifact-bands', $state.snapshot(bands))
    })
  })
} else {
  listen<Record<string, Band>>('artifact-bands', ({ payload }) => replace(payload))
  emitTo('main', 'artifact-bands-wanted')
}

listen<{ agent: string; kind: 'point' | 'share' | 'show'; text: string; hint?: string }>('artifact-do', ({ payload: { agent, kind, text, hint } }) => {
  if (!main()) return
  const found = find(agent)
  if (kind === 'show' && found) answer(found)
  else if (kind === 'share') send(agent, text, [], [], '', hint)
  else pointed[agent] = [...(pointed[agent] ?? []), text]
})

export function answer(agent: Agent) {
  show(agent)
  getCurrentWindow().setFocus()
  requestAnimationFrame(() => document.querySelector('[data-fork-ask]')?.scrollIntoView({ block: 'center', behavior: 'smooth' }))
}

export async function migrate(agent: Agent) {
  for (const a of agent.artifacts ?? []) {
    const id = a.id.replace(/[^\w.-]/g, '-') || 'artifact'
    await invoke('write_artifact', { agent: agent.id, id, kind: a.kind, content: a.content })
    if (a.link) await invoke('set_artifact_link', { agent: agent.id, id, link: a.link })
  }
  agent.artifacts = undefined
  await refresh(agent.id)
}
