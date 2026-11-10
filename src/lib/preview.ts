import { invoke } from '@tauri-apps/api/core'

export type Preview = { title: string; description: string; site: string; image: string | null; icon: string | null }

const known = new Map<string, Promise<Preview | null>>()

export function preview(url: string) {
  let hit = known.get(url)
  if (!hit) known.set(url, (hit = invoke<Preview | null>('link_preview', { url }).catch(() => null)))
  return hit
}

export function siteOf(url: string) {
  try {
    const parsed = new URL(url)
    const rest = parsed.pathname.replace(/\/$/, '') + parsed.search + parsed.hash
    let path = rest
    try {
      path = decodeURI(rest)
    } catch {}
    return { host: parsed.host.replace(/^www\./, ''), path, favicon: `${parsed.origin}/favicon.ico` }
  } catch {
    return { host: url, path: '', favicon: '' }
  }
}
