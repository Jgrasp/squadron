<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import Icon from './Icon.svelte'
  import { ui } from './actions.svelte'
  import { t } from './i18n/index.svelte'
  import { OFFICIAL, shownOf, type CatalogEntry, type Source } from './extensions'
  import { addSource, asks, catalog, extensions, install, loadCatalog, removeSource } from './extensions.svelte'
  import { attempt, named, work } from './extension-pages.svelte'

  let query = $state('')
  let filter = $state('')
  let dialog = $state<'sources' | 'local' | null>(null)
  let chosen = $state<CatalogEntry | null>(null)
  let source = $state('')

  onMount(loadCatalog)

  const sources = $derived<Source[]>(catalog.sources.length ? catalog.sources : [{ source: OFFICIAL, official: true }, ...extensions.sources.map((s) => ({ source: s, official: false }))])
  const entries = $derived(shownOf(catalog.entries ?? [], extensions.origins).filter((c) => !extensions.list.some((e) => e.id === c.id)))
  const needle = $derived(query.trim().toLowerCase())
  const visible = $derived(entries.filter((c) => (!filter || c.source === filter) && (!needle || [named(c.name), named(c.description), c.id, c.source].join(' ').toLowerCase().includes(needle))))
  const countOf = (s: string) => (catalog.entries ?? []).filter((c) => c.source === s).length

  async function addFrom(event: SubmitEvent) {
    event.preventDefault()
    if (await attempt(() => addSource(source.trim()), 'source')) source = ''
  }

  async function confirm() {
    const entry = chosen!
    if (await attempt(() => install(entry), entry.id)) {
      chosen = null
      ui.settings = `ext:${entry.id}`
    }
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

  const close = () => ((dialog = null), (chosen = null), (work.error = ''))
</script>

<div class="discover">
<div class="top">
  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    <input placeholder={t('extensions.searchCatalog')} aria-label={t('extensions.searchLabel')} spellcheck="false" autocomplete="off" bind:value={query} />
  </label>
  <div class="chips" role="group" aria-label={t('extensions.filters')}>
    <button class="chip" aria-pressed={!filter} onclick={() => (filter = '')}>{t('extensions.filterAll')}<span class="n">{entries.length}</span></button>
    {#each sources.filter((s) => entries.some((c) => c.source === s.source)) as s (s.source)}
      <button class="chip" aria-pressed={filter === s.source} onclick={() => (filter = s.source)}>{s.official ? '✓ ' : ''}{s.source}<span class="n">{entries.filter((c) => c.source === s.source).length}</span></button>
    {/each}
    <span class="grow"></span>
    <button class="btn btn-ghost" onclick={() => ((work.error = ''), (dialog = 'sources'))}><Icon name="plus" />{t('extensions.addMarketplace')}</button>
  </div>
</div>

{#if work.error && !dialog && !chosen}<p class="bad" role="alert">{work.error}</p>{/if}
{#if catalog.error}<p class="bad">{t('extensions.catalogError', { error: catalog.error })}</p>{/if}

<div class="grid">
  {#each visible as c (c.id)}
    {@const permissions = asks(c).length}
    <article class="card">
      <div class="head">
        <ExtensionLogo id={c.id} name={named(c.name)} />
        <strong>{named(c.name)}</strong>
        <span class="version">{c.version}</span>
      </div>
      <div>
        <span class="pill" class:official={c.official} class:imported={!c.official} title={c.official ? null : t('extensions.importedTitle', { source: c.source })}>
          {c.official ? `✓ ${t('extensions.official')}` : `⇄ ${t('extensions.imported', { owner: c.source.split('/')[0] })}`}
        </span>
      </div>
      <p class="desc">{named(c.description)}</p>
      <div class="foot">
        <small>{permissions ? t('extensions.permissionCount', { count: permissions }) : ''}</small>
        <button class="btn btn-primary" disabled={!!work.busy} onclick={() => ((work.error = ''), (chosen = c))}>{work.busy === c.id ? t('extensions.installing') : t('extensions.install')}</button>
      </div>
    </article>
  {/each}
  <button class="card new" onclick={() => ((work.error = ''), (dialog = 'local'))}>
    <span class="plus" aria-hidden="true">+</span>
    <strong>{t('extensions.addLocal')}</strong>
    <small>{t('extensions.addLocalSub')}</small>
  </button>
  {#if !visible.length}
    <p class="empty">
      {needle ? t('extensions.noMatch', { query: query.trim() }) : !catalog.entries && !catalog.error ? t('extensions.catalogLoading') : t('extensions.allInstalled')}
    </p>
  {/if}
</div>

{#if chosen}
  {@const lines = asks(chosen)}
  <Dialog title={t('extensions.installTitle', { name: named(chosen.name) })} width={520} onclose={close}>
    <div class="box">
      <p class="sub"><span class="mono">{chosen.version}</span> · {chosen.official ? t('extensions.fromOfficial', { source: chosen.source }) : t('extensions.fromImported', { source: chosen.source })}</p>
      {#if chosen.description}<p class="text">{named(chosen.description)}</p>{/if}
      {#if !chosen.official}<p class="warn">{t('extensions.notReviewed', { owner: chosen.source.split('/')[0] })}</p>{/if}
      <section>
        <h4>{t('extensions.permissions')}</h4>
        {#each lines as line (line)}<p class="line">{line}</p>{:else}<p class="line muted">{t('extensions.asksNothing')}</p>{/each}
      </section>
      {#if work.error}<p class="bad" role="alert">{work.error}</p>{/if}
    </div>
    <footer>
      <button class="btn btn-secondary" onclick={close}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!!work.busy} onclick={confirm}>{work.busy ? t('extensions.installing') : t('extensions.install')}</button>
    </footer>
  </Dialog>
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

  .grow {
    flex: 1;
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

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 10px;
  }

  .card {
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
    color: var(--text-2);
  }

  .card.new strong,
  .head strong {
    color: var(--text-strong);
  }

  .card.new small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .version {
    font: 12px var(--mono);
    color: var(--text-dim);
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

  .empty {
    grid-column: 1 / -1;
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

  h4 {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
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
