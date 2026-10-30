export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

export const MIN_PANEL = 420

import { projects } from './projects.svelte'

export type PanelKind = 'diff' | 'editor' | 'terminal' | 'subagents' | 'artifacts' | 'kanban' | 'pulls'

const read = (key: string) => JSON.parse(localStorage.getItem(key) ?? '{}')

let stored = 304
let split = false
let fileTree = true
let widths: Record<string, number> = { unified: 680, split: 1120, editor: 780, terminal: 680, subagents: 680, artifacts: 680, panel: 760 }
let full: Record<string, boolean> = {}
let panels: Record<string, PanelKind> = {}

export type AgentSort = { by: 'none' | 'updated' | 'created' | 'name'; desc: boolean }
let sort: AgentSort = { by: 'none', desc: false }
let sorts: Record<string, AgentSort> = {}
let projectsWidth = 360
let foldedGroups0: Record<string, boolean> = {}
let orders: Record<string, string[]> = {}
try {
  sort = { ...sort, ...read('squadron.sort') }
  sorts = read('squadron.sorts')
  orders = read('squadron.agent-orders')
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
  split = localStorage.getItem('squadron.diff') === 'split'
  fileTree = localStorage.getItem('squadron.diff-tree') !== 'off'
  widths = { ...widths, ...read('squadron.diff-widths'), ...read('squadron.widths') }
  full = { diff: localStorage.getItem('squadron.diff-full') === '1', ...read('squadron.full') }
  panels = read('squadron.panels')
  projectsWidth = Number(localStorage.getItem('squadron.projects-width')) || 360
  foldedGroups0 = read('squadron.groups-folded')
} catch {}

export const layout = $state({
  sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)),
  split,
  fileTree,
  widths,
  full,
  panels,
  sort,
  sorts,
  orders,
  projectsWidth,
  foldedGroups: foldedGroups0,
  leaving: false,
})

export function setSort(project: string | null, next: Partial<AgentSort>) {
  if (project) layout.sorts[project] = { ...(layout.sorts[project] ?? layout.sort), ...next }
  else layout.sort = { ...layout.sort, ...next }
  keep('squadron.sort', JSON.stringify(layout.sort))
  keep('squadron.sorts', JSON.stringify(layout.sorts))
}

export function orderAgents(project: string, ids: string[]) {
  layout.orders[project] = ids
  keep('squadron.agent-orders', JSON.stringify(layout.orders))
}

export function resetSort(project: string) {
  delete layout.sorts[project]
  keep('squadron.sorts', JSON.stringify(layout.sorts))
}

function keep(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {}
}

export function setSplit(on: boolean) {
  layout.split = on
  keep('squadron.diff', on ? 'split' : 'unified')
}

export function setFileTree(on: boolean) {
  layout.fileTree = on
  keep('squadron.diff-tree', on ? 'on' : 'off')
}

export function resizePanel(size: string, width: number, max: number) {
  layout.widths[size] = Math.round(Math.max(MIN_PANEL, Math.min(max, width)))
  keep('squadron.widths', JSON.stringify(layout.widths))
}

export function toggleFull() {
  layout.full.panel = !layout.full.panel
  keep('squadron.full', JSON.stringify(layout.full))
}

let leaving = 0

export function closeOverlay(done: () => void) {
  if (layout.leaving) return
  layout.leaving = true
  leaving = window.setTimeout(() => ((layout.leaving = false), done()), 280)
}

export function setPanel(project: string, kind: PanelKind | null, now = false) {
  if (!kind && !now && layout.panels[project]) closeOverlay(() => showPanel(project, null))
  else showPanel(project, kind)
}

function showPanel(project: string, kind: PanelKind | null) {
  clearTimeout(leaving)
  layout.leaving = false
  if (kind) projects.board = projects.pulls = false
  if (kind) layout.panels[project] = kind
  else delete layout.panels[project]
  keep('squadron.panels', JSON.stringify(layout.panels))
}

export function resizeSidebar(width: number) {
  layout.sidebar = Math.round(Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, width)))
  keep('squadron.sidebar', String(layout.sidebar))
}

export const MIN_PROJECTS = 280

export function resizeProjects(width: number) {
  layout.projectsWidth = Math.round(Math.min(window.innerWidth, Math.max(MIN_PROJECTS, width)))
  keep('squadron.projects-width', String(layout.projectsWidth))
}

export function foldGroup(id: string, folded: boolean) {
  if (folded) layout.foldedGroups[id] = true
  else delete layout.foldedGroups[id]
  keep('squadron.groups-folded', JSON.stringify(layout.foldedGroups))
}
