<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import Icon from './Icon.svelte'
  import { ui } from './actions.svelte'
  import { t } from './i18n/index.svelte'
  import type { Extension } from './extensions'
  import { approve, catalog, extensions, install, loadCatalog } from './extensions.svelte'
  import { attempt, fromOf, named, nameOf, pendingOf, problemOf, sorted, updateOf, work } from './extension-pages.svelte'

  const FILTERS = [
    ['all', 'extensions.filterAll'],
    ['updates', 'extensions.filterUpdates'],
    ['on', 'extensions.filterOn'],
    ['off', 'extensions.filterOff'],
  ] as const

  let query = $state('')
  let filter = $state<(typeof FILTERS)[number][0]>('all')
  let menu = $state<string | null>(null)

  onMount(loadCatalog)

  const list = $derived(sorted(extensions.list))
  const needle = $derived(query.trim().toLowerCase())
  const count = (id: (typeof FILTERS)[number][0]) => list.filter((e) => keep(e, id)).length
  const keep = (e: Extension, id: (typeof FILTERS)[number][0]) => id === 'all' || (id === 'updates' ? !!updateOf(e) : id === 'on' ? e.enabled : !e.enabled)
  const words = (e: Extension) => [nameOf(e), named(e.manifest?.description), e.id, ...(e.manifest?.contributes?.tools ?? []).map((tool) => `${e.id}_${tool.name}`)].join(' ').toLowerCase()
  const visible = $derived(list.filter((e) => keep(e, filter) && (!needle || words(e).includes(needle))))
  const waiting = $derived(list.filter((e) => pendingOf(e).length))
  const updates = $derived(list.flatMap((e) => (updateOf(e) ? [{ e, entry: updateOf(e)! }] : [])))

  const open = (e: Extension) => (ui.settings = `ext:${e.id}`)
  const enable = (e: Extension, on: boolean) => attempt(() => invoke('extensions_enable', { id: e.id, on }))
  const updateAll = () =>
    attempt(async () => {
      for (const { entry } of updates) await install(entry)
    }, 'updates')
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest('.pair') && (menu = null)} />

