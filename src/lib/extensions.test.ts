import { describe, expect, it } from 'vitest'
import { argsOf, catalogOf, grantsOf, itemsOf, load, needsAgent, newer, OFFICIAL, panelsOf, shownOf, spotlightOf, updatesOf, themeOf, toolsOf, validate } from './extensions'

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

describe('extensions', () => {
  it('accepts a complete manifest', () => {
    expect(validate(lando)).toEqual({ manifest: lando, problems: [] })
  })

  it('names every problem of a broken manifest', () => {
    expect(keys('x')).toEqual(['extensions.notObject'])
    expect(keys({ id: 'Bad Id', api: 2, name: 'Lando' })).toEqual(['extensions.badId', 'extensions.badApi', 'extensions.noVersion', 'extensions.noName'])
    expect(keys(lando, ['lando'])).toEqual(['extensions.takenId'])
  })

  it('checks each item against its slot', () => {
    const item = (extra: object) => keys({ ...lando, contributes: { items: [{ slot: 'agent.tools', id: 'a', label: { en: 'A' }, action: { squadron: 'kanban' }, ...extra }] } })
    expect(item({})).toEqual([])
    expect(item({ slot: 'agent.info' })).toEqual(['extensions.badSlot'])
    expect(item({ icon: 'rocket' })).toEqual(['extensions.badIcon'])
    expect(item({ action: { squadron: 'explode' } })).toEqual(['extensions.badSquadron'])
    expect(item({ action: { 'terminal.run': 'ls\nrm -rf ~' } })).toEqual(['extensions.badCommand'])
    expect(item({ action: { 'url.open': 'javascript:alert(1)' } })).toEqual(['extensions.badUrl'])
    expect(item({ action: { 'url.open': 'https://a.b', squadron: 'kanban' } })).toEqual(['extensions.badAction'])
    expect(item({ slot: 'menu.squadron', action: { 'prompt.insert': 'Review' } })).toEqual(['extensions.needsAgent'])
    expect(item({ words: ['logs', 'tail'], spotlight: false })).toEqual([])
    expect(item({ words: 'logs' })).toEqual(['extensions.badWords'])
    expect(item({ words: ['a\nb'] })).toEqual(['extensions.badWords'])
    expect(item({ spotlight: 'no' })).toEqual(['extensions.badSpotlight'])
    expect(keys({ ...lando, contributes: { items: [lando.contributes.items[0], lando.contributes.items[0]] } })).toEqual(['extensions.twiceItem'])
  })

  it('only lets colors into the theme', () => {
    expect(keys({ ...lando, contributes: { theme: { accent: 'red; } body { display: none' } } })).toEqual(['extensions.badTheme'])
    expect(keys({ ...lando, contributes: { theme: { accent: 'rgb(10, 20, 30)' } } })).toEqual([])
  })

  it('keeps the first extension that claims an id and contributes only enabled valid extensions', () => {
    const found = (dir: string, manifest: unknown, error: string | null = null) => ({ dir, linked: false, manifest, error })
    const extensions = load([found('/m/lando', lando), found('/m/copy', lando), found('/m/broken', null, 'extension.json is missing.')], [])
    expect(extensions.map((m) => [m.id, m.problems.map(([k]) => k)])).toEqual([
      ['lando', []],
      ['lando', ['extensions.takenId']],
      ['broken', ['extensions.unreadable']],
    ])
    expect(itemsOf(extensions, 'agent.tools').map((i) => i.key)).toEqual(['lando.logs'])
    expect(themeOf(extensions)).toEqual({ accent: '#4fa3e0' })
    expect(itemsOf(load([found('/m/lando', lando)], ['lando']), 'agent.tools')).toEqual([])
  })

  it('offers every item of every slot to the spotlight, unless it opts out', () => {
    const found = (dir: string, manifest: unknown) => ({ dir, linked: false, manifest, error: null })
    const hidden = { ...lando, contributes: { items: [...lando.contributes.items, { slot: 'agent.menu', id: 'quiet', label: { en: 'Quiet' }, action: { squadron: 'kanban' }, spotlight: false }] } }
    const items = spotlightOf(load([found('/m/lando', hidden)], []))
    expect(items.map((i) => i.key)).toEqual(['lando.logs', 'lando.docs'])
    expect(items.map(needsAgent)).toEqual([true, false])
  })

  it('checks the tools offered to agents', () => {
    const tool = (extra: object) => keys({ ...lando, contributes: { tools: [{ name: 'logs', description: 'Logs', command: 'lando logs', ...extra }] } })
    expect(tool({})).toEqual([])
    expect(tool({ name: 'Logs!' })).toEqual(['extensions.badToolName'])
    expect(tool({ description: ' ' })).toEqual(['extensions.noToolDescription'])
    expect(tool({ command: 'a\nb' })).toEqual(['extensions.badToolCommand'])
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
        items: [{ slot: 'agent.tools', id: 'count', label: { en: 'Count' }, action: { run: 'count' } }],
        tools: [{ name: 'changed_files', description: 'Changed files', input: { type: 'object', properties: {} } }],
      },
      permissions: { exec: ['git'] },
    }
    expect(keys(hello)).toEqual([])
    expect(keys({ ...hello, wasm: undefined })).toEqual(['extensions.needsWasm', 'extensions.toolNeedsWasm'])
    expect(keys({ ...hello, wasm: '../other/x.wasm' })).toEqual(['extensions.badWasm'])
    expect(keys({ ...hello, permissions: { exec: ['/bin/rm'] } })).toEqual(['extensions.badExec'])
    expect(keys({ ...hello, contributes: { tools: [{ name: 'x', description: 'X', input: { type: 'string' } }] } })).toEqual(['extensions.badToolInput'])
    const found = [{ dir: '/m/hello', linked: false, manifest: hello, error: null }]
    const waiting = load(found, [], {})[0]
    expect(waiting.pending).toEqual(['exec:git'])
    expect(grantsOf(waiting)).toEqual([])
    expect(toolsOf([waiting]).map((t) => [t.name, t.tool])).toEqual([['hello_changed_files', 'changed_files']])
    expect(grantsOf(load(found, [], { hello: ['exec:git'] })[0])).toEqual(['exec:git'])
  })

  it('passes the Claude Code options the user allowed', () => {
    const base = { id: 'c', version: '1.0.0', api: 1, name: { en: 'C' } }
    const chrome = { ...base, contributes: { args: ['--add-dir', '/tmp/x'] } }
    expect(keys(chrome)).toEqual([])
    for (const args of [['chrome'], '--chrome', [''], ['--a\nb']]) expect(keys({ ...base, contributes: { args } })).toEqual(['extensions.badArgs'])
    expect(keys({ ...base, contributes: { args: ['--chrome', '--permission-mode=bypassPermissions'] } })).toEqual(['extensions.reservedArg'])
    const found = [{ dir: '/m/c', linked: false, manifest: chrome, error: null }]
    const key = 'args:["--add-dir","/tmp/x"]'
    expect(load(found, [], {})[0].pending).toEqual([key])
    expect(argsOf(load(found, [], {}))).toEqual([])
    expect(argsOf(load(found, [], { c: [key] }))).toEqual(['--add-dir', '/tmp/x'])
    expect(argsOf(load(found, ['c'], { c: [key] }))).toEqual([])
  })

  it('checks panels and the items that open them', () => {
    const base = { id: 'p', version: '1.0.0', api: 1, name: { en: 'P' } }
    const panel = { id: 'main', title: { en: 'Main' }, page: 'panel/index.html' }
    const open = (value: string, slot = 'agent.tools') => ({ items: [{ slot, id: 'open', label: { en: 'Open' }, action: { 'panel.open': value } }] })
    expect(keys({ ...base, contributes: { panels: [panel], ...open('main') } })).toEqual([])
    expect(keys({ ...base, contributes: { panels: [panel], ...open('other') } })).toEqual(['extensions.unknownPanel'])
    expect(keys({ ...base, contributes: { panels: [panel], ...open('main', 'menu.squadron') } })).toEqual(['extensions.needsAgent'])
    for (const page of ['/etc/x.html', '../x.html', 'panel/x.js', 'a//b.html'])
      expect(keys({ ...base, contributes: { panels: [{ ...panel, page }] } })).toEqual(['extensions.badPanels'])
    expect(keys({ ...base, contributes: { panels: [panel, panel] } })).toEqual(['extensions.badPanels'])
    const found = [{ dir: '/m/p', linked: false, manifest: { ...base, contributes: { panels: [panel] } }, error: null }]
    expect(panelsOf(load(found, [])).map((p) => p.key)).toEqual(['extension:p.main'])
    expect(panelsOf(load(found, ['p']))).toEqual([])
  })

  it('merges the sources and takes updates only from the source an extension came from', () => {
    const entry = (id: string, version: string) => ({ id, version, name: { en: id }, repository: 'https://github.com/a/b', archive: 'https://x', sha256: 'f'.repeat(64) })
    const { entries, sources } = catalogOf([
      { source: OFFICIAL, official: true, index: { extensions: [entry('hello', '1.2.0'), { id: '../x', version: '1' }, 'junk'] } },
      { source: 'me/ext', official: false, index: { extensions: [entry('hello', '9.0.0'), entry('mine', '1.0.0')] } },
      { source: 'gone/ext', official: false, error: 'GitHub answered 404' },
    ])
    expect(entries.map((e) => `${e.id}@${e.source}`)).toEqual([`hello@${OFFICIAL}`, 'hello@me/ext', 'mine@me/ext'])
    expect(sources.map((s) => [s.source, s.official, s.error])).toEqual([[OFFICIAL, true, undefined], ['me/ext', false, undefined], ['gone/ext', false, 'GitHub answered 404']])
    expect(catalogOf(null).entries).toEqual([])
    expect(shownOf(entries, {}).map((e) => `${e.id}@${e.source}`)).toEqual([`hello@${OFFICIAL}`, 'mine@me/ext'])
    expect(shownOf(entries, { hello: 'me/ext' }).map((e) => `${e.id}@${e.source}`)).toEqual(['hello@me/ext', 'mine@me/ext'])
    expect([newer('1.10.0', '1.9.3'), newer('1.0.0', '1.0.0'), newer('0.9', '1.0.0'), newer('2.0.0', '1.99.99')]).toEqual([true, false, false, true])
    const manifest = (id: string, version: string) => ({ id, version, api: 1, name: { en: id } })
    const list = load(
      [
        { dir: '/x/hello', linked: false, manifest: manifest('hello', '1.0.0'), error: null },
        { dir: '/dev/mine', linked: true, manifest: manifest('mine', '0.1.0'), error: null },
      ],
      [],
    )
    expect(updatesOf(list, entries, {}).map((e) => `${e.id}@${e.version}`)).toEqual(['hello@1.2.0'])
    expect(updatesOf(list, entries, { hello: 'me/ext' }).map((e) => `${e.id}@${e.version}`)).toEqual(['hello@9.0.0'])
  })

  it('asks for network hosts and checks secrets', () => {
    const base = { id: 'gh', version: '1.0.0', api: 1, name: { en: 'GH' } }
    const net = { ...base, permissions: { network: ['api.github.com'] }, contributes: { secrets: [{ id: 'token', label: { en: 'Token' } }] } }
    expect(keys(net)).toEqual([])
    for (const network of [['https://api.github.com'], ['api.github.com/x'], ['localhost'], 'api.github.com'])
      expect(keys({ ...base, permissions: { network } })).toEqual(['extensions.badNetwork'])
    expect(keys({ ...base, contributes: { secrets: [{ id: 'Token', label: { en: 'T' } }] } })).toEqual(['extensions.badSecrets'])
    expect(keys({ ...base, contributes: { secrets: [{ id: 't', label: { en: 'T' } }, { id: 't', label: { en: 'U' } }] } })).toEqual(['extensions.badSecrets'])
    const found = [{ dir: '/m/gh', linked: false, manifest: net, error: null }]
    expect(load(found, [], {})[0].pending).toEqual(['net:api.github.com'])
    expect(grantsOf(load(found, [], { gh: ['net:api.github.com'] })[0])).toEqual(['net:api.github.com'])
  })

  it('asks to see and archive agents', () => {
    const base = { id: 'jira', version: '1.0.0', api: 1, name: { en: 'Jira' } }
    const both = { ...base, permissions: { agents: ['read', 'archive'] } }
    expect(keys(both)).toEqual([])
    for (const agents of [['delete'], 'read', [1]]) expect(keys({ ...base, permissions: { agents } })).toEqual(['extensions.badAgents'])
    const found = [{ dir: '/m/jira', linked: false, manifest: both, error: null }]
    expect(load(found, [], {})[0].pending).toEqual(['agents:read', 'agents:archive'])
    expect(grantsOf(load(found, [], { jira: ['agents:read'] })[0])).toEqual(['agents:read'])
  })
})
