<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { ask } from './asking.svelte'
  import { errorText, locale, t } from './i18n/index.svelte'
  import { OFFICIAL, shownOf, text, updatesOf, type Account, type CatalogEntry, type Extension, type Source, type Text } from './extensions'
  import { addSource, approve, asks, catalog, commandLine, install, loadCatalog, loadExtensions, extensions, removeAccount, removeSource, saveAccount, uninstall } from './extensions.svelte'
  import { home } from './projects.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { keepLabel } from './cleanup.svelte'

  type From = 'official' | 'imported' | 'local'
  type Card = {
    id: string
    name: string
    version: string
    description: string
    from: From
    source: string
    installed?: Extension
    entry?: CatalogEntry
    update?: CatalogEntry
    error: string
    pending: string[]
    tools: string[]
    permissions: string[]
  }

  const FILTERS = [
    ['all', 'extensions.filterAll'],
    ['installed', 'extensions.filterInstalled'],
    ['discover', 'extensions.filterDiscover'],
    ['official', 'extensions.filterOfficial'],
    ['imported', 'extensions.filterImported'],
    ['local', 'extensions.filterLocal'],
  ] as const
  const COLORS = ['#e08a5b', '#4fa3e0', '#a78bfa', '#7cc47c', '#e3b341', '#6fa0d0', '#3cc39a', '#e07a9b']

  const cleanupOf = (extension?: Extension) =>
    extension?.manifest?.contributes?.items?.flatMap((i) => ('artifact.share' in i.action && i.action['artifact.share'].cleanup ? [i.action['artifact.share'].cleanup.days] : []))[0]

  function setCleanup(id: string, value: string) {
    const { [id]: _, ...rest } = settings.artifacts_extensions
    saveSettings({ ...$state.snapshot(settings), artifacts_extensions: value === 'own' ? rest : { ...rest, [id]: Number(value) } }).catch((e) => (error = errorText(e)))
  }

  let error = $state('')
  let busy = $state('')
  let query = $state('')
  let filter = $state<(typeof FILTERS)[number][0]>('all')
  let shown = $state<string | null>(null)
  let adding = $state(false)
  let source = $state('')
  let set = $state<Record<string, string[]>>({})
  let typed = $state<Record<string, string>>({})
  let draft = $state<{ extension: string; account: string | null; values: Record<string, string> } | null>(null)

  const named = (value: Text | undefined) => (value ? text(value, locale()) : '')
  const color = (id: string) => COLORS[[...id].reduce((sum, c) => sum + c.charCodeAt(0), 0) % COLORS.length]

  const cards = $derived.by((): Card[] => {
    const entries = catalog.entries ?? []
    const ids = new Set(extensions.list.map((e) => e.id))
    const installed = extensions.list.map((e): Card => {
      const origin = e.linked ? undefined : extensions.origins[e.id]
      const m = e.manifest
      return {
        id: e.id,
        name: m ? named(m.name) : e.id,
        version: m?.version ?? '',
        description: named(m?.description),
        from: !origin ? 'local' : origin === OFFICIAL ? 'official' : 'imported',
        source: origin ?? '',
        installed: e,
        update: e.linked ? undefined : updatesOf([e], entries, extensions.origins)[0],
        error: [...e.problems.map(([key, values]) => t(key, values)), ...(e.enabled && extensions.errors[e.id] ? [extensions.errors[e.id]] : [])].join(' '),
        pending: e.enabled && m ? e.pending.map(commandLine) : [],
        tools: (m?.contributes?.tools ?? []).map((tool) => `${e.id}_${tool.name}`),
        permissions: e.commands.map(commandLine),
      }
    })
    const available = shownOf(entries, extensions.origins)
      .filter((c) => !ids.has(c.id))
      .map((c): Card => ({ id: c.id, name: named(c.name), version: c.version, description: named(c.description), from: c.official ? 'official' : 'imported', source: c.source, entry: c, error: '', pending: [], tools: [], permissions: asks(c) }))
    return [...installed, ...available]
  })

  const needle = $derived(query.trim().toLowerCase())
  const visible = $derived(
    cards.filter(
      (c) =>
        (filter === 'all' || (filter === 'installed' ? !!c.installed : filter === 'discover' ? !c.installed : c.from === filter)) &&
        (!needle || [c.name, c.description, c.id, c.source, ...c.tools].join(' ').toLowerCase().includes(needle)),
    ),
  )
  const waiting = $derived(cards.filter((c) => c.pending.length))
  const updates = $derived(cards.filter((c) => c.update))
  const detail = $derived(cards.find((c) => c.id === shown))
  const sources = $derived<Source[]>(catalog.sources.length ? catalog.sources : [{ source: OFFICIAL, official: true }, ...extensions.sources.map((s) => ({ source: s, official: false }))])

  onMount(loadCatalog)

  $effect(() => {
    for (const m of extensions.list) {
      const names = (m.manifest?.contributes?.secrets ?? []).map((s) => s.id)
      if (names.length) invoke<string[]>('extensions_secrets', { id: m.id, names }).then((found) => (set[m.id] = found), (e) => (error = errorText(e)))
    }
  })

  async function attempt(job: () => Promise<unknown>, key = '') {
    error = ''
    busy = key
    try {
      await job()
      await loadExtensions()
    } catch (e) {
      error = errorText(e)
    }
    busy = ''
  }

  async function link() {
    const path = await open({ directory: true, title: t('extensions.addTitle') })
    if (typeof path === 'string') attempt(() => invoke('extensions_link', { path })).then(() => (adding = false))
  }

  const enable = (e: Extension, on: boolean) => attempt(() => invoke('extensions_enable', { id: e.id, on }))

  async function remove(c: Card) {
    const e = c.installed!
    if (e.linked) return attempt(() => invoke('extensions_unlink', { path: e.dir }))
    if (!(await ask(t('extensions.uninstallText', { name: c.name }), { title: t('extensions.uninstallTitle', { name: c.name }), kind: 'warning', okLabel: t('extensions.uninstall') }))) return
    shown = null
    attempt(() => uninstall(e))
  }

  const updateAll = () => attempt(async () => {
    for (const c of updates) await install(c.update!)
  }, 'updates')

  async function addFrom(event: SubmitEvent) {
    event.preventDefault()
    await attempt(() => addSource(source.trim()), 'source')
    if (!error) source = ''
  }

  async function keep(e: Extension, name: string, value: string | null) {
    await attempt(() => invoke('extensions_secret_set', { id: e.id, name, value }))
    if (!error) {
      set[e.id] = value === null ? (set[e.id] ?? []).filter((n) => n !== name) : [...new Set([...(set[e.id] ?? []), name])]
      typed[`${e.id}.${name}`] = ''
    }
  }

  const summary = (e: Extension, a: Account) =>
    (e.manifest?.contributes?.accounts?.fields ?? [])
      .filter((f) => f.kind !== 'secret')
      .map((f) => a.values[f.id])
      .filter(Boolean)
      .join(' · ')

  const blank = (extension: string) => ({ extension, account: null, values: {} })

  $effect(() => {
    const e = detail?.installed
    if (e?.manifest?.contributes?.accounts && draft?.extension !== e.id) draft = blank(e.id)
  })

  const complete = (e: Extension) => {
    const values = draft?.extension === e.id ? draft.values : {}
    const kept = draft?.account ? (extensions.accounts[e.id]?.find((a) => a.id === draft!.account)?.secrets ?? []) : []
    return (e.manifest?.contributes?.accounts?.fields ?? []).every((f) => values[f.id]?.trim() || kept.includes(f.id))
  }

  async function keepAccount(event: SubmitEvent, e: Extension) {
    event.preventDefault()
    if (!draft || draft.extension !== e.id) return
    const { account, values } = $state.snapshot(draft)
    await attempt(() => saveAccount(e.id, account, values))
    if (!error) draft = blank(e.id)
  }

  async function dropAccount(e: Extension, a: Account, name: string) {
    if (!(await ask(t('extensions.accountRemoveText', { name }), { title: t('extensions.accountRemoveTitle'), kind: 'warning', okLabel: t('common.remove') }))) return
    if (draft?.account === a.id) draft = blank(e.id)
    attempt(() => removeAccount(e.id, a.id))
  }

  const counts = (c: Card) =>
    [c.tools.length ? t('extensions.toolCount', { count: c.tools.length }) : '', c.permissions.length ? t('extensions.permissionCount', { count: c.permissions.length }) : ''].filter(Boolean).join(' · ')
