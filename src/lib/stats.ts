export type Hour = {
  input: number
  cache_write: number
  cache_read: number
  output: number
  advisor: number
  turns: number
  sources: Record<string, number>
  projects: Record<string, number>
  models: Record<string, number>
  contexts: number[]
  calls: Record<string, number>
}

export type Session = { id: string; project: string; title: string | null; tokens: number; turns: number; peak: number; start: number; end: number }

export type Big = { label: string; source: string; tokens: number; rereads: number; project: string; at: number }

export type Usage = { hours: [number, Hour][]; sessions: Session[]; bigs: Big[] }

export const GROUPS = ['tools', 'replies', 'setup', 'claude', 'hooks', 'mcp', 'plugins', 'messages', 'squadron', 'advisor', 'none'] as const

export type Group = (typeof GROUPS)[number]

export type Fixed = 'startup' | 'listing' | 'compaction' | 'setup' | 'messages' | 'kept' | 'generated' | 'advisor' | 'jumps'

export type Item = { name: string } | { fixed: Fixed }

const FIXED: Record<string, [Group, Fixed]> = {
  'Claude Code · system prompt, tools, MCP schemas': ['claude', 'startup'],
  'Claude Code · skill listing': ['claude', 'listing'],
  'Compaction summary': ['claude', 'compaction'],
  'Your setup (CLAUDE.md, memory, env)': ['setup', 'setup'],
  'Your messages': ['messages', 'messages'],
  Replies: ['replies', 'kept'],
  'Replies · generated': ['replies', 'generated'],
  Advisor: ['advisor', 'advisor'],
  'Not attributed': ['none', 'jumps'],
}

export function groupOf(source: string): [Group, Item] {
  const fixed = FIXED[source]
  if (fixed) return [fixed[0], { fixed: fixed[1] }]
  const [kind, ...rest] = source.split(' · ')
  const name = rest.join(' · ')
  if (kind === 'MCP') return [name.startsWith('squadron') ? 'squadron' : 'mcp', { name }]
  if (kind === 'Plugin') return ['plugins', { name }]
  if (kind === 'Tool') return ['tools', { name }]
  if (kind === 'Subagent') return ['tools', { name: source }]
  if (kind === 'Skill') return ['setup', { name: source }]
  if (kind === 'Hooks' || kind === 'Reminders') return ['hooks', { name }]
  return ['none', { name: source }]
}

export const itemKey = (item: Item) => ('fixed' in item ? `:${item.fixed}` : item.name)

const pad = (n: number) => String(n).padStart(2, '0')

export const dayOf = (at: Date) => `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}`

export const hourDate = (key: number) => new Date(key * 3_600_000)

export const total = (h: Pick<Hour, 'input' | 'cache_write' | 'cache_read' | 'output' | 'advisor'>) => h.input + h.cache_write + h.cache_read + h.output + h.advisor

export type Totals = {
  tokens: number
  cacheRead: number
  output: number
  turns: number
  days: Set<string>
  sources: Map<Group, Map<string, { item: Item; tokens: number }>>
  projects: Record<string, number>
  models: Record<string, number>
  contexts: number[]
  hours: number[]
  calls: Record<string, number>
}

const add = (into: Record<string, number>, from: Record<string, number>) => {
  for (const [k, v] of Object.entries(from)) into[k] = (into[k] ?? 0) + v
}

export function sum(hours: [number, Hour][], keep: (day: string) => boolean): Totals {
  const t: Totals = { tokens: 0, cacheRead: 0, output: 0, turns: 0, days: new Set(), sources: new Map(), projects: {}, models: {}, contexts: [0, 0, 0, 0, 0], hours: Array(24).fill(0), calls: {} }
  for (const [key, h] of hours) {
    const at = hourDate(key)
    const day = dayOf(at)
    if (!keep(day)) continue
    const n = total(h)
    t.tokens += n
    t.cacheRead += h.cache_read
    t.output += h.output
    t.turns += h.turns
    t.days.add(day)
    t.hours[at.getHours()] += n
    h.contexts.forEach((v, i) => (t.contexts[i] += v))
    add(t.projects, h.projects)
    add(t.models, h.models)
    add(t.calls, h.calls)
    for (const [source, v] of Object.entries(h.sources)) {
      const [group, item] = groupOf(source)
      let items = t.sources.get(group)
      if (!items) t.sources.set(group, (items = new Map()))
      const k = itemKey(item)
      const cur = items.get(k)
      if (cur) cur.tokens += v
      else items.set(k, { item, tokens: v })
    }
  }
  return t
}

export function daily(hours: [number, Hour][]) {
  const days: Record<string, { tokens: number; turns: number }> = {}
  for (const [key, h] of hours) {
    const d = (days[dayOf(hourDate(key))] ??= { tokens: 0, turns: 0 })
    d.tokens += total(h)
    d.turns += h.turns
  }
  return days
}

export function lastDays(count: number, today = new Date()) {
  return Array.from({ length: count }, (_, i) => dayOf(new Date(today.getFullYear(), today.getMonth(), today.getDate() - count + 1 + i)))
}

export const groupTotal = (t: Totals, g: Group) => [...(t.sources.get(g)?.values() ?? [])].reduce((s, x) => s + x.tokens, 0)

export const ranked = (t: Totals, g: Group) => [...(t.sources.get(g)?.values() ?? [])].filter((x) => x.tokens > 0).sort((a, b) => b.tokens - a.tokens)

type Plugin = { id: string; name: string | null; enabled: boolean }

const nameOf = (p: Plugin) => p.name ?? p.id.split('@')[0]

export const uncalledPlugins = (t: Totals, installed: Plugin[]) =>
  ranked(t, 'plugins').flatMap((x) => {
    const plugin = installed.find((p) => p.enabled && 'name' in x.item && nameOf(p) === x.item.name)
    return plugin && !t.calls[`Plugin · ${nameOf(plugin)}`] ? [{ ...x, plugin }] : []
  })

export const unusedPlugins = (t: Totals, installed: Plugin[], hooks: (id: string) => number | undefined) =>
  uncalledPlugins(t, installed).filter((x) => hooks(x.plugin.id) === 0)
