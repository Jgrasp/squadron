import { invoke } from '@tauri-apps/api/core'

export function links(event: MouseEvent) {
  const link = (event.target as HTMLElement).closest?.('a[href]')
  const href = link?.getAttribute('href')
  if (!href || href.startsWith('#')) return
  event.preventDefault()
  if (/^https?:\/\//.test(href)) invoke('open_link', { url: href })
  else if (/^(\/|file:\/\/)/.test(href)) invoke('open_attachment', { path: decodeURI(href.replace(/^file:\/\//, '')) }).catch(() => {})
}
