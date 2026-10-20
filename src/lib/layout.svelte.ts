export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

export const MIN_DIFF = 420

let stored = 304
let folded = false
let split = false
let widths = { unified: 680, split: 1120 }
let full = false
try {
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
  folded = localStorage.getItem('squadron.collapsed') === '1'
  split = localStorage.getItem('squadron.diff') === 'split'
  widths = { ...widths, ...JSON.parse(localStorage.getItem('squadron.diff-widths') ?? '{}') }
  full = localStorage.getItem('squadron.diff-full') === '1'
} catch {}

export const layout = $state({
  sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)),
  collapsed: folded,
  split,
  diffWidths: widths,
  diffFull: full,
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

export function resizeDiff(width: number, max: number) {
  layout.diffWidths[layout.split ? 'split' : 'unified'] = Math.round(Math.max(MIN_DIFF, Math.min(max, width)))
  keep('squadron.diff-widths', JSON.stringify(layout.diffWidths))
}

export function toggleDiffFull() {
  layout.diffFull = !layout.diffFull
  keep('squadron.diff-full', layout.diffFull ? '1' : '0')
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