<div class="manage">
<div class="top">
  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    <input placeholder={t('extensions.searchInstalled', { count: list.length })} aria-label={t('extensions.searchLabel')} spellcheck="false" autocomplete="off" bind:value={query} />
  </label>
  <div class="chips" role="group" aria-label={t('extensions.filters')}>
    {#each FILTERS as [id, key] (id)}
      <button class="chip" aria-pressed={filter === id} onclick={() => (filter = id)}>{t(key)}<span class="n">{count(id)}</span></button>
    {/each}
  </div>
</div>

{#if !needle}
  {#each waiting as e (e.id)}
    <div class="banner wait">
      <span>{t('extensions.wants', { name: nameOf(e), list: pendingOf(e).join(', ') })}</span>
      <button class="btn btn-secondary" onclick={() => open(e)}>{t('extensions.see')}</button>
      <button class="btn btn-primary" onclick={() => attempt(() => approve(e))}>{t('extensions.allow')}</button>
    </div>
  {/each}
  {#if updates.length}
    <div class="banner">
      <span>{t('extensions.updates', { count: updates.length, list: updates.map(({ e, entry }) => `${nameOf(e)} ${entry.version}`).join(', ') })}</span>
      <button class="btn btn-primary" disabled={!!work.busy} onclick={updateAll}>{work.busy === 'updates' ? t('extensions.installing') : t('extensions.updateAll')}</button>
    </div>
  {/if}
{/if}
{#if work.error}<p class="bad" role="alert">{work.error}</p>{/if}
{#if catalog.error}<p class="bad">{t('extensions.catalogError', { error: catalog.error })}</p>{/if}

<div class="rows">
  {#each visible as e (e.id)}
    {@const update = updateOf(e)}
    {@const problem = problemOf(e)}
    <div class="row" class:off={!e.enabled}>
      <ExtensionLogo id={e.id} name={nameOf(e)} />
      <button class="what" onclick={() => open(e)}>
        <span class="l1">
          <strong>{nameOf(e)}</strong>
          <span class="version">{e.manifest?.version ?? ''}{#if update}{' → '}<b>{update.version}</b>{/if}</span>
          <span class="pill {fromOf(e)}">{fromOf(e) === 'official' ? `✓ ${t('extensions.official')}` : fromOf(e) === 'imported' ? `⇄ ${t('extensions.imported', { owner: (extensions.origins[e.id] ?? '').split('/')[0] })}` : `▣ ${t('extensions.local')}`}</span>
          {#if problem}<span class="pill error" title={problem}>⚠ {t('extensions.error')}</span>{:else if pendingOf(e).length}<span class="pill pending">{t('extensions.toAllow')}</span>{/if}
        </span>
        <small>{named(e.manifest?.description)}</small>
      </button>
      <span class="tail">
        {#if e.manifest}
          <input class="switch" type="checkbox" role="switch" checked={e.enabled} aria-label={t('extensions.enabledFor', { name: nameOf(e) })} onchange={(event) => enable(e, event.currentTarget.checked)} />
        {/if}
        <span class="pair">
          {#if update}
            <button class="main up" disabled={!!work.busy} title={t('extensions.update', { version: update.version })} onclick={() => attempt(() => install(update), e.id)}>
              <Icon name="arrow-up" />{work.busy === e.id ? t('extensions.updating') : t('extensions.updateNow')}
            </button>
            <button class="caret" aria-label={t('extensions.moreFor', { name: nameOf(e) })} aria-expanded={menu === e.id} onclick={() => (menu = menu === e.id ? null : e.id)}><Icon name="chevron" /></button>
            {#if menu === e.id}
              <div class="menu" role="menu">
                <button role="menuitem" onclick={() => ((menu = null), open(e))}><Icon name="gear" />{t('extensions.settings')}</button>
              </div>
            {/if}
          {:else}
            <button class="main solo" onclick={() => open(e)}><Icon name="gear" />{t('extensions.settings')}</button>
          {/if}
        </span>
      </span>
    </div>
  {:else}
    <p class="empty">
      {#if needle}{t('extensions.noMatch', { query: query.trim() })}{:else if list.length}{t('extensions.noneHere')}{:else}{t('extensions.noneInstalled')}
        <button class="btn btn-secondary" onclick={() => (ui.settings = 'discover')}>{t('extensions.discoverTitle')}</button>{/if}
    </p>
  {/each}
</div>
</div>

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

  .chip .n {
    margin-left: 5px;
    font: 11px var(--mono);
    color: var(--text-faint);
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

  .banner.wait .btn-secondary {
    background: var(--wait-btn);
    color: var(--wait-btn-text);
  }

  .banner.wait .btn-primary {
    background: var(--accent);
    color: var(--accent-ink);
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

  .row:has(.menu) .tail {
    z-index: 2;
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

  .pair {
    position: relative;
    display: inline-flex;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .pair > button {
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 12px;
    border-radius: 4px;
    color: var(--text-2);
    font-size: 13px;
  }

  .pair > button:hover:not(:disabled) {
    background: var(--surface-3);
    color: var(--text);
  }

  .pair .main {
    width: 104px;
  }

  .pair .solo {
    width: 132px;
  }

  .pair .up {
    border-radius: 4px 0 0 4px;
    color: var(--text);
    font-weight: 600;
  }

  .pair .caret {
    width: 28px;
    padding: 0;
    border-radius: 0 4px 4px 0;
    box-shadow: inset 1px 0 0 var(--border-strong);
  }

  .pair .caret[aria-expanded='true'] {
    background: var(--surface-4);
    color: var(--text);
  }

  .menu {
    position: absolute;
    top: 34px;
    right: 0;
    width: 200px;
    padding: 5px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.5),
      0 0 0 1px var(--border-strong);
  }

  .menu button {
    width: 100%;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
    color: var(--text);
  }

  .empty {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0;
    padding: 20px;
    color: var(--text-dim);
  }

  .bad {
    margin: 0 0 10px;
    color: var(--err-text);
  }
</style>
