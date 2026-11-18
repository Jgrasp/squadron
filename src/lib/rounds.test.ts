import { describe, expect, it } from 'vitest'
import { byHead } from './rounds'
import type { Comment } from './stream'

const draft = (note: string, head?: string): Comment => ({ path: 'a.ts', line: 1, removed: false, code: '', note, head })

describe('byHead', () => {
  it('posts each round on the commit it was written on, oldest round first', () => {
    const groups = byHead([draft('r1 a', 'aaa'), draft('r2 a', 'bbb'), draft('r1 b', 'aaa'), draft('old')], 'bbb')
    expect(groups.map((g) => [g.at, g.items.map((c) => c.note)])).toEqual([
      ['aaa', ['r1 a', 'r1 b']],
      ['bbb', ['r2 a', 'old']],
    ])
  })

  it('still posts once without drafts, for a verdict alone', () => {
    expect(byHead([], 'ccc')).toEqual([{ at: 'ccc', items: [] }])
  })
})
