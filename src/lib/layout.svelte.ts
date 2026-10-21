export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

export const MIN_PANEL = 420

export type PanelKind = 'diff' | 'editor' | 'terminal'

const read = (key: string) => JSON.parse(localStorage.getItem(key) ?? '{}')

let stored = 304
let folded = false
let split = false
let widths: Record<string, number> = { unified: 680, split: 1120, editor: 780, terminal: 680 }
let full: Record<string, boolean> = {}
let panels: Record<string, PanelKind> = {}
try {
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
  folded = localStorage.getItem('squadron.collapsed') === '1'
  split = localStorage.getItem('squadron.diff') === 'split'
  widths = { ...widths, ...read('squadron.diff-widths'), ...read('squadron.widths') }
  full = { diff: localStorage.getItem('squadron.diff-full') === '1', ...read('squadron.full') }
  panels = read('squadron.panels')
} catch {}

export const layout = $state({
  sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)),
  collapsed: folded,
  split,
  widths,
  full,
  panels,
})

function keep(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {}
}

export function setSplit(on: boolean) {
  layout.split = on
  keep('squadron.diff', on ? 'split' : 'unified')
}

export function resizePanel(size: string, width: number, max: number) {
  layout.widths[size] = Math.round(Math.max(MIN_PANEL, Math.min(max, width)))
  keep('squadron.widths', JSON.stringify(layout.widths))
}

export function toggleFull(kind: PanelKind) {
  layout.full[kind] = !layout.full[kind]
  keep('squadron.full', JSON.stringify(layout.full))
}

export function setPanel(project: string, kind: PanelKind | null) {
  if (kind) layout.panels[project] = kind
  else delete layout.panels[project]
  keep('squadron.panels', JSON.stringify(layout.panels))
}

export const FOLDED_SIDEBAR = 68
const FOLD_BELOW = (FOLDED_SIDEBAR + MIN_SIDEBAR) / 2

export function setCollapsed(collapsed: boolean) {
  if (layout.collapsed === collapsed) return
  layout.collapsed = collapsed
  keep('squadron.collapsed', collapsed ? '1' : '0')
}

export function toggleSidebar() {
  setCollapsed(!layout.collapsed)
}

export function dragSidebar(width: number) {
  setCollapsed(width < FOLD_BELOW)
  if (!layout.collapsed) resizeSidebar(width)
}

export function resizeSidebar(width: number) {
  layout.sidebar = Math.round(Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, width)))
  keep('squadron.sidebar', String(layout.sidebar))
}
