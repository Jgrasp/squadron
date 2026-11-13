import { describe, expect, it } from 'vitest'
import { active, argsOf, catalogOf, fits, grantsOf, itemsOf, load, needsAgent, newer, OFFICIAL, panelsOf, shownOf, skillsOf, spotlightOf, updatesOf, themeOf, toolsOf, validate } from './extensions'

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
    expect(item({ slot: 'artifact.actions', action: { 'prompt.insert': 'Share it' } })).toEqual([])
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

  it('only takes a share action in the Share menu, with tool steps from a wasm module', () => {
    const share = (action: object, slot = 'artifact.actions', wasm?: string) =>
      keys({ ...lando, wasm, contributes: { items: [{ slot, id: 'team', label: { en: 'Team' }, action: { 'artifact.share': action } }] } })
    const base = { confirm: { en: 'Share {artifact.title}?' }, share: 'Share {artifact.id}' }
    expect(share(base)).toEqual([])
    expect(share(base, 'agent.menu')).toEqual(['extensions.shareSlot'])
    expect(share({ share: 'x' })).toEqual(['extensions.badShare'])
    expect(share({ ...base, unshare: { tool: 'undeploy' } })).toEqual(['extensions.badShare'])
    expect(share({ ...base, unshare: { tool: 'undeploy' } }, 'artifact.actions', 'share.wasm')).toEqual([])
    expect(share({ ...base, lines: ['a\nb'] })).toEqual(['extensions.badShare'])
    const tool = { ...base, unshare: { tool: 'undeploy' } }
    expect(share({ ...tool, cleanup: { days: 90 } }, 'artifact.actions', 'share.wasm')).toEqual([])
    expect(share({ ...tool, cleanup: { days: 0 } }, 'artifact.actions', 'share.wasm')).toEqual([])
    expect(share({ ...tool, cleanup: { days: 1.5 } }, 'artifact.actions', 'share.wasm')).toEqual(['extensions.badCleanup'])
    expect(share({ ...base, unshare: 'Stop it', cleanup: { days: 90 } })).toEqual(['extensions.badCleanup'])
  })

  it('gives the skills of an extension only once the user allowed them', () => {
    expect(keys({ ...lando, contributes: { skills: ['skills/share'] } })).toEqual([])
    expect(keys({ ...lando, contributes: { skills: ['../share'] } })).toEqual(['extensions.badSkills'])
    expect(keys({ ...lando, contributes: { skills: ['a/share', 'b/share'] } })).toEqual(['extensions.badSkills'])
    const found = [{ dir: '/x/team', linked: false, error: null, manifest: { ...lando, id: 'team', contributes: { skills: ['skills/share'] } } }]
    expect(skillsOf(load(found, []))).toEqual([])
    expect(skillsOf(load(found, [], { team: ['skills:["skills/share"]'] }))).toEqual([{ id: 'team', dir: '/x/team', skills: ['skills/share'] }])
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

  it('offers every item to the spotlight, unless it opts out or needs an open artifact', () => {
    const found = (dir: string, manifest: unknown) => ({ dir, linked: false, manifest, error: null })
    const hidden = { ...lando, contributes: { items: [...lando.contributes.items, { slot: 'agent.menu', id: 'quiet', label: { en: 'Quiet' }, action: { squadron: 'kanban' }, spotlight: false }, { slot: 'artifact.actions', id: 'share', label: { en: 'Share' }, action: { 'prompt.insert': 'Share it' } }] } }
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

  it('keeps an extension or a catalog version to the Squadron versions it declares', () => {
    expect([fits(undefined, '0.45.0'), fits('>=0.45.0', '0.45.0'), fits('>=0.45.0', '0.44.9'), fits('>=0.45.0 <0.50.0', '0.50.0'), fits('>=0.45.0 <0.50.0', '0.49.3'), fits('>=0.46.0', '')]).toEqual([true, true, false, false, true, true])
    const manifest = { id: 'share', version: '0.3.0', api: 1, name: { en: 'Share' } }
    const found = (squadron: unknown) => [{ dir: '/x/share', linked: false, manifest: { ...manifest, squadron }, error: null }]
    expect(load(found('0.45'), [], {}, '0.45.0')[0].problems.map(([key]) => key)).toEqual(['extensions.badRange'])
    const old = load(found('>=0.46.0'), [], {}, '0.45.0')
    expect(old[0].problems).toEqual([['extensions.needsSquadron', { range: '>=0.46.0', version: '0.45.0' }]])
    expect(old[0].manifest?.version).toBe('0.3.0')
    expect(active(old)).toEqual([])
    expect(active(load(found('>=0.45.0'), [], {}, '0.45.0')).length).toBe(1)
    const release = (version: string, squadron?: string) => ({ version, archive: `https://x/${version}`, sha256: 'f'.repeat(64), ...(squadron ? { squadron } : {}) })
    const index = { extensions: [{ id: 'share', name: { en: 'Share' }, repository: 'https://github.com/a/b', ...release('0.3.0', '>=0.46.0'), versions: [release('0.3.0', '>=0.46.0'), { ...release('0.2.1', '>=0.45.0'), changelog: [{ version: '0.2.1', date: '2026-11-13', notes: { en: ['Fix'] } }] }, release('0.1.0')] }, { id: 'later', name: { en: 'Later' }, ...release('1.0.0', '>=0.50.0') }] }
    const pick = (app: string) => catalogOf([{ source: OFFICIAL, official: true, index }], app).entries.map((e) => `${e.id}@${e.version}`)
    expect(pick('0.46.0')).toEqual(['share@0.3.0'])
    expect(pick('0.45.0')).toEqual(['share@0.2.1'])
    expect(pick('0.44.0')).toEqual(['share@0.1.0'])
    expect(catalogOf([{ source: OFFICIAL, official: true, index }], '0.45.0').entries[0]).toMatchObject({ name: { en: 'Share' }, archive: 'https://x/0.2.1', squadron: '>=0.45.0', changelog: [{ version: '0.2.1', date: '2026-11-13', notes: { en: ['Fix'] } }] })
    expect(updatesOf(old, catalogOf([{ source: OFFICIAL, official: true, index }], '0.45.0').entries, {})).toEqual([])
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
    expect(keys({ ...base, contributes: { secrets: [{ id: 't', label: { en: 'T' }, url: 'https://github.com/settings/tokens' }] } })).toEqual([])
    for (const url of ['http://github.com/settings/tokens', 'javascript:alert(1)', 'https://', 42])
      expect(keys({ ...base, contributes: { secrets: [{ id: 't', label: { en: 'T' }, url }] } })).toEqual(['extensions.badSecrets'])
    const found = [{ dir: '/m/gh', linked: false, manifest: net, error: null }]
    expect(load(found, [], {})[0].pending).toEqual(['net:api.github.com'])
    expect(grantsOf(load(found, [], { gh: ['net:api.github.com'] })[0])).toEqual(['net:api.github.com'])
  })

  it('checks the accounts the user adds', () => {
    const base = { id: 'jira', version: '1.0.0', api: 1, name: { en: 'Jira' } }
    const field = (id: string, kind?: string) => ({ id, label: { en: id }, ...(kind ? { kind } : {}) })
    const with_ = (accounts: unknown) => keys({ ...base, contributes: { accounts } })
    expect(with_({ label: { en: 'Jira sites' }, fields: [field('site', 'host'), field('email'), field('token', 'secret')] })).toEqual([])
    const tokens = 'https://id.atlassian.com/manage-profile/security/api-tokens'
    expect(with_({ label: { en: 'Jira sites' }, fields: [{ ...field('token', 'secret'), url: tokens }] })).toEqual([])
    for (const bad of [
      { fields: [field('site', 'host')] },
      { label: { en: 'Sites' }, fields: [] },
      { label: { en: 'Sites' }, fields: [field('Site')] },
      { label: { en: 'Sites' }, fields: [field('a'), field('a')] },
      { label: { en: 'Sites' }, fields: [field('a', 'password')] },
      { label: { en: 'Sites' }, fields: [field('a', 'host'), field('b', 'host')] },
      { label: { en: 'Sites' }, fields: [{ ...field('a', 'secret'), url: 'http://id.atlassian.com/manage-profile/security/api-tokens' }] },
      { label: { en: 'Sites' }, fields: [{ ...field('a', 'secret'), url: 'https://id.atlassian.com/a b' }] },
      [field('a')],
    ])
      expect(with_(bad)).toEqual(['extensions.badAccounts'])
    expect(load([{ dir: '/m/jira', linked: false, manifest: { ...base, contributes: { accounts: { label: { en: 'S' }, fields: [field('s', 'host')] } } }, error: null }], [])[0].pending).toEqual([])
  })

  it('checks the options and the settings page an extension declares', () => {
    const base = { id: 'jira', version: '1.0.0', api: 1, name: { en: 'Jira' } }
    const with_ = (settings: unknown) => keys({ ...base, contributes: { settings } })
    const choice = (value: string) => ({ value, label: { en: value } })
    expect(
      with_({
        title: { en: 'Sharing' },
        page: 'settings/index.html',
        options: [
          { id: 'sprint', label: { en: 'Sprint' }, kind: 'select', default: 'open', choices: [choice('open'), choice('next')] },
          { id: 'chips', label: { en: 'Chips' }, kind: 'toggle', default: true },
          { id: 'limit', label: { en: 'Limit' }, kind: 'number', min: 1, default: 50, max: 100 },
          { id: 'folder', label: { en: 'Folder' }, hint: { en: 'Where' }, kind: 'text' },
        ],
      }),
    ).toEqual([])
    expect(with_({ page: '../outside.html' })).toEqual(['extensions.badSettingsPage'])
    expect(with_([])).toEqual(['extensions.badSettings'])
    for (const bad of [
      { id: 'a', label: { en: 'A' }, kind: 'color' },
      { id: 'a', label: { en: 'A' }, kind: 'toggle', default: 'yes' },
      { id: 'a', label: { en: 'A' }, kind: 'number', min: 10, default: 5 },
      { id: 'a', label: { en: 'A' }, kind: 'select', choices: [] },
      { id: 'a', label: { en: 'A' }, kind: 'select', default: 'z', choices: [choice('x')] },
      { id: 'A', label: { en: 'A' }, kind: 'text' },
    ])
      expect(with_({ options: [bad] })).toEqual(['extensions.badOption'])
    const twice = { id: 'a', label: { en: 'A' }, kind: 'text' }
    expect(with_({ options: [twice, twice] })).toEqual(['extensions.twiceOption'])
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
