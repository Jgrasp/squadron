export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

let stored = 304
let folded = false
try {
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
  folded = localStorage.getItem('squadron.collapsed') === '1'
} catch {}

export const layout = $state({ sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)), collapsed: folded })

export function toggleSidebar() {
  layout.collapsed = !layout.collapsed
  try {
    localStorage.setItem('squadron.collapsed', layout.collapsed ? '1' : '0')
  } catch {}
}

export function resizeSidebar(width: number) {
  layout.sidebar = Math.round(Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, width)))
  try {
    localStorage.setItem('squadron.sidebar', String(layout.sidebar))
  } catch {}
}
