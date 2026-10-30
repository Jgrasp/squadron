import { describe, expect, it } from 'vitest'
import { emptyFilters, filterTickets, kindTone, statusColumns } from './jira'
import type { Summary } from './tickets.svelte'

const ticket = (key: string, status: string, category: string, extra: Partial<Summary> = {}): Summary => ({
  connection: 'akki',
  key,
  title: `Title ${key}`,
  status,
  category,
  kind: 'Story',
  priority: 'High',
  urgency: 3,
  epic: '',
  assignee: null,
  mine: false,
  ...extra,
})

const list = [
  ticket('RING-3', 'Done', 'done'),
  ticket('RING-1', 'Review', 'indeterminate', { mine: true, assignee: { name: 'Quentin', avatar: '' } }),
  ticket('RING-2', 'To do', 'new', { kind: 'Bug' }),
  ticket('TMA-4', 'In progress', 'indeterminate', { assignee: { name: 'Marie', avatar: '' } }),
]
const sources = [
  { connection: 'akki', key: 'RING', name: '' },
  { connection: 'akki', key: 'TMA', name: '' },
]

describe('jira', () => {
  it('orders the status columns by category, then by first ticket', () => {
    expect(statusColumns(list).map((c) => c.status)).toEqual(['To do', 'Review', 'In progress', 'Done'])
  })

  it('filters by source, person, facet, task and text', () => {
    const keys = (f: Partial<ReturnType<typeof emptyFilters>>, linked = (_: Summary) => false) =>
      filterTickets(list, { ...emptyFilters(), ...f }, sources, linked).map((t) => t.key)
    expect(keys({ source: 'akki:TMA' })).toEqual(['TMA-4'])
    expect(keys({ mine: true })).toEqual(['RING-1'])
    expect(keys({ people: ['Marie', ''] })).toEqual(['RING-3', 'RING-2', 'TMA-4'])
    expect(keys({ hidden: { status: ['Done'], kind: ['Bug'], priority: [], epic: [] } })).toEqual(['RING-1', 'TMA-4'])
    expect(keys({ free: true }, (t) => t.key === 'RING-1')).toEqual(['RING-3', 'RING-2', 'TMA-4'])
    expect(keys({ query: 'ring-2' })).toEqual(['RING-2'])
  })

  it('tones the issue types', () => {
    expect([kindTone('Bug'), kindTone('Récit'), kindTone('Epic'), kindTone('Sous-tâche')]).toEqual(['bug', 'story', 'epic', 'task'])
  })
})
