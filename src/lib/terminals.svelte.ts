import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type Session = { id: number; project: string; title: string; ended: boolean }

export const sessions = $state({ list: [] as Session[], current: {} as Record<string, number> })

const MAX_LOG = 400_000
const logs = new Map<number, string>()
const readers = new Map<number, (data: string) => void>()

function output(id: number, data: string) {
  logs.set(id, ((logs.get(id) ?? '') + data).slice(-MAX_LOG))
  readers.get(id)?.(data)
}

listen<{ id: number; data: string }>('term-out', ({ payload }) => output(payload.id, payload.data))

listen<number>('term-exit', ({ payload: id }) => {
  const session = sessions.list.find((s) => s.id === id)
  if (session) session.ended = true
  output(id, '\r\n\x1b[2m[Process exited]\x1b[0m\r\n')
})

export function ofProject(project: string) {
  return sessions.list.filter((s) => s.project === project)
}

export async function start(project: string, cols: number, rows: number) {
  const { id, shell } = await invoke<{ id: number; shell: string }>('term_open', { project, cols, rows })
  sessions.list.push({ id, project, title: shell, ended: false })
  sessions.current[project] = id
}

export function close(id: number) {
  const session = sessions.list.find((s) => s.id === id)
  invoke('term_close', { id })
  logs.delete(id)
  readers.delete(id)
  sessions.list = sessions.list.filter((s) => s.id !== id)
  if (session && sessions.current[session.project] === id) {
    const next = ofProject(session.project).at(-1)
    if (next) sessions.current[session.project] = next.id
    else delete sessions.current[session.project]
  }
}

export function attach(id: number, reader: (data: string) => void) {
  readers.set(id, reader)
  return logs.get(id) ?? ''
}

export function detach(id: number) {
  readers.delete(id)
}
