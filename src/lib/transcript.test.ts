import { describe, expect, it } from 'vitest'
import { entriesOf } from './transcript'

const line = (value: object) => JSON.stringify(value)

describe('entriesOf', () => {
  it('rebuilds the conversation of a Claude Code transcript', () => {
    const transcript = [
      line({ type: 'user', timestamp: '2026-10-22T10:00:00Z', message: { content: '<command-name>/clear</command-name>' } }),
      line({ type: 'user', timestamp: '2026-10-22T10:00:01Z', message: { content: 'Fix the export' } }),
      line({ type: 'assistant', uuid: 'u1', message: { id: 'm1', model: 'claude-opus', content: [{ type: 'thinking', thinking: 'Look first' }, { type: 'text', text: 'Reading it.' }] } }),
      line({ type: 'assistant', uuid: 'u2', message: { id: 'm2', model: 'claude-opus', content: [{ type: 'tool_use', id: 't1', name: 'Read', input: { file_path: 'a.php' } }] } }),
      line({ type: 'user', message: { content: [{ type: 'tool_result', tool_use_id: 't1', content: 'x'.repeat(5000) }] } }),
      line({ type: 'assistant', uuid: 'u3', isSidechain: true, message: { id: 'm3', model: 'claude-opus', content: [{ type: 'text', text: 'subagent' }] } }),
      line({ type: 'user', isCompactSummary: true, message: { content: 'Summary' } }),
      line({ type: 'user', message: { content: [{ type: 'text', text: 'Thanks' }] } }),
      line({ type: 'user', message: { content: '<pasted_content id="a">\nLog line\n</pasted_content>\nWhat is wrong?' } }),
      'not json',
    ].join('\n')
    const entries = entriesOf(transcript)
    expect(entries.map((e) => e.kind)).toEqual(['user', 'agent', 'compacted', 'user', 'user'])
    const reply = entries[1] as Extract<(typeof entries)[number], { kind: 'agent' }>
    expect(reply.uuid).toBe('u2')
    expect(reply.blocks.map((b) => b.type)).toEqual(['thinking', 'text', 'tools'])
    const tools = reply.blocks[2] as Extract<(typeof reply.blocks)[number], { type: 'tools' }>
    expect(tools.calls[0].result?.text.length).toBe(4001)
    expect(entries[0]).toMatchObject({ kind: 'user', text: 'Fix the export', at: Date.parse('2026-10-22T10:00:01Z') })
  })
})
