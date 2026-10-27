import { invoke } from '@tauri-apps/api/core'

export type FileChoice = { value: string; label: string; text: string; at: number }

export function rank(path: string, query: string) {
  const lower = path.toLowerCase()
  const name = lower.replace(/\/$/, '').split('/').pop()!
  if (!query) return path.replace(/\/$/, '').includes('/') ? -1 : 0
  if (name.startsWith(query)) return 0
  if (name.includes(query)) return 1
  if (lower.includes(query)) return 2
  let i = 0
  for (const ch of lower) if (ch === query[i]) i++
  return i === query.length ? 3 : -1
}

export function mentionAt(text: string, caret: number) {
  const m = /(^|\s)@([^\s@"]*)$/.exec(text.slice(0, caret))
  return m ? { start: m.index + m[1].length, query: m[2].toLowerCase() } : null
}

export function fileChoices(paths: string[], text: string, caret: number): FileChoice[] {
  const found = mentionAt(text, caret)
  if (!found) return []
  return paths
    .map((path) => ({ path, score: rank(path, found.query) }))
    .filter((f) => f.score >= 0)
    .sort((a, b) => a.score - b.score || a.path.length - b.path.length || a.path.localeCompare(b.path))
    .slice(0, 50)
    .map(({ path }) => {
      const token = `@${/\s/.test(path) ? `"${path}"` : path}${path.endsWith('/') ? '' : ' '}`
      return { value: path, label: `@${path}`, text: text.slice(0, found.start) + token + text.slice(caret), at: found.start + token.length }
    })
}

export class Mention {
  caret = $state(0)
  #files = $state<{ dir: string; list: string[] }>({ dir: '', list: [] })
  #text: () => string
  #dir: () => string

  constructor(text: () => string, dir: () => string) {
    this.#text = text
    this.#dir = dir
    $effect(() => {
      if (!this.open) return
      const dir = this.#dir()
      invoke<string[]>('project_files', { dir })
        .then((list) => (this.#files = { dir, list }))
        .catch(() => (this.#files = { dir, list: [] }))
    })
  }

  found = $derived.by(() => mentionAt(this.#text(), this.caret))
  open = $derived(!!this.found)
  choices = $derived.by(() => (this.#files.dir === this.#dir() ? fileChoices(this.#files.list, this.#text(), this.caret) : []))

  track = (event: Event) => {
    this.caret = (event.currentTarget as HTMLTextAreaElement).selectionStart
  }
}
