import { invoke } from '@tauri-apps/api/core'
import { clock } from './agents.svelte'
import { projects, type Project } from './projects.svelte'
import { settings } from './settings.svelte'

export type Run = {
  host: 'github' | 'bitbucket'
  id: string
  number: number
  name: string
  title: string
  event: string
  branch: string
  status: 'queued' | 'running' | 'success' | 'failure' | 'cancelled' | 'skipped'
  url: string
  started: string
  finished: string | null
}

export const runs = $state<Record<string, { list: Run[]; error: string }>>({})
const dismissed = $state<Record<string, true>>({})
const fetched: Record<string, number> = {}

export const active = (run: Run) => run.status === 'queued' || run.status === 'running'
export const enabled = (project: Project) => settings.runs && project.runs && project.exists
export const hiddenIn = (project: Project, name: string) => settings.runs_hidden.includes(name) || project.runs_hidden.includes(name)
const key = (project: Project, run: Run) => `${project.path}#${run.id}`

export function shown(project: Project): Run[] {
  if (!enabled(project)) return []
  const linger = settings.runs_linger * 60_000
  return (runs[project.path]?.list ?? []).filter(
    (run) =>
      !hiddenIn(project, run.name) &&
      !dismissed[key(project, run)] &&
      (active(run) || (run.status !== 'skipped' && !!run.finished && clock.now - Date.parse(run.finished) < linger)),
  )
}

export function outcome(project: Project): 'running' | 'failure' | 'success' | null {
  const list = shown(project)
  if (list.some(active)) return 'running'
  if (list.some((r) => r.status === 'failure')) return 'failure'
  return list.length ? 'success' : null
}

export const names = (project: Project) => [...new Set((runs[project.path]?.list ?? []).map((r) => r.name))].filter(Boolean)

export const dismiss = (project: Project, run: Run) => (dismissed[key(project, run)] = true)

export async function lookUpRuns(project: Project, force = false) {
  const found = runs[project.path]
  const every = found?.error ? 300_000 : found?.list.some(active) || project.path === projects.current ? 15_000 : 60_000
  if (!force && Date.now() - (fetched[project.path] ?? 0) < every) return
  fetched[project.path] = Date.now()
  try {
    runs[project.path] = { list: await invoke<Run[]>('pipeline_runs', { dir: project.path, repository: project.repository }), error: '' }
  } catch (e) {
    runs[project.path] = { list: [], error: String(e) }
  }
}

export function watchRuns() {
  const tick = () => projects.list.filter(enabled).forEach((p) => lookUpRuns(p))
  tick()
  setInterval(tick, 5_000)
}
