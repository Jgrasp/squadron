import type { Remark } from './pulls.svelte'

export const seen = $state<Record<string, string[]>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.seen-remarks') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

export function markSeen(key: string, ids: string[]) {
  seen[key] = [...new Set([...(seen[key] ?? []), ...ids])]
  try {
    localStorage.setItem('squadron.seen-remarks', JSON.stringify(seen))
  } catch {}
}

export const settle = (key: string, found: Remark[]) => !seen[key] && markSeen(key, found.map((r) => r.id))

export const isUnread = (key: string, r: Remark, self: string) => r.author !== self && !(seen[key] ?? []).includes(r.id)
