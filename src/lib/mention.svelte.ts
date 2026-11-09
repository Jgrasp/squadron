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

const MENTION = /(^|\s)@(?:"([^"\n]+)"|([^\s@"]+))/g

export function citedFiles(text: string, files: Set<string>) {
  const found: { from: number; to: number; path: string }[] = []
  for (const m of text.matchAll(MENTION)) {
    const from = m.index + m[1].length
    let raw = m[2] ?? m[3]
    let to = from + 1 + (m[2] ? raw.length + 2 : raw.length)
    if (!m[2] && !files.has(raw) && !files.has(`${raw}/`)) {
      const bare = raw.replace(/[.,;:!?)]+$/, '')
      to -= raw.length - bare.length
      raw = bare
    }
    const path = files.has(raw) ? raw : files.has(`${raw}/`) ? `${raw}/` : ''
    if (path) found.push({ from, to, path })
  }
  return found
}

export const filesIn = (list: string[], dir: string) => list.filter((f) => f.startsWith(dir) && !f.endsWith('/')).length

export function splitCited(text: string, files: Set<string>) {
  const parts: { text: string; path?: string }[] = []
  let last = 0
  for (const cited of citedFiles(text, files)) {
    if (cited.from > last) parts.push({ text: text.slice(last, cited.from) })
    parts.push({ text: text.slice(cited.from, cited.to), path: cited.path })
    last = cited.to
  }
  if (last < text.length) parts.push({ text: text.slice(last) })
  return parts
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
      if (!this.open && !this.#cited) return
      const dir = this.#dir()
      invoke<string[]>('project_files', { dir })
        .then((list) => (this.#files = { dir, list }))
        .catch(() => (this.#files = { dir, list: [] }))
    })
  }

  found = $derived.by(() => mentionAt(this.#text(), this.caret))
  open = $derived(!!this.found)
  #cited = $derived.by(() => this.#text().includes('@'))
  paths = $derived.by(() => (this.#files.dir === this.#dir() ? this.#files.list : []))
  choices = $derived.by(() => (this.#files.dir === this.#dir() ? fileChoices(this.#files.list, this.#text(), this.caret) : []))

  track = (caret: number) => {
    this.caret = caret
  }
}
