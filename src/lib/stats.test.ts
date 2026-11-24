import { describe, expect, it } from 'vitest'
import { groupOf, groupTotal, lastDays, ranked, sum, unusedPlugins, type Hour } from './stats'

const hour = (sources: Record<string, number>, extra: Partial<Hour> = {}): Hour => ({
  input: 0,
  cache_write: 0,
  cache_read: Object.values(sources).reduce((s, v) => s + v, 0),
  output: 0,
  advisor: 0,
  turns: 1,
  sources,
  projects: { app: 1 },
  models: { m: 1 },
  contexts: [1, 0, 0, 0, 0],
  calls: {},
  ...extra,
})

const at = (y: number, m: number, d: number, h: number) => new Date(y, m - 1, d, h).getTime() / 3_600_000

describe('stats', () => {
  it('groups sources like the context panel', () => {
    expect(groupOf('Tool · Bash')).toEqual(['tools', { name: 'Bash' }])
    expect(groupOf('MCP · squadron-pilot')).toEqual(['squadron', { name: 'squadron-pilot' }])
    expect(groupOf('MCP · chrome-devtools')).toEqual(['mcp', { name: 'chrome-devtools' }])
    expect(groupOf('Hooks · SessionStart')).toEqual(['hooks', { name: 'SessionStart' }])
    expect(groupOf('Replies · generated')).toEqual(['replies', { fixed: 'generated' }])
    expect(groupOf('Not attributed')).toEqual(['none', { fixed: 'jumps' }])
  })

  it('sums the kept local days only', () => {
    const hours: [number, Hour][] = [
      [at(2026, 11, 23, 23), hour({ 'Tool · Bash': 100, Replies: 50 })],
      [at(2026, 11, 24, 9), hour({ 'Tool · Bash': 10, 'Plugin · crew': 5 })],
    ]
    const t = sum(hours, (d) => d === '2026-11-24')
    expect(t.tokens).toBe(15)
    expect(t.hours[9]).toBe(15)
    expect(groupTotal(t, 'tools')).toBe(10)
    expect(ranked(t, 'plugins').map((x) => x.tokens)).toEqual([5])
    expect(sum(hours, () => true).days).toEqual(new Set(['2026-11-23', '2026-11-24']))
  })

  it('lists the last days up to today', () => {
    expect(lastDays(3, new Date(2026, 2, 1))).toEqual(['2026-02-27', '2026-02-28', '2026-03-01'])
  })

  it('offers to disable only enabled, never called plugins without hooks', () => {
    const t = sum([[at(2026, 11, 24, 9), hour({ 'Plugin · idle': 30, 'Plugin · used': 20, 'Plugin · hooked': 10, 'Plugin · synced': 5 }, { calls: { 'Plugin · used': 2 } })]], () => true)
    const installed = [
      { id: 'idle@m', name: 'idle', enabled: true },
      { id: 'used@m', name: 'used', enabled: true },
      { id: 'hooked@m', name: 'hooked', enabled: true },
    ]
    const hooks = (id: string) => (id === 'hooked@m' ? 2 : 0)
    expect(unusedPlugins(t, installed, hooks).map((x) => x.plugin.id)).toEqual(['idle@m'])
  })
})
