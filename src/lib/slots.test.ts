import { describe, expect, it } from 'vitest'
import { arrange, moved } from './slots'

describe('slots', () => {
  it('keeps the saved order, drops unknown items and appends new ones', () => {
    expect(arrange(['a', 'b', 'c'])).toEqual(['a', 'b', 'c'])
    expect(arrange(['a', 'b', 'c'], ['c', 'gone', 'a', 'c'])).toEqual(['c', 'a', 'b'])
  })

  it('moves an item to an index among the others', () => {
    expect(moved(['a', 'b', 'c', 'd'], 'a', 2)).toEqual(['b', 'c', 'a', 'd'])
    expect(moved(['a', 'b', 'c'], 'c', 0)).toEqual(['c', 'a', 'b'])
    expect(moved(['a', 'b', 'c'], 'b', 9)).toEqual(['a', 'c', 'b'])
  })
})
