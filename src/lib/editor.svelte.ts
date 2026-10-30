import { invoke } from '@tauri-apps/api/core'
import { errorText } from './i18n/index.svelte'

export type OpenFile = { path: string; saved: string; dirty: boolean; error: string; base: string | null | undefined; ready: boolean; doc?: string }

type Workspace = { files: OpenFile[]; active: string; expanded: string[]; restore: string[]; preview: string[] }

const KEY = 'squadron.editor'

let stored: Record<string, { files: string[]; active: string; expanded: string[]; preview?: string[] }> = {}
try {
  stored = JSON.parse(localStorage.getItem(KEY) ?? '{}')
} catch {}

export const workspaces = $state<Record<string, Workspace>>({})

export function workspace(project: string) {
  const saved = stored[project]
  workspaces[project] ??= { files: [], active: saved?.active ?? '', expanded: saved?.expanded ?? [], restore: saved?.files ?? [], preview: saved?.preview ?? [] }
  return workspaces[project]
}

export function openFile(project: string, path: string, activate = true) {
  const ws = workspace(project)
  if (!ws.files.some((f) => f.path === path)) {
    ws.files.push({ path, saved: '', dirty: false, error: '', base: undefined, ready: false })
    const file = ws.files.at(-1)!
    invoke<string | null>('read_project_file', { project, path })
      .then((text) => ((file.saved = text ?? ''), (file.doc = file.saved)))
      .catch((e) => (file.error = errorText(e)))
      .finally(() => (file.ready = true))
  }
  if (activate) ws.active = path
  remember(project)
}

export function remember(project: string) {
  const ws = workspaces[project]
  if (!ws) return
  stored[project] = { files: ws.files.map((f) => f.path), active: ws.active, expanded: ws.expanded, preview: ws.preview }
  try {
    localStorage.setItem(KEY, JSON.stringify(stored))
  } catch {}
}
