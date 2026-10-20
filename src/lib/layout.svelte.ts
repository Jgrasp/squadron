export const MIN_SIDEBAR = 240
export const MAX_SIDEBAR = 560

let stored = 304
try {
  stored = Number(localStorage.getItem('squadron.sidebar')) || 304
} catch {}

export const layout = $state({ sidebar: Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, stored)) })

export function resizeSidebar(width: number) {
  layout.sidebar = Math.round(Math.min(MAX_SIDEBAR, Math.max(MIN_SIDEBAR, width)))
  try {
    localStorage.setItem('squadron.sidebar', String(layout.sidebar))
  } catch {}
}
