import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { find, persist } from './agents.svelte'
import { plain, said } from './shell'
import { fail, starting, workdir, type Agent, type Entry } from './stream'

export type Shell = Extract<Entry, { kind: 'shell' }>

type Run = { agent: string; entry: Shell; raw: string; reader?: (data: string) => void }

const MAX = 400_000
const runs = new Map<number, Run>()
const early = new Map<number, { data: string; ended?: boolean; code?: number | null }>()

listen<{ id: number; data: string }>('shell-out', ({ payload: { id, data } }) => {
  const run = runs.get(id)
  if (!run) return void early.set(id, { ...early.get(id), data: (early.get(id)?.data ?? '') + data })
  run.raw = (run.raw + data).slice(-MAX)
  run.reader?.(data)
})

listen<{ id: number; code: number | null }>('shell-exit', ({ payload: { id, code } }) => {
  if (runs.has(id)) end(id, code)
  else early.set(id, { data: early.get(id)?.data ?? '', ended: true, code })
})

function end(id: number, code: number | null) {
  const run = runs.get(id)!
  runs.delete(id)
  const { entry } = run
  entry.state = code === null ? 'stopped' : 'done'
  entry.code = code
  entry.ms = Date.now() - entry.at
  entry.output = plain(run.raw)
  delete entry.pty
  const agent = find(run.agent)
  if (!agent) return
  persist(agent)
  deliver(agent)
}

export async function run(agent: Agent, command: string) {
  agent.entries.push({ kind: 'shell', at: Date.now(), command, output: '', state: 'running' })
  const entry = agent.entries.at(-1) as Shell
  persist(agent)
  try {
    const id = await invoke<number>('shell_run', { dir: workdir(agent), command, cols: 100, rows: 3 })
    entry.pty = id
    runs.set(id, { agent: agent.id, entry, raw: '' })
    const before = early.get(id)
    early.delete(id)
    if (before?.data) runs.get(id)!.raw = before.data
    if (before?.ended) end(id, before.code ?? null)
  } catch (e) {
    Object.assign(entry, { state: 'done', code: -1, ms: 0, output: String(e) })
    persist(agent)
    deliver(agent)
  }
}

export function attach(id: number, reader: (data: string) => void) {
  const run = runs.get(id)
  if (!run) return ''
  run.reader = reader
  return run.raw
}

export function detach(id: number) {
  const run = runs.get(id)
  if (run) run.reader = undefined
}

export function stop(entry: Shell) {
  if (entry.pty) invoke('term_close', { id: entry.pty })
}

export function take(agent: Agent) {
  const ready = agent.entries.filter((e): e is Shell => e.kind === 'shell' && e.state !== 'running' && !e.sent)
  for (const e of ready) e.sent = true
  return said(ready)
}

export function deliver(agent: Agent) {
  if (agent.status === 'working' || agent.status === 'waiting') return
  const text = take(agent)
  if (!text) return
  starting(agent)
  persist(agent)
  invoke('send_message', { id: agent.id, text }).catch((e) => fail(agent, String(e)))
}
