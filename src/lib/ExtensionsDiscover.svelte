<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import InstallDialog from './InstallDialog.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import Icon from './Icon.svelte'
  import { ui } from './actions.svelte'
  import { locale, t } from './i18n/index.svelte'
  import { OFFICIAL, shownOf, type CatalogEntry, type Extension, type Source } from './extensions'
  import { addSource, asks, brokenAccounts, catalog, extensions, install, loadCatalog, removeSource } from './extensions.svelte'
  import { attempt, brand, fromOf, named, nameOf, originOf, pendingOf, problemOf, rankOf, updateOf, work } from './extension-pages.svelte'

  type Item = { id: string; name: string; source?: string; words: string; entry?: CatalogEntry; ext?: Extension }

  let query = $state('')
  let filter = $state('')
  let dialog = $state<'sources' | 'local' | null>(null)
  let chosen = $state<CatalogEntry | null>(null)
  let source = $state('')

  onMount(loadCatalog)

  const sources = $derived<Source[]>(catalog.sources.length ? catalog.sources : [{ source: OFFICIAL, official: true }, ...extensions.sources.map((s) => ({ source: s, official: false }))])
  const entries = $derived(shownOf(catalog.entries ?? [], extensions.origins).filter((c) => !extensions.list.some((e) => e.id === c.id)))
  const rank = (item: Item) => (item.ext ? rankOf(item.ext) : 3)
  const items = $derived<Item[]>(
    [
      ...extensions.list.map((e) => ({ id: e.id, name: nameOf(e), source: originOf(e), ext: e, words: [nameOf(e), named(e.manifest?.description), e.id, originOf(e) ?? ''].join(' ') })),
      ...entries.map((c) => ({ id: c.id, name: named(c.name), source: c.source, entry: c, words: [named(c.name), named(c.description), c.id, c.source].join(' ') })),
    ].sort((a, b) => rank(a) - rank(b) || a.name.localeCompare(b.name, locale())),
  )
  const keep = (item: Item, id: string) => !id || (id === 'installed' ? !!item.ext : id === 'discover' ? !item.ext : item.source === id)
  const needle = $derived(query.trim().toLowerCase())
  const visible = $derived(items.filter((item) => keep(item, filter) && (!needle || item.words.toLowerCase().includes(needle))))
  const countOf = (s: string) => (catalog.entries ?? []).filter((c) => c.source === s).length

  async function addFrom(event: SubmitEvent) {
    event.preventDefault()
    if (await attempt(() => addSource(source.trim()), 'source')) source = ''
  }

  async function link() {
    const path = await open({ directory: true, title: t('extensions.addTitle') })
    if (typeof path !== 'string') return
    const before = new Set(extensions.list.map((e) => e.dir))
    if (!(await attempt(() => invoke('extensions_link', { path }), 'local'))) return
    dialog = null
    const added = extensions.list.find((e) => e.linked && !before.has(e.dir))
    if (added) ui.settings = `ext:${added.id}`
  }

  const settingsOf = (e: Extension) => (ui.settings = `ext:${e.id}`)
  const enable = (e: Extension, on: boolean) => attempt(() => invoke('extensions_enable', { id: e.id, on }))
  const pick = (c: CatalogEntry) => ((work.error = ''), (chosen = c))

  const close = () => ((dialog = null), (chosen = null), (work.error = ''))
</script>

