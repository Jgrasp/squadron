import { describe, expect, it } from 'vitest'
import type { Card } from './board'
import { codeOf, leading, mentions, nameOf, taskAt, taskId } from './taskref'

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

  it('splits a text around the tasks it names', () => {
    const cards = [card('T61 — BMC layout'), card('T65 — TMC layout'), card('T5 — Autre')]
    const parts = mentions(cards, 'Prêtes : T61 et T65 (layout), T52, T5B.')
    expect(parts.map((p) => (typeof p === 'string' ? p : `[${p.code}]`)).join('')).toBe('Prêtes : [T61] et [T65] (layout), T52, T5B.')
    expect(mentions([], 'T61')).toEqual(['T61'])
  })

  it('reads the card id of a task link', () => {
    expect(taskId('squadron:task/abb24043-bd0a-4de0-b8a8-8a1247146687')).toBe('abb24043-bd0a-4de0-b8a8-8a1247146687')
  })
})
