import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { persist } from './agents.svelte'
import { plain } from './shell'
import type { Agent } from './stream'
import type { Shell } from './shells.svelte'

const MAX = 400_000
const runs = new Map<number, { agent: Agent; entry: Shell; raw: string; done: (entry: Shell) => void }>()
const early = new Map<number, { data: string; ended?: boolean; code?: number | null }>()

listen<{ id: number; data: string }>('sudo-out', ({ payload: { id, data } }) => {
  const run = runs.get(id)
  if (!run) return void early.set(id, { ...early.get(id), data: (early.get(id)?.data ?? '') + data })
  run.raw = (run.raw + data).slice(-MAX)
  run.entry.output = plain(run.raw)
})

listen<{ id: number; code: number | null }>('sudo-exit', ({ payload: { id, code } }) => {
  if (runs.has(id)) end(id, code)
  else early.set(id, { data: early.get(id)?.data ?? '', ended: true, code })
})

function end(id: number, code: number | null) {
  const run = runs.get(id)!
  runs.delete(id)
  Object.assign(run.entry, { state: code === null ? 'stopped' : 'done', code, ms: Date.now() - run.entry.at, output: plain(run.raw), sudo: true })
  persist(run.agent)
  run.done(run.entry)
}

export function follow(agent: Agent, id: number, command: string) {
  agent.entries.push({ kind: 'shell', at: Date.now(), command, output: '', state: 'running', sent: true, sudo: id })
  const entry = agent.entries.at(-1) as Shell
  persist(agent)
  return new Promise<Shell>((done) => {
    runs.set(id, { agent, entry, raw: '', done })
    const before = early.get(id)
    early.delete(id)
    if (before?.data) {
      runs.get(id)!.raw = before.data
      entry.output = plain(before.data)
    }
    if (before?.ended) end(id, before.code ?? null)
  })
}

export function stopSudo(entry: Shell) {
  if (typeof entry.sudo === 'number') invoke('sudo_stop', { id: entry.sudo })
}
