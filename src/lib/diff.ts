import { diffLines } from 'diff'
import type { ToolCall } from './stream'

export const EDITS = ['Edit', 'MultiEdit', 'Write', 'NotebookEdit']

export type Hunk = { oldStart: number; newStart: number; lines: string[] }

export type Proposal = { ask: string; path: string; hunks: Hunk[] }

export type Changes = { from: string; now: string; files: { path: string; status: string; add: number | null; del: number | null }[] }

export type Row = { kind: 'head'; text: string; hunk: number } | { kind: 'add' | 'del' | 'same'; old?: number; new?: number; text: string }

export type Cell = { kind: 'add' | 'del' | 'same'; number: number; text: string }

export type Pair = { kind: 'head'; text: string; hunk: number } | { kind: 'pair'; left: Cell | null; right: Cell | null }

export function rows(hunks: Hunk[]): Row[] {
  const out: Row[] = []
  for (const [index, hunk] of hunks.entries()) {
    let old = hunk.oldStart
    let now = hunk.newStart
    const lines = hunk.lines.filter((l) => !l.startsWith('\\'))
    const oldCount = lines.filter((l) => l[0] !== '+').length
    const newCount = lines.filter((l) => l[0] !== '-').length
    if (oldCount > 0) out.push({ kind: 'head', hunk: index, text: `@@ -${hunk.oldStart},${oldCount} +${hunk.newStart},${newCount} @@` })
    for (const line of lines) {
      const text = line.slice(1)
      if (line[0] === '+') out.push({ kind: 'add', new: now++, text })
      else if (line[0] === '-') out.push({ kind: 'del', old: old++, text })
      else out.push({ kind: 'same', old: old++, new: now++, text })
    }
  }
  return out
}

export function pairs(hunks: Hunk[]): Pair[] {
  const out: Pair[] = []
  let dels: Cell[] = []
  let adds: Cell[] = []
  const flush = () => {
    for (let i = 0; i < Math.max(dels.length, adds.length); i++) out.push({ kind: 'pair', left: dels[i] ?? null, right: adds[i] ?? null })
    dels = []
    adds = []
  }
  for (const row of rows(hunks)) {
    if (row.kind === 'del') {
      if (adds.length) flush()
      dels.push({ kind: 'del', number: row.old!, text: row.text })
    } else if (row.kind === 'add') adds.push({ kind: 'add', number: row.new!, text: row.text })
    else {
      flush()
      out.push(
        row.kind === 'head'
          ? row
          : { kind: 'pair', left: { kind: 'same', number: row.old!, text: row.text }, right: { kind: 'same', number: row.new!, text: row.text } },
      )
    }
  }
  flush()
  return out
}

export const fileLines = (text: string) => (text.endsWith('\n') ? text.slice(0, -1) : text).split('\n')

function newCount(hunk: Hunk) {
  return hunk.lines.filter((l) => l[0] !== '-' && l[0] !== '\\').length
}

export function hidden(hunks: Hunk[], total: number): number[] {
  const out: number[] = []
  let end = 0
  for (const hunk of hunks) {
    out.push(Math.max(0, hunk.newStart - end - 1))
    const count = newCount(hunk)
    end = count ? hunk.newStart + count - 1 : hunk.newStart
  }
  out.push(Math.max(0, total - end))
  return out
}

export function reveal(hunks: Hunk[], file: string[], gap: number, way: 'up' | 'down', count = Infinity): Hunk[] {
  if (!hunks.length) return hunks
  const free = hidden(hunks, file.length)[gap] ?? 0
  const n = Math.min(count, free)
  if (!n) return hunks
  const out = hunks.map((h) => ({ ...h, lines: [...h.lines] }))
  if ((way === 'up' && gap < out.length) || gap === 0) {
    const hunk = out[gap]
    hunk.lines.unshift(...file.slice(hunk.newStart - 1 - n, hunk.newStart - 1).map((l) => ` ${l}`))
    hunk.newStart -= n
    hunk.oldStart -= n
  } else {
    const hunk = out[gap - 1]
    const after = hunk.newStart + newCount(hunk) - 1
    hunk.lines.push(...file.slice(after, after + n).map((l) => ` ${l}`))
  }
  if (n === free && gap > 0 && gap < out.length) {
    out[gap - 1].lines.push(...out[gap].lines)
    out.splice(gap, 1)
  }
  return out
}

export function everything(hunks: Hunk[], file: string[]): Hunk[] {
  for (let gap = hunks.length; gap >= 0; gap--) hunks = reveal(hunks, file, gap, gap ? 'down' : 'up')
  return hunks
}

export function hunks(call: ToolCall): Hunk[] {
  const data = call.result?.data
  if (!data) return []
  if (Array.isArray(data.structuredPatch) && data.structuredPatch.length)
    return data.structuredPatch as Hunk[]
  if (data.type === 'create' && typeof data.content === 'string')
    return [{ oldStart: 0, newStart: 1, lines: data.content.replace(/\n$/, '').split('\n').map((l) => `+${l}`) }]
  return []
}

export function counts(call: ToolCall) {
  let add = 0
  let del = 0
  for (const hunk of hunks(call))
    for (const line of hunk.lines) {
      if (line.startsWith('+')) add++
      else if (line.startsWith('-')) del++
    }
  return { add, del }
}

export type FileChanges = { path: string; baseline: string | null; calls: ToolCall[]; add: number; del: number }

export function changedFiles(calls: ToolCall[]): FileChanges[] {
  const files = new Map<string, FileChanges>()
  for (const call of calls) {
    if (!EDITS.includes(call.name) || !call.result || call.result.error) continue
    const data = call.result.data ?? {}
    const path = String(data.filePath ?? call.input.file_path ?? call.input.notebook_path ?? '')
    if (!path) continue
    let file = files.get(path)
    if (!file) {
      const original = data.type === 'create' ? null : (data.originalFile ?? null)
      file = { path, baseline: typeof original === 'string' ? original : null, calls: [], add: 0, del: 0 }
      files.set(path, file)
    }
    const c = counts(call)
    file.calls.push(call)
    file.add += c.add
    file.del += c.del
  }
  return [...files.values()]
}

export function lineMarks(base: string, now: string) {
  const added = new Set<number>()
  const removed = new Set<number>()
  let line = 1
  for (const part of diffLines(base, now)) {
    if (part.removed) removed.add(line)
    else {
      if (part.added) for (let i = 0; i < part.count; i++) added.add(line + i)
      line += part.count
    }
  }
  return { added, removed }
}
