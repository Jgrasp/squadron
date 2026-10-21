import { describe, expect, it } from 'vitest'
import { changedFiles, counts, lineMarks, pairs, rows, type Cell } from './diff'
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

  it('lays out hunks unified and side by side', () => {
    const hunks = [{ oldStart: 1, newStart: 1, lines: [' a', '-b', '-c', '+B', ' d', '+e', '\\ No newline at end of file'] }]
    expect(rows(hunks).map((r) => (r.kind === 'head' ? r.text : `${r.kind}:${r.old ?? ''}:${r.new ?? ''}:${r.text}`))).toEqual([
      '@@ -1,4 +1,4 @@',
      'same:1:1:a',
      'del:2::b',
      'del:3::c',
      'add::2:B',
      'same:4:3:d',
      'add::4:e',
    ])
    const cell = (c: Cell | null) => (c ? `${c.kind}${c.number}${c.text}` : '-')
    expect(pairs(hunks).map((p) => (p.kind === 'head' ? 'head' : `${cell(p.left)}|${cell(p.right)}`))).toEqual([
      'head',
      'same1a|same1a',
      'del2b|add2B',
      'del3c|-',
      'same4d|same3d',
      '-|add4e',
    ])
  })

  it('marks added lines and where lines were removed', () => {
    const marks = lineMarks('a\nb\nc\nd\n', 'a\nB\nc\nnew\n')
    expect([...marks.added]).toEqual([2, 4])
    expect([...marks.removed]).toEqual([2, 4])
    const gone = lineMarks('a\nb\n', 'a\n')
    expect([...gone.added]).toEqual([])
    expect([...gone.removed]).toEqual([2])
  })
})
