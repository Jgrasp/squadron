import { describe, expect, it } from 'vitest'
import { itemsOf, load, themeOf, validate } from './mods'

const lando = {
  id: 'lando',
  version: '1.0.0',
  api: 1,
  name: { en: 'Lando' },
  contributes: {
    items: [
      { slot: 'agent.tools', id: 'logs', label: { en: 'Logs', fr: 'Journaux' }, icon: 'terminal', action: { 'terminal.run': 'lando logs -f' } },
      { slot: 'menu.squadron', id: 'docs', label: { en: 'Lando docs' }, action: { 'url.open': 'https://docs.lando.dev' } },
    ],
    theme: { accent: '#4fa3e0' },
  },
}

const keys = (raw: unknown, taken?: string[]) => validate(raw, taken).problems.map(([key]) => key)

describe('mods', () => {
  it('accepts a complete manifest', () => {
    expect(validate(lando)).toEqual({ manifest: lando, problems: [] })
  })

  it('names every problem of a broken manifest', () => {
    expect(keys('x')).toEqual(['mods.notObject'])
    expect(keys({ id: 'Bad Id', api: 2, name: 'Lando' })).toEqual(['mods.badId', 'mods.badApi', 'mods.noVersion', 'mods.noName'])
    expect(keys(lando, ['lando'])).toEqual(['mods.takenId'])
  })

  it('checks each item against its slot', () => {
    const item = (extra: object) => keys({ ...lando, contributes: { items: [{ slot: 'agent.tools', id: 'a', label: { en: 'A' }, action: { squadron: 'kanban' }, ...extra }] } })
    expect(item({})).toEqual([])
    expect(item({ slot: 'agent.info' })).toEqual(['mods.badSlot'])
    expect(item({ icon: 'rocket' })).toEqual(['mods.badIcon'])
    expect(item({ action: { squadron: 'explode' } })).toEqual(['mods.badSquadron'])
    expect(item({ action: { 'terminal.run': 'ls\nrm -rf ~' } })).toEqual(['mods.badCommand'])
    expect(item({ action: { 'url.open': 'javascript:alert(1)' } })).toEqual(['mods.badUrl'])
    expect(item({ action: { 'url.open': 'https://a.b', squadron: 'kanban' } })).toEqual(['mods.badAction'])
    expect(item({ slot: 'menu.squadron', action: { 'prompt.insert': 'Review' } })).toEqual(['mods.needsAgent'])
    expect(keys({ ...lando, contributes: { items: [lando.contributes.items[0], lando.contributes.items[0]] } })).toEqual(['mods.twiceItem'])
  })

  it('only lets colors into the theme', () => {
    expect(keys({ ...lando, contributes: { theme: { accent: 'red; } body { display: none' } } })).toEqual(['mods.badTheme'])
    expect(keys({ ...lando, contributes: { theme: { accent: 'rgb(10, 20, 30)' } } })).toEqual([])
  })

  it('keeps the first mod that claims an id and contributes only enabled valid mods', () => {
    const found = (dir: string, manifest: unknown, error: string | null = null) => ({ dir, linked: false, manifest, error })
    const mods = load([found('/m/lando', lando), found('/m/copy', lando), found('/m/broken', null, 'mod.json is missing.')], [])
    expect(mods.map((m) => [m.id, m.problems.map(([k]) => k)])).toEqual([
      ['lando', []],
      ['lando', ['mods.takenId']],
      ['broken', ['mods.unreadable']],
    ])
    expect(itemsOf(mods, 'agent.tools').map((i) => i.key)).toEqual(['lando.logs'])
    expect(themeOf(mods)).toEqual({ accent: '#4fa3e0' })
    expect(itemsOf(load([found('/m/lando', lando)], ['lando']), 'agent.tools')).toEqual([])
  })
})
