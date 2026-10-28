export const PANEL_HEIGHT = { min: 24, max: 240, default: 64 }
export const PANEL_MAX_HTML = 50_000

const THEME = ['--text', '--text-strong', '--text-muted', '--text-faint', '--accent', '--accent-soft', '--ok', '--done', '--err', '--surface-1', '--surface-3', '--surface-5', '--border', '--sans', '--mono']

const CSP = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; form-action 'none'"

export function panelHeight(value: unknown) {
  const n = Math.round(Number(value))
  if (value === undefined || value === null || !Number.isFinite(n)) return PANEL_HEIGHT.default
  return Math.min(PANEL_HEIGHT.max, Math.max(PANEL_HEIGHT.min, n))
}

export function strip(html: string) {
  return html
    .replace(/<script\b[\s\S]*?(<\/script\s*>|$)/gi, '')
    .replace(/<\/?(script|meta|base|link|iframe|frame|frameset|object|embed|portal)\b[^>]*>?/gi, '')
}

export function panelDocument(html: string, theme: Record<string, string>, scroll = false) {
  const vars = THEME.filter((name) => theme[name]).map((name) => `${name}:${theme[name]}`).join(';')
  const body = scroll ? 'html,body{margin:0;background:transparent}body{padding:16px 20px;color:var(--text);font:400 14px/1.5 var(--sans)}' : 'html,body{margin:0;overflow:hidden;background:transparent}body{padding:8px 20px;color:var(--text);font:400 13px/1.4 var(--sans)}'
  return `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="${CSP}"><style>:root{${vars};color-scheme:dark}${body}</style></head><body>${strip(html)}</body></html>`
}

export function themeOf(element: Element) {
  const style = getComputedStyle(element)
  return Object.fromEntries(THEME.map((name) => [name, style.getPropertyValue(name).trim()]))
}
