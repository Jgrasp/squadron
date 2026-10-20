import { describe, expect, it } from 'vitest'
import { changedFiles, counts } from './diff'
import type { ToolCall } from './stream'

const edit = (id: string, path: string, data: Record<string, unknown>): ToolCall => ({
  id,
  name: data.type === 'create' ? 'Write' : 'Edit',
  input: { file_path: path },
  result: { text: '', error: false, data: { filePath: path, ...data } },
})

describe('diff', () => {
  it('counts the lines of a creation and of a patch', () => {
    expect(counts(edit('1', '/p/a', { type: 'create', content: 'a\nb\n' }))).toEqual({ add: 2, del: 0 })
    expect(counts(edit('2', '/p/b', { structuredPatch: [{ oldStart: 1, newStart: 1, lines: [' x', '-y', '+z', '+w'] }] }))).toEqual({ add: 2, del: 1 })
  })

  it('keeps the first known content of each file as the baseline', () => {
    const files = changedFiles([
      edit('1', '/p/a', { originalFile: 'v1', structuredPatch: [{ oldStart: 1, newStart: 1, lines: ['-v1', '+v2'] }] }),
      edit('2', '/p/a', { originalFile: 'v2', structuredPatch: [{ oldStart: 1, newStart: 1, lines: ['-v2', '+v3'] }] }),
      edit('3', '/p/new', { type: 'create', content: 'n\n' }),
      { ...edit('4', '/p/failed', { originalFile: 'x' }), result: { text: 'boom', error: true } },
    ])
    expect(files.map((f) => [f.path, f.baseline, f.calls.length, f.add, f.del])).toEqual([
      ['/p/a', 'v1', 2, 2, 2],
      ['/p/new', null, 1, 1, 0],
    ])
  })
})
