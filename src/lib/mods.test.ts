import { describe, expect, it } from 'vitest'
import { grantsOf, itemsOf, load, themeOf, toolsOf, validate } from './mods'

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
    tools: [{ name: 'logs', description: 'Last lines of the Lando logs', command: 'lando logs --tail 200' }],
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

  it('checks the tools offered to agents', () => {
    const tool = (extra: object) => keys({ ...lando, contributes: { tools: [{ name: 'logs', description: 'Logs', command: 'lando logs', ...extra }] } })
    expect(tool({})).toEqual([])
    expect(tool({ name: 'Logs!' })).toEqual(['mods.badToolName'])
    expect(tool({ description: ' ' })).toEqual(['mods.noToolDescription'])
    expect(tool({ command: 'a\nb' })).toEqual(['mods.badToolCommand'])
  })

  it('runs commands and offers tools only once the user allowed them', () => {
    const found = [{ dir: '/m/lando', linked: false, manifest: lando, error: null }]
    const waiting = load(found, [], { lando: ['lando logs -f'] })
    expect(waiting[0].pending).toEqual(['lando logs --tail 200'])
    expect(itemsOf(waiting, 'agent.tools')[0].allowed).toBe(true)
    expect(toolsOf(waiting)).toEqual([])
    const allowed = load(found, [], { lando: ['lando logs -f', 'lando logs --tail 200'] })
    expect(toolsOf(allowed).map((t) => t.name)).toEqual(['lando_logs'])
    expect(itemsOf(load(found, [], {}), 'agent.tools')[0].allowed).toBe(false)
  })

  it('accepts logic in a wasm module and asks for the programs it runs', () => {
    const hello = {
      id: 'hello',
      version: '1.0.0',
      api: 1,
      name: { en: 'Hello' },
      wasm: 'target/wasm32-wasip2/release/hello.wasm',
      contributes: {
        items: [{ slot: 'agent.tools', id: 'count', label: { en: 'Count' }, action: { mod: 'count' } }],
        tools: [{ name: 'changed_files', description: 'Changed files', input: { type: 'object', properties: {} } }],
      },
      permissions: { exec: ['git'] },
    }
    expect(keys(hello)).toEqual([])
    expect(keys({ ...hello, wasm: undefined })).toEqual(['mods.needsWasm', 'mods.toolNeedsWasm'])
    expect(keys({ ...hello, wasm: '../other/x.wasm' })).toEqual(['mods.badWasm'])
    expect(keys({ ...hello, permissions: { exec: ['/bin/rm'] } })).toEqual(['mods.badExec'])
    expect(keys({ ...hello, contributes: { tools: [{ name: 'x', description: 'X', input: { type: 'string' } }] } })).toEqual(['mods.badToolInput'])
    const found = [{ dir: '/m/hello', linked: false, manifest: hello, error: null }]
    const waiting = load(found, [], {})[0]
    expect(waiting.pending).toEqual(['exec:git'])
    expect(grantsOf(waiting)).toEqual([])
    expect(toolsOf([waiting]).map((t) => [t.name, t.tool])).toEqual([['hello_changed_files', 'changed_files']])
    expect(grantsOf(load(found, [], { hello: ['exec:git'] })[0])).toEqual(['exec:git'])
  })

  it('asks for network hosts and checks secrets', () => {
    const base = { id: 'gh', version: '1.0.0', api: 1, name: { en: 'GH' } }
    const net = { ...base, permissions: { network: ['api.github.com'] }, contributes: { secrets: [{ id: 'token', label: { en: 'Token' } }] } }
    expect(keys(net)).toEqual([])
    for (const network of [['https://api.github.com'], ['api.github.com/x'], ['localhost'], 'api.github.com'])
      expect(keys({ ...base, permissions: { network } })).toEqual(['mods.badNetwork'])
    expect(keys({ ...base, contributes: { secrets: [{ id: 'Token', label: { en: 'T' } }] } })).toEqual(['mods.badSecrets'])
    expect(keys({ ...base, contributes: { secrets: [{ id: 't', label: { en: 'T' } }, { id: 't', label: { en: 'U' } }] } })).toEqual(['mods.badSecrets'])
    const found = [{ dir: '/m/gh', linked: false, manifest: net, error: null }]
    expect(load(found, [], {})[0].pending).toEqual(['net:api.github.com'])
    expect(grantsOf(load(found, [], { gh: ['net:api.github.com'] })[0])).toEqual(['net:api.github.com'])
  })
})
