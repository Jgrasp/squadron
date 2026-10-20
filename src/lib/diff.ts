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
