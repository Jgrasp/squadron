import { describe, expect, it } from 'vitest'
import type { Card } from './board'
import { codeOf, leading, nameOf, taskAt } from './taskref'

const card = (title: string): Card => ({ id: title, key: '', title, prompt: '', column: 'todo', agent: null })

describe('taskref', () => {
  it('reads the code at the head of a card title', () => {
    expect(codeOf('T7B — Admin : options produit TMC')).toBe('T7B')
    expect(codeOf('T42-02 - Faille Alma')).toBe('T42-02')
    expect(codeOf('API — sans chiffre')).toBeUndefined()
    expect(nameOf('T7B — Admin : options produit TMC')).toBe('Admin : options produit TMC')
  })

  it('finds the card a list item starts with', () => {
    const cards = [card('T7B — Admin : options produit TMC'), card('T7 — Autre')]
    expect(taskAt(cards, 'T7B (admin des options), débloquée')?.title).toBe('T7B — Admin : options produit TMC')
    expect(taskAt(cards, 'T7Bis')).toBeUndefined()
    expect(taskAt(cards, 'Une petite tâche pour T7B')).toBeUndefined()
  })

  it('strips the code only when a separator follows it', () => {
    expect('T7B — corrige'.replace(leading('T7B'), '')).toBe('corrige')
    expect(leading('T7B').test('T7B (admin)')).toBe(false)
  })
})
