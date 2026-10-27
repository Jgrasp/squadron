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

export function remember(project: string) {
  const ws = workspaces[project]
  if (!ws) return
  stored[project] = { files: ws.files.map((f) => f.path), active: ws.active, expanded: ws.expanded, preview: ws.preview }
  try {
    localStorage.setItem(KEY, JSON.stringify(stored))
  } catch {}
}
