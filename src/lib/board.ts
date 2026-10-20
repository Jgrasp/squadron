import type { Agent, Options } from './stream'

export type Column = 'todo' | 'progress' | 'review' | 'done'

export type Card = { id: string; key: string; title: string; prompt: string; column: Column; agent: string | null }

export type Mirror = Pick<Agent, 'id' | 'project' | 'name' | 'status' | 'attention'> & { detail: string; done: number; total: number }

export type Actions = {
  cards(path: string, cards: Card[]): void
  start(path: string, card: string, name: string, options: Options, text: string): void
  show(agent: string): void
}

export const COLUMNS: [Column, string][] = [
  ['todo', 'To do'],
  ['progress', 'In progress'],
  ['review', 'To review'],
  ['done', 'Done'],
]

export const mirror = (a: Agent, detail: string): Mirror => ({
  id: a.id,
  project: a.project,
  name: a.name,
  status: a.status,
  attention: a.attention,
  detail,
  done: a.tasks.filter((t) => t.status === 'completed').length,
  total: a.tasks.length,
})

export function place(card: Card, agent?: Mirror): Column {
  if (card.column === 'done' || !agent) return card.column
  return agent.status === 'done' ? 'review' : 'progress'
}

export function moveCard(cards: Card[], id: string, column: Column, before: string | null = null) {
  const card = cards.find((c) => c.id === id)
  if (!card) return cards
  if (before === id) return cards.map((c) => (c.id === id ? { ...c, column } : c))
  const rest = cards.filter((c) => c.id !== id)
  const at = rest.findIndex((c) => c.id === before)
  rest.splice(at < 0 ? rest.length : at, 0, { ...card, column })
  return rest
}
