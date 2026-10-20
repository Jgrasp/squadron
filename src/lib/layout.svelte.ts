export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

let stored = 304
let folded = false
try {
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
  folded = localStorage.getItem('squadron.collapsed') === '1'
} catch {}

export const layout = $state({ sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)), collapsed: folded })

export const FOLDED_SIDEBAR = 68
const FOLD_BELOW = (FOLDED_SIDEBAR + MIN_SIDEBAR) / 2

export function setCollapsed(collapsed: boolean) {
  if (layout.collapsed === collapsed) return
  layout.collapsed = collapsed
  try {
    localStorage.setItem('squadron.collapsed', collapsed ? '1' : '0')
  } catch {}
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
  try {
    localStorage.setItem('squadron.sidebar', String(layout.sidebar))
  } catch {}
}
