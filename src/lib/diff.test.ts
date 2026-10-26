import { describe, expect, it } from 'vitest'
import { changedFiles, counts, everything, hidden, lineMarks, pairs, reveal, rows, type Cell } from './diff'
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

  it('unfolds the lines around hunks and the whole file', () => {
    const file = Array.from({ length: 30 }, (_, i) => `l${i + 1}`)
    const hunks = [
      { oldStart: 5, newStart: 5, lines: [' l5', '-x', '+l6', ' l7'] },
      { oldStart: 20, newStart: 20, lines: [' l20', '+l21', ' l22'] },
    ]
    expect(hidden(hunks, file.length)).toEqual([4, 12, 8])
    const up = reveal(hunks, file, 1, 'up', 5)
    expect(up[1]).toMatchObject({ oldStart: 15, newStart: 15 })
    expect(up[1].lines.slice(0, 5)).toEqual([' l15', ' l16', ' l17', ' l18', ' l19'])
    const down = reveal(hunks, file, 1, 'down', 5)
    expect(down[0].lines.slice(-5)).toEqual([' l8', ' l9', ' l10', ' l11', ' l12'])
    expect(hidden(down, file.length)).toEqual([4, 7, 8])
    const joined = reveal(hunks, file, 1, 'up')
    expect(joined).toHaveLength(1)
    expect(hidden(joined, file.length)).toEqual([4, 8])
    const all = everything(hunks, file)
    expect(all).toHaveLength(1)
    expect(hidden(all, file.length)).toEqual([0, 0])
    expect(rows(all).filter((r) => r.kind === 'same' || r.kind === 'add').map((r) => r.text)).toEqual(file)
    expect(rows(all).find((r) => r.kind === 'same' && r.text === 'l30')).toMatchObject({ old: 29, new: 30 })
    expect(hunks[0].lines).toHaveLength(4)
  })
})