</script>

{#snippet badges(c: Card)}
  <span class="pill {c.from}" title={c.from === 'imported' ? t('extensions.importedTitle', { source: c.source }) : null}>
    {c.from === 'official' ? `✓ ${t('extensions.official')}` : c.from === 'imported' ? `⇄ ${t('extensions.imported', { owner: c.source.split('/')[0] })}` : `▣ ${t('extensions.local')}`}
  </span>
  {#if c.error}
    <span class="pill error">⚠ {t('extensions.error')}</span>
  {:else if c.pending.length}
    <span class="pill pending">{t('extensions.toAllow')}</span>
  {:else if c.update}
    <span class="pill update">↑ {c.update.version}</span>
  {/if}
{/snippet}

{#snippet action(c: Card)}
  {#if c.entry}
    <button class="btn btn-primary" disabled={!!busy} onclick={() => attempt(() => install(c.entry!), c.id)}>{busy === c.id ? t('extensions.installing') : t('extensions.install')}</button>
  {:else if c.installed}
    <button class="btn danger" onclick={() => remove(c)} title={c.installed.linked ? t('extensions.removeHint') : null}>{c.installed.linked ? t('common.remove') : t('extensions.uninstall')}</button>
  {/if}
{/snippet}

<div class="top">
  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    <input placeholder={t('extensions.search', { count: cards.length })} aria-label={t('extensions.searchLabel')} spellcheck="false" autocomplete="off" bind:value={query} />
  </label>
  <div class="chips" role="group" aria-label={t('extensions.filters')}>
    {#each FILTERS as [id, key] (id)}
      <button class="chip" aria-pressed={filter === id} onclick={() => (filter = id)}>{t(key)}</button>
    {/each}
  </div>
</div>

{#if !needle}
  {#each waiting as c (c.id)}
    <div class="banner wait">
      <span>{t('extensions.wants', { name: c.name, list: c.pending.join(', ') })}</span>
      <button class="btn btn-secondary" onclick={() => (shown = c.id)}>{t('extensions.see')}</button>
      <button class="btn btn-primary" onclick={() => attempt(() => approve(c.installed!))}>{t('extensions.allow')}</button>
    </div>
  {/each}
  {#if updates.length}
    <div class="banner">
      <span>{t('extensions.updates', { count: updates.length, list: updates.map((c) => `${c.name} ${c.update!.version}`).join(', ') })}</span>
      <button class="btn btn-primary" disabled={!!busy} onclick={updateAll}>{busy === 'updates' ? t('extensions.installing') : t('extensions.updateAll')}</button>
    </div>
  {/if}
{/if}
{#if error}<p class="bad" role="alert">{error}</p>{/if}
{#if catalog.error}<p class="bad">{t('extensions.catalogError', { error: catalog.error })}</p>{/if}

<div class="grid">
  <button class="card new" onclick={() => (adding = true)}>
    <span class="plus" aria-hidden="true">+</span>
    <strong>{t('extensions.addExtension')}</strong>
    <small>{t('extensions.addHint')}</small>
  </button>
  {#each visible as c (c.id)}
    <article class="card" class:off={c.installed && !c.installed.enabled}>
      <div class="head">
        <span class="logo" style:background={color(c.id)} aria-hidden="true">{c.name.charAt(0).toUpperCase()}</span>
        <button class="open" onclick={() => (shown = c.id)}><strong>{c.name}</strong><span class="version">{c.version}</span></button>
        {#if c.installed?.manifest}
          <input class="switch" type="checkbox" role="switch" checked={c.installed.enabled} aria-label={t('extensions.enabledFor', { name: c.name })} onchange={(e) => enable(c.installed!, e.currentTarget.checked)} />
        {/if}
      </div>
      <div class="badges">{@render badges(c)}</div>
      <p class="desc">{c.description}</p>
      <div class="foot">
        <small>{counts(c)}</small>
        {@render action(c)}
      </div>
    </article>
  {/each}
  {#if !visible.length && (needle || filter !== 'all')}
    <p class="empty">{needle ? t('extensions.noMatch', { query: query.trim() }) : t('extensions.noneHere')}</p>
  {:else if !catalog.entries && !catalog.error}
    <p class="empty">{t('extensions.catalogLoading')}</p>
  {/if}
</div>

{#if detail}
  <Dialog title={detail.name} width={640} onclose={() => (shown = null)}>
    {#snippet badge()}<span class="version">{detail.version}</span><span class="badges">{@render badges(detail)}</span>{/snippet}
    <div class="detail">
      {#if detail.description}<p>{detail.description}</p>{/if}
      <div class="actions">
        {#if detail.installed}
          {#if detail.pending.length}<button class="btn btn-primary" onclick={() => attempt(() => approve(detail.installed!))}>{t('extensions.allow')}</button>{/if}
          {#if detail.update}<button class="btn btn-primary" disabled={!!busy} onclick={() => attempt(() => install(detail.update!), detail.id)}>{t('extensions.update', { version: detail.update.version })}</button>{/if}
          {#if detail.installed.manifest}
            <label class="enabled">{t('extensions.enabled')}<input class="switch" type="checkbox" role="switch" checked={detail.installed.enabled} onchange={(e) => enable(detail.installed!, e.currentTarget.checked)} /></label>
          {/if}
          <span class="grow"></span>
          <button class="btn btn-secondary" onclick={() => attempt(() => invoke('reveal_attachment', { path: detail.installed!.dir }))}>{t('extensions.openItsFolder')}</button>
        {/if}
        {@render action(detail)}
      </div>
      {#if detail.error}<p class="bad">{detail.error}</p>{/if}
      <div class="cols">
        <section>
          <h4>{t('extensions.permissions')}</h4>
          {#each detail.permissions as line (line)}<p class="line" class:new={detail.pending.includes(line)}>{line}</p>{:else}<p class="line muted">{t('extensions.nothing')}</p>{/each}
          {#if detail.installed}
            <h4>{t('extensions.toolsTitle')}</h4>
            {#each detail.tools as tool (tool)}<code>{tool}</code>{:else}<p class="line muted">{t('extensions.nothing')}</p>{/each}
          {/if}
        </section>
        <section>
          {#if detail.installed?.enabled}
            {#each detail.installed.manifest?.contributes?.secrets ?? [] as s (s.id)}
              {@const key = `${detail.id}.${s.id}`}
              {@const saved = set[detail.id]?.includes(s.id)}
              <h4>{text(s.label, locale())}</h4>
              <form class="secret" onsubmit={(e) => (e.preventDefault(), keep(detail.installed!, s.id, typed[key]))}>
                <input class="field mono" type="password" autocomplete="off" spellcheck="false" placeholder={saved ? t('extensions.secretSet') : t('extensions.secretUnset')} aria-label={text(s.label, locale())} bind:value={typed[key]} />
                <button class="btn btn-secondary" disabled={!typed[key]?.trim()}>{t('common.save')}</button>
                {#if saved}<button type="button" class="btn danger" onclick={() => keep(detail.installed!, s.id, null)}>{t('common.remove')}</button>{/if}
              </form>
              <p class="line muted">{s.hint ? `${text(s.hint, locale())} ` : ''}{t('extensions.secretHint')}</p>
            {/each}
            {@const accounts = detail.installed.manifest?.contributes?.accounts}
            {#if accounts && draft?.extension === detail.id}
              {@const e = detail.installed}
              {@const form = draft}
              {@const kept = form.account ? (extensions.accounts[e.id]?.find((a) => a.id === form.account)?.secrets ?? []) : []}
              <h4>{text(accounts.label, locale())}</h4>
              <ul class="accounts">
                {#each extensions.accounts[e.id] ?? [] as a (a.id)}
                  {@const name = summary(e, a) || a.id}
                  <li class:editing={form.account === a.id}>
                    <span class="mono">{name}</span>
                    <button class="btn btn-secondary" onclick={() => (draft = { extension: e.id, account: a.id, values: { ...a.values } })}>{t('common.edit')}</button>
                    <button class="btn danger" onclick={() => dropAccount(e, a, name)}>{t('common.remove')}</button>
                  </li>
                {:else}
                  <li class="muted">{t('extensions.accountsNone')}</li>
                {/each}
              </ul>
              <form class="account" onsubmit={(event) => keepAccount(event, e)}>
                {#each accounts.fields as f (f.id)}
                  {@const label = text(f.label, locale())}
                  <input
                    class="field"
                    class:mono={f.kind !== 'text' && f.kind !== undefined}
                    type={f.kind === 'secret' ? 'password' : 'text'}
                    autocomplete="off"
                    spellcheck="false"
                    placeholder={kept.includes(f.id) ? t('extensions.accountKept', { label }) : label}
                    aria-label={label}
                    title={f.hint ? text(f.hint, locale()) : null}
                    bind:value={form.values[f.id]}
                  />
                {/each}
                <div class="row">
                  <button class="btn btn-secondary" disabled={!complete(e) || !!busy}>{form.account ? t('common.save') : t('extensions.accountAdd')}</button>
                  {#if form.account}<button type="button" class="btn btn-ghost" onclick={() => (draft = blank(e.id))}>{t('common.cancel')}</button>{/if}
                </div>
              </form>
              <p class="line muted">{t('extensions.accountsHint')}</p>
            {/if}
          {/if}
          {#if detail.installed?.enabled && cleanupOf(detail.installed) !== undefined}
            {@const cleanup = cleanupOf(detail.installed)!}
            <h4>{t('settings.extensionCleanup')}</h4>
            <select class="field" aria-label={t('settings.extensionCleanup')} value={String(settings.artifacts_extensions?.[detail.id] ?? "own")} onchange={(e) => setCleanup(detail.id, e.currentTarget.value)}>
              <option value="own">{t('settings.extensionCleanupOwn', { time: keepLabel(cleanup) })}</option>
              <option value="-1">{t('settings.extensionCleanupRule', { time: keepLabel(settings.artifacts_after_days) })}</option>
              {#each [30, 90, 365, 0] as days (days)}<option value={String(days)}>{keepLabel(days)}</option>{/each}
            </select>
            <p class="line muted">{t('settings.extensionCleanupHint')}</p>
          {/if}
          {#if detail.installed}
            <h4>{t('extensions.location')}</h4>
            <p class="line mono">{home(detail.installed.dir)}</p>
          {:else if detail.entry}
            <h4>{t('extensions.sourceTitle')}</h4>
            <p class="line mono">{detail.entry.source}</p>
          {/if}
        </section>
      </div>
    </div>
  </Dialog>
{/if}

{#if adding}
  <Dialog title={t('extensions.addExtension')} width={580} onclose={() => (adding = false)}>
    <div class="ways">
      <button class="way" onclick={() => ((filter = 'discover'), (query = ''), (adding = false))}>
        <span class="ico" aria-hidden="true">◎</span>
        <span class="text"><strong>{t('extensions.fromCatalog')}</strong><small>{t('extensions.fromCatalogHint')}</small></span>
        <span class="btn btn-secondary">{t('extensions.browse')}</span>
      </button>
      <div class="way">
        <span class="ico" aria-hidden="true">▣</span>
        <span class="text"><strong>{t('extensions.fromFolder')}</strong><small>{t('extensions.fromFolderHint')}</small></span>
        <button class="btn btn-secondary" onclick={link}>{t('extensions.add')}</button>
      </div>
      <div class="way column">
        <div class="row">
          <span class="ico" aria-hidden="true">⇄</span>
          <span class="text"><strong>{t('extensions.fromSource')}</strong><small>{t('extensions.sourcesHint')}</small></span>
        </div>
        <ul class="sources">
          {#each sources as s (s.source)}
            <li>
              <span class="mono">{s.source}</span>
              {#if s.error}<small class="bad">{s.error}</small>{/if}
              {#if s.official}<span class="pill official">✓ {t('extensions.official')}</span>{:else}<button class="btn danger" onclick={() => attempt(() => removeSource(s.source))}>{t('common.remove')}</button>{/if}
            </li>
          {/each}
        </ul>
        <form class="secret" onsubmit={addFrom}>
          <input class="field mono" placeholder={t('extensions.sourcePlaceholder')} aria-label={t('extensions.sourcePlaceholder')} spellcheck="false" autocomplete="off" bind:value={source} />
          <button class="btn btn-secondary" disabled={!source.trim() || !!busy}>{busy === 'source' ? t('common.checking') : t('extensions.sourceAdd')}</button>
        </form>
      </div>
      {#if error}<p class="bad" role="alert">{error}</p>{/if}
      <button class="btn btn-ghost folder" onclick={() => attempt(() => invoke('open_extensions_folder'))}>{t('extensions.openFolder')}</button>
    </div>
  </Dialog>
{/if}

<style>
  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 12px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .search svg {
    flex: none;
    fill: none;
    stroke: var(--text-dim);
    stroke-width: 2;
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 12px;
  }

  .chip[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    padding: 8px 8px 8px 12px;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .banner.wait {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text);
  }

  .banner span {
    flex: 1;
    min-width: 0;
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .btn-secondary {
    background: var(--surface-4);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--surface-5);
  }

  .btn.danger {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
  }

  .btn.danger:hover {
    background: var(--del-bg);
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }

  .switch:focus-visible {
    outline: 2px solid var(--border-focus);
    outline-offset: 2px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 10px;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  .card:hover {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .card.off .head,
  .card.off .desc {
    opacity: 0.55;
  }

  .card.new {
    align-items: center;
    justify-content: center;
    min-height: 150px;
    gap: 4px;
    text-align: center;
    background: none;
    box-shadow: none;
    border: 1px dashed var(--border-dashed);
  }

  .card.new:hover {
    border-color: var(--border-focus);
    background: var(--surface-1);
  }

  .card.new .plus {
    font-size: 24px;
    line-height: 1;
    color: var(--accent);
  }

  .card.new strong {
    color: var(--text-strong);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 7px;
    color: var(--ink);
    font-weight: 700;
  }

  .open {
    flex: 1;
    min-width: 0;
    text-align: left;
  }

  .open::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 6px;
  }

  .open strong {
    color: var(--text-strong);
  }

  .switch,
  .foot .btn {
    position: relative;
    z-index: 1;
  }

  .version {
    margin-left: 6px;
    font: 400 12px var(--mono);
    color: var(--text-dim);
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 9px;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 700;
    white-space: nowrap;
  }

  .pill.official {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--ok);
  }

  .pill.imported {
    background: var(--option-off);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .pill.local {
    background: var(--background-bg);
    box-shadow: inset 0 0 0 1px var(--background-line);
    color: var(--remote-text);
  }

  .pill.update {
    background: var(--accent);
    color: var(--ink);
  }

  .pill.pending {
    background: var(--persona);
    color: var(--ink);
  }

  .pill.error {
    background: var(--err);
    color: var(--err-ink);
  }

  .desc {
    margin: 0;
    min-height: 36px;
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: auto;
  }

  .foot small {
    flex: 1;
    color: var(--text-dim);
  }

  .empty,
  .bad {
    margin: 0 0 10px;
    color: var(--text-dim);
  }

  .grid .empty {
    grid-column: 1 / -1;
    padding: 20px;
    text-align: center;
  }

  .bad {
    color: var(--err-text);
  }

  .detail {
    padding: 0 22px 22px;
  }

  .detail > p {
    margin: 0 0 14px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .grow {
    flex: 1;
  }

  .enabled {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 24px;
  }

  h4 {
    margin: 18px 0 8px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .line {
    margin: 0 0 4px;
    color: var(--text-2);
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }

  .line.new {
    color: var(--text-strong);
    font-weight: 600;
  }

  .muted {
    color: var(--text-dim);
  }

  .mono {
    font-family: var(--mono);
  }

  code {
    display: inline-block;
    margin: 0 4px 4px 0;
    padding: 1px 6px;
    border-radius: 3px;
    font: 12px var(--mono);
    background: var(--surface-3);
  }

  .secret {
    display: flex;
    gap: 6px;
    margin-bottom: 6px;
  }

  .secret .field {
    flex: 1;
    height: 30px;
    padding: 0 10px;
  }

  .accounts {
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
  }

  .accounts li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 0;
    font-size: 12.5px;
  }

  .accounts li .mono {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .accounts li.editing .mono {
    color: var(--text-strong);
  }

  .account {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 6px;
  }

  .account .field {
    height: 30px;
    padding: 0 10px;
  }

  .account .row {
    display: flex;
    gap: 6px;
  }

  .ways {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0 22px 22px;
  }

  .way {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  button.way:hover {
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  .way.column {
    flex-direction: column;
    align-items: stretch;
  }

  .way .row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .ico {
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: var(--surface-4);
    color: var(--accent);
    font-size: 16px;
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .text strong {
    color: var(--text-strong);
  }

  .text small {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .sources {
    margin: 0;
    padding: 0 0 0 50px;
    list-style: none;
  }

  .sources li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-size: 12.5px;
  }

  .sources li .mono {
    flex: 1;
  }

  .way.column .secret {
    padding-left: 50px;
  }

  .folder {
    align-self: flex-start;
  }
</style>
