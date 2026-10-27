import { invoke } from '@tauri-apps/api/core'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { clock } from './agents.svelte'
import { projects, type Project } from './projects.svelte'
import { settings } from './settings.svelte'
import { t, type Key } from './i18n/index.svelte'

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
  author: string
  author_avatar: string
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

export const names = (project: Project) => [...new Set((runs[project.path]?.list ?? []).map((r) => r.name))].filter(Boolean)

export const dismiss = (project: Project, run: Run) => (dismissed[key(project, run)] = true)

export async function lookUpRuns(project: Project, force = false) {
  const found = runs[project.path]
  const every = found?.error ? 300_000 : found?.list.some(active) || project.path === projects.current ? 15_000 : 60_000
  if (!force && Date.now() - (fetched[project.path] ?? 0) < every) return
  fetched[project.path] = Date.now()
  try {
    const list = await invoke<Run[]>('pipeline_runs', { dir: project.path, repository: project.repository })
    if (found && !found.error) announce(project, found.list, list).catch(() => {})
    runs[project.path] = { list, error: '' }
  } catch (e) {
    runs[project.path] = { list: [], error: String(e) }
  }
}

const ENDED: Partial<Record<Run['status'], Key>> = {
  success: 'runs.notifySuccess',
  failure: 'runs.notifyFailure',
  cancelled: 'runs.notifyCancelled',
}

async function announce(project: Project, before: Run[], after: Run[]) {
  if (!settings.runs_notify || !project.runs_notify) return
  const ended = after.filter((run) => ENDED[run.status] && !hiddenIn(project, run.name) && before.some((b) => b.id === run.id && active(b)))
  if (!ended.length || !((await isPermissionGranted()) || (await requestPermission()) === 'granted')) return
  for (const run of ended)
    sendNotification({ title: t(ENDED[run.status]!, { name: run.name || `#${run.number}`, project: project.name }), body: [run.title, run.branch].filter(Boolean).join(' · ') })
}

export function watchRuns() {
  const tick = () => projects.list.filter(enabled).forEach((p) => lookUpRuns(p))
  tick()
  setInterval(tick, 5_000)
}
