import { changedFiles } from './diff'
import { calls, type Agent, type Options } from './stream'

export type Column = 'todo' | 'progress' | 'review' | 'done'

export type Card = { id: string; key: string; title: string; prompt: string; goal?: string; column: Column; agent: string | null; auto?: Column | null }

export type Mirror = Pick<Agent, 'id' | 'project' | 'name' | 'status' | 'attention' | 'goal'> & { detail: string; done: number; total: number; add: number; del: number }

export type Actions = {
  cards(path: string, cards: Card[]): void
  start(path: string, card: string, name: string, options: Options, text: string, goal: string, worktree: string | null): void
  show(agent: string): void
  diff(agent: string): void
}

export const COLUMNS: [Column, string][] = [
  ['todo', 'To do'],
  ['progress', 'In progress'],
  ['review', 'To review'],
  ['done', 'Done'],
]

export function mirror(a: Agent, detail: string): Mirror {
  const files = changedFiles(calls(a))
  return {
    id: a.id,
    project: a.project,
    name: a.name,
    status: a.status,
    attention: a.attention,
    goal: a.goal,
    detail,
    done: a.tasks.filter((t) => t.status === 'completed').length,
    total: a.tasks.length,
    add: files.reduce((n, f) => n + f.add, 0),
    del: files.reduce((n, f) => n + f.del, 0),
  }
}

export const stage = (agent: Mirror): Column => (agent.status === 'done' ? 'review' : 'progress')

export function follow(card: Card, agent?: Mirror): Card {
  if (card.column === 'done' || !agent) return card
  const now = stage(agent)
  return card.auto === now ? card : { ...card, column: now, auto: now }
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
