import type { Source, Summary } from './tickets.svelte'

export type Facet = 'status' | 'kind' | 'priority' | 'epic'
export type Filters = { source: string; mine: boolean; people: string[]; free: boolean; query: string; hidden: Record<Facet, string[]> }

export const FACETS: Facet[] = ['status', 'kind', 'priority', 'epic']
export const emptyFilters = (): Filters => ({ source: '', mine: false, people: [], free: false, query: '', hidden: { status: [], kind: [], priority: [], epic: [] } })

export const sourceId = (s: Source) => `${s.connection}:${s.key}`
export const inSource = (t: Summary, s: Source) => t.connection === s.connection && t.key.startsWith(`${s.key}-`)

const ORDER = ['new', 'indeterminate', 'done']
const rank = (category: string) => (ORDER.includes(category) ? ORDER.indexOf(category) : 1)

export function filterTickets(list: Summary[], f: Filters, sources: Source[], linked: (t: Summary) => boolean) {
  const source = sources.find((s) => sourceId(s) === f.source)
  const q = f.query.trim().toLowerCase()
  return list.filter(
    (t) =>
      (!source || inSource(t, source)) &&
      (!f.mine || t.mine) &&
      (!f.people.length || f.people.includes(t.assignee?.name ?? '')) &&
      (!f.free || !linked(t)) &&
      FACETS.every((facet) => !f.hidden[facet].includes(t[facet])) &&
      (!q || `${t.key} ${t.title}`.toLowerCase().includes(q)),
  )
}

export function statusColumns(list: Summary[]) {
  const columns: { status: string; category: string; tickets: Summary[] }[] = []
  for (const t of list) {
    const column = columns.find((c) => c.status === t.status)
    if (column) column.tickets.push(t)
    else columns.push({ status: t.status, category: t.category, tickets: [t] })
  }
  return columns.sort((a, b) => rank(a.category) - rank(b.category))
}

export const values = (list: Summary[], facet: Facet) => [...new Set(list.map((t) => t[facet]))]

export function kindTone(kind: string) {
  if (/bug|anomal/i.test(kind)) return 'bug'
  if (/stor|récit|recit/i.test(kind)) return 'story'
  if (/epic|épopée|epopee/i.test(kind)) return 'epic'
  return 'task'
}

export const URGENCY = ['', '▼', '═', '▲']
