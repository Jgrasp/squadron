import { describe, expect, it } from 'vitest'
import { follow, moveCard, type Card, type Mirror } from './board'

const card = (id: string, column: Card['column'] = 'todo'): Card => ({ id, key: '', title: id, prompt: '', column, agent: null })
const agent = (status: Mirror['status']): Mirror => ({ id: 'a', project: '/p', name: 'a', status, attention: '', goal: null, detail: '', done: 0, total: 0, add: 0, del: 0 })

describe('board', () => {
  it('moves a card when its agent changes state, and keeps a manual move until then', () => {
    expect(follow(card('c', 'todo')).column).toBe('todo')
    const working = follow(card('c', 'todo'), agent('working'))
    expect(working).toMatchObject({ column: 'progress', auto: 'progress' })
    const moved = { ...working, column: 'todo' as const }
    expect(follow(moved, agent('waiting'))).toBe(moved)
    expect(follow(moved, agent('done'))).toMatchObject({ column: 'review', auto: 'review' })
    expect(follow(card('c', 'done'), agent('working')).column).toBe('done')
  })

  it('moves a card to a column, before another card or at the end', () => {
    const cards = [card('a'), card('b'), card('c', 'review')]
    expect(moveCard(cards, 'a', 'review').map((c) => [c.id, c.column])).toEqual([['b', 'todo'], ['c', 'review'], ['a', 'review']])
    expect(moveCard(cards, 'c', 'todo', 'a').map((c) => c.id)).toEqual(['c', 'a', 'b'])
    expect(moveCard(cards, 'b', 'done', 'b').map((c) => [c.id, c.column])).toEqual([['a', 'todo'], ['b', 'done'], ['c', 'review']])
    expect(moveCard(cards, 'x', 'done')).toBe(cards)
  })
})