<div class="discover">
<div class="top">
  <div class="bar">
    <label class="search">
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
      <input placeholder={t('extensions.searchCatalog')} aria-label={t('extensions.searchLabel')} spellcheck="false" autocomplete="off" bind:value={query} />
    </label>
    <button class="btn btn-ghost" onclick={() => ((work.error = ''), (dialog = 'local'))}><Icon name="folder" />{t('extensions.localExtension')}</button>
    <button class="btn btn-ghost" onclick={() => ((work.error = ''), (dialog = 'sources'))}><Icon name="plus" />{t('extensions.marketplaces')}</button>
  </div>
  <div class="chips" role="group" aria-label={t('extensions.filters')}>
    {#each [['', 'extensions.filterAll'], ['installed', 'extensions.filterInstalled'], ['discover', 'extensions.filterDiscover']] as const as [id, key] (id)}
      <button class="chip" aria-pressed={filter === id} onclick={() => (filter = id)}>{t(key)}<span class="n">{items.filter((item) => keep(item, id)).length}</span></button>
    {/each}
    {#each sources.filter((s) => items.some((item) => item.source === s.source)) as s (s.source)}
      <button class="chip" aria-pressed={filter === s.source} onclick={() => (filter = s.source)}>{s.official ? '✓ ' : ''}{s.source}<span class="n">{items.filter((item) => item.source === s.source).length}</span></button>
    {/each}
  </div>
</div>

{#if work.error && !dialog && !chosen}<p class="bad" role="alert">{work.error}</p>{/if}
{#if catalog.error}<p class="bad">{t('extensions.catalogError', { error: catalog.error })}</p>{/if}

<div class="rows">
  {#each visible as item (item.ext?.dir ?? item.id)}
    {#if item.ext}
      {@const e = item.ext}
      {@const update = updateOf(e)}
      {@const problem = problemOf(e)}
      <div class="row" class:off={!e.enabled}>
        <ExtensionLogo id={e.id} name={item.name} logo={e.logo} />
        <button class="what" onclick={() => settingsOf(e)}>
          <span class="l1">
            <strong>{item.name}</strong>
            <span class="version">{e.manifest?.version ?? ''}{#if update}{' → '}<b>{update.version}</b>{/if}</span>
            <span class="pill {fromOf(e)}">{fromOf(e) === 'official' ? `✓ ${t('extensions.official')}` : fromOf(e) === 'imported' ? `⇄ ${t('extensions.imported', { owner: (item.source ?? '').split('/')[0] })}` : `▣ ${t('extensions.local')}`}</span>
            {#if pendingOf(e).length}<span class="pill pending">{t('extensions.toAllow')}</span>{:else if problem}<span class="pill error" title={problem}>⚠ {t('extensions.error')}</span>{:else if brokenAccounts(e)}<span class="pill error" title={t('extensions.accountBroken')}>{t('extensions.accountProblem')}</span>{/if}
          </span>
          <small>{named(e.manifest?.description)}</small>
        </button>
        <span class="tail">
          {#if update}
            <button class="btn btn-secondary update" disabled={!!work.busy} title={t('extensions.update', { version: update.version })} onclick={() => attempt(() => install(update), e.id)}>
              <Icon name="arrow-up" />{work.busy === e.id ? t('extensions.updating') : t('extensions.updateNow')}
            </button>
          {/if}
          {#if e.manifest}
            <input class="switch" type="checkbox" role="switch" checked={e.enabled} aria-label={t('extensions.enabledFor', { name: item.name })} onchange={(event) => enable(e, event.currentTarget.checked)} />
          {/if}
        </span>
        <span class="open" aria-hidden="true"><Icon name="chevron" /></span>
      </div>
    {:else if item.entry}
      {@const c = item.entry}
      {@const permissions = asks(c).length}
      <div class="row">
        <ExtensionLogo id={c.id} name={item.name} logo={brand(c.id).logo} />
        <button class="what" onclick={() => pick(c)}>
          <span class="l1">
            <strong>{item.name}</strong>
            <span class="version">{c.version}</span>
            <span class="pill" class:official={c.official} class:imported={!c.official} title={c.official ? null : t('extensions.importedTitle', { source: c.source })}>
              {c.official ? `✓ ${t('extensions.official')}` : `⇄ ${t('extensions.imported', { owner: c.source.split('/')[0] })}`}
            </span>
          </span>
          <small>{named(c.description)}</small>
        </button>
        <span class="tail">
          {#if permissions}<small class="asks">{t('extensions.permissionCount', { count: permissions })}</small>{/if}
          <button class="btn btn-primary" disabled={!!work.busy} onclick={() => pick(c)}>{work.busy === c.id ? t('extensions.installing') : t('extensions.install')}</button>
        </span>
      </div>
    {/if}
  {:else}
    <p class="empty">
      {needle ? t('extensions.noMatch', { query: query.trim() }) : filter !== 'installed' && !catalog.entries && !catalog.error ? t('extensions.catalogLoading') : filter === 'discover' ? t('extensions.allInstalled') : t('extensions.noneHere')}
    </p>
  {/each}
</div>

{#if chosen}
  <InstallDialog entry={chosen} onclose={close} />
{/if}

{#if dialog === 'sources'}
  <Dialog title={t('extensions.marketplaces')} width={560} onclose={close}>
    <div class="box">
      <p class="sub">{t('extensions.marketplacesSub')}</p>
      <form class="add" onsubmit={addFrom}>
        <input class="field mono" placeholder={t('extensions.sourcePlaceholder')} aria-label={t('extensions.sourcePlaceholder')} spellcheck="false" autocomplete="off" bind:value={source} />
        <button class="btn btn-primary" disabled={!source.trim() || !!work.busy}>{work.busy === 'source' ? t('common.checking') : t('common.add')}</button>
      </form>
      <p class="hint">{t('extensions.marketplaceHelp')}</p>
      {#if work.error}<p class="bad" role="alert">{work.error}</p>{/if}
      <ul class="sources">
        {#each sources as s (s.source)}
          <li class:failed={!!s.error}>
            <span class="mono">{s.source}</span>
            {#if s.official}<span class="pill official">✓ {t('extensions.official')}</span>{/if}
            <span class="count">{s.error ?? t('extensions.marketplaceCount', { count: countOf(s.source) })}</span>
            {#if !s.official}<button class="btn btn-ghost remove" onclick={() => attempt(() => removeSource(s.source))}>{t('common.remove')}</button>{/if}
          </li>
        {/each}
      </ul>
    </div>
    <footer>
      <button class="btn btn-secondary" onclick={close}>{t('common.close')}</button>
    </footer>
  </Dialog>
{/if}

{#if dialog === 'local'}
  <Dialog title={t('extensions.addLocal')} width={520} onclose={close}>
    <div class="box">
      <p class="sub">{t('extensions.addLocalSub')}</p>
      <p class="text">{t('extensions.fromFolderHint')}</p>
      {#if work.error}<p class="bad" role="alert">{work.error}</p>{/if}
    </div>
    <footer>
      <button class="btn btn-ghost folder" onclick={() => attempt(() => invoke('open_extensions_folder'))}><Icon name="folder" />{t('extensions.openFolder')}</button>
      <button class="btn btn-secondary" onclick={close}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!!work.busy} onclick={link}>{t('extensions.add')}</button>
    </footer>
  </Dialog>
{/if}
</div>

<style>
  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 14px;
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
    align-items: center;
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

  .chip .n {
    margin-left: 5px;
    font: 11px var(--mono);
    color: var(--text-faint);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .bar .search {
    flex: 1;
    min-width: 0;
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 6px;
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .row:hover {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .row.off :global(.logo),
  .row.off .what {
    opacity: 0.55;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
  }

  .what::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 6px;
  }

  .l1 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .l1 strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-strong);
    font-weight: 600;
  }

  .version {
    flex: none;
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .version b {
    font-weight: 400;
    color: var(--text);
  }

  .what small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .tail {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .pill {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: 4px;
    font-size: 11px;
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

  .pill.pending {
    background: var(--persona);
    color: var(--ink);
  }

  .pill.error {
    background: var(--err);
    color: var(--err-ink);
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
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .update {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }

  .open {
    flex: none;
    display: grid;
    color: var(--text-faint);
    transform: rotate(-90deg);
    pointer-events: none;
  }

  .row:hover .open {
    color: var(--text-2);
  }

  .asks {
    color: var(--text-dim);
    font-size: 12px;
    white-space: nowrap;
  }

  .empty {
    margin: 0;
    padding: 20px;
    text-align: center;
    color: var(--text-dim);
  }

  .bad {
    margin: 0 0 10px;
    color: var(--err-text);
  }

  .box {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0 22px 20px;
  }

  .box .bad {
    margin: 0;
  }

  .sub {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .text {
    margin: 0;
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.5;
  }

  .warn {
    margin: 0;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--wait-note);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-muted);
    font-size: 12.5px;
  }

  .line {
    margin: 0 0 4px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .muted {
    color: var(--text-dim);
  }

  .mono {
    font-family: var(--mono);
  }

  .add {
    display: flex;
    gap: 8px;
  }

  .add .field {
    flex: 1;
    height: 38px;
    padding: 0 12px;
    font-size: 13px;
  }

  .add .btn {
    height: 38px;
    padding: 0 16px;
  }

  .hint {
    margin: -6px 0 0;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .sources {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .sources li {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
    padding: 0 8px 0 12px;
    border-radius: 4px;
    background: var(--surface-1);
    font-size: 12.5px;
  }

  .sources .mono {
    color: var(--text);
  }

  .sources .count {
    flex: 1;
    text-align: right;
    color: var(--text-dim);
    font-size: 12px;
  }

  .sources li.failed .count {
    color: var(--err-text);
  }

  .sources .remove {
    height: 28px;
  }

  .sources .remove:hover {
    color: var(--err);
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 38px;
    padding: 0 16px;
  }

  footer .folder {
    margin-right: auto;
    height: 32px;
  }
</style>
