import type { Entry, ToolCall } from './stream'

type Json = Record<string, any>

const LIMIT = 4000

const at = (line: Json) => {
  const time = Date.parse(line.timestamp ?? '')
  return Number.isNaN(time) ? Date.now() : time
}

const textOf = (content: unknown): string =>
  typeof content === 'string'
    ? content
    : Array.isArray(content)
      ? content
          .filter((part) => part?.type === 'text')
          .map((part) => String(part.text ?? ''))
          .join('\n')
      : ''

const technical = /^<(command-|local-command|task-notification|system-reminder|bash-|user-prompt-submit)/

const clip = (text: string) => (text.length > LIMIT ? `${text.slice(0, LIMIT)}…` : text)

export function entriesOf(transcript: string): Entry[] {
  const entries: Entry[] = []
  const calls = new Map<string, ToolCall>()
  let reply: Extract<Entry, { kind: 'agent' }> | null = null
  for (const raw of transcript.split('\n')) {
    let line: Json
    try {
      line = JSON.parse(raw)
    } catch {
      continue
    }
    if (line.isSidechain || line.isMeta) continue
    if (line.isCompactSummary) {
      entries.push({ kind: 'compacted', at: at(line), before: 0, after: 0 })
      reply = null
      continue
    }
    if (line.type === 'user') {
      const content = line.message?.content
      const parts: Json[] = Array.isArray(content) ? content : []
      for (const part of parts) {
        const call = part.type === 'tool_result' ? calls.get(part.tool_use_id) : undefined
        if (call) call.result = { text: clip(textOf(part.content)), error: !!part.is_error }
      }
      const said = textOf(content).trim()
      if (said && !line.isSynthetic && !technical.test(said) && !parts.some((p) => p.type === 'tool_result')) {
        entries.push({ kind: 'user', text: said, at: at(line) })
        reply = null
      }
    } else if (line.type === 'assistant' && line.message?.model !== '<synthetic>') {
      if (!reply) {
        reply = { kind: 'agent', blocks: [] }
        entries.push(reply)
      }
      reply.uuid = line.uuid
      for (const part of line.message?.content ?? []) {
        if (part.type === 'thinking') reply.blocks.push({ type: 'thinking', ms: 0, ...(part.thinking ? { text: part.thinking } : {}) })
        else if (part.type === 'text' && String(part.text ?? '').trim())
          reply.blocks.push({ type: 'text', text: part.text, key: `${line.message.id}:${line.uuid}` })
        else if (part.type === 'tool_use') {
          const call: ToolCall = { id: part.id, name: part.name, input: part.input ?? {} }
          calls.set(part.id, call)
          const last = reply.blocks.at(-1)
          if (last?.type === 'tools') last.calls.push(call)
          else reply.blocks.push({ type: 'tools', calls: [call] })
        }
      }
    }
  }
  return entries
}
