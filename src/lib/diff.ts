import type { ToolCall } from './stream'

export const EDITS = ['Edit', 'MultiEdit', 'Write', 'NotebookEdit']

type Hunk = { oldStart: number; newStart: number; lines: string[] }

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
