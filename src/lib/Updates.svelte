<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { ui } from './actions.svelte'
  import { errorText, list, locale, number, t } from './i18n/index.svelte'
  import { ownerOf, releasesOf, updatesOf, type CatalogEntry, type Extension } from './extensions'
  import { asks, catalog, commandLine, extensions, install } from './extensions.svelte'
  import { fromOf, nameOf } from './extension-pages.svelte'
  import { install as installSquadron, update } from './update.svelte'

  let { onclose }: { onclose: () => void } = $props()

  type Row = { e: Extension; entry: CatalogEntry; from: string; asking: string[] }
  type Step = { status: 'busy' | 'done' | 'error'; error?: string }

  const rows: Row[] = untrack(() =>
    updatesOf(extensions.list, catalog.entries ?? [], extensions.origins).flatMap((entry) => {
      const e = extensions.list.find((x) => x.id === entry.id && !x.linked)
      if (!e?.manifest) return []
      const granted = e.commands.map(commandLine)
      return [{ e, entry, from: e.manifest.version, asking: asks(entry).filter((line) => !granted.includes(line)) }]
    }),
  )

  const SQUADRON = ''

  let steps = $state<Record<string, Step>>({})
  let open = $state<string | null>(null)
  let version = $state('')

  getVersion()
    .then((v) => (version = v))
    .catch(() => {})

  const squadron = $derived(update.found)
  const installed = $derived(extensions.list.filter((e) => e.manifest && !e.linked))
  const done = $derived(!rows.length && !squadron && !catalog.error)
  const known = $derived(!import.meta.env.DEV && !!version)
  const summary = $derived(known ? t('extensions.allUpToDate', { version, count: installed.length }) : t('extensions.noUpdates', { count: installed.length }))
  const shown = $derived(installed.slice(0, 7))
  const hidden = $derived(installed.slice(7))
  const busy = $derived(update.progress !== null || Object.values(steps).some((s) => s.status === 'busy'))
  const left = $derived(rows.filter((r) => steps[r.e.id]?.status !== 'done'))
  const asking = $derived(left.filter((r) => r.asking.length))

  async function one(row: Row) {
    steps[row.e.id] = { status: 'busy' }
    try {
      await install(row.entry)
      steps[row.e.id] = { status: 'done' }
    } catch (err) {
      steps[row.e.id] = { status: 'error', error: errorText(err) }
    }
  }

  async function all() {
    for (const row of left) await one(row)
    if (squadron) await installSquadron()
  }

  const notes = (row: Row) => releasesOf(row.entry, row.from)

  const lines = (notes: Record<string, string[]>) => (notes[locale()] ?? notes.en).map((l) => `- ${l}`).join('\n')

  function manage() {
    onclose()
    ui.settings = 'discover'
  }
</script>

{#snippet tick()}<span class="tick"><Icon name="check" size={9} /></span>{/snippet}

<Dialog title={t('extensions.updatesTitle')} width={600} {onclose}>
  {#if done}
    <div class="hero">
      <span class="seal"><Icon name="check" size={30} /></span>
      <h2>{t('extensions.upToDate')}</h2>
      <p class="summary">
        {#if known}
          {@const [before, after] = summary.split(version)}
          {before}<span class="mono">{version}</span>{after}
        {:else}
          {summary}
        {/if}
      </p>
      {#if known || shown.length}
        <div class="crew">
          {#if known}
            <span style:--i={0} title="Squadron {version}"><span class="brand" aria-hidden="true"><span class="triangle"></span></span>{@render tick()}</span>
          {/if}
          {#each shown as e, i (e.id)}
            <span style:--i={i + 1} title="{nameOf(e)} {e.manifest?.version}"><ExtensionLogo id={e.id} name={nameOf(e)} logo={e.logo} size={30} />{@render tick()}</span>
          {/each}
          {#if hidden.length}
            <span class="more" style:--i={shown.length + 1} title={list(hidden.map(nameOf))}>+{hidden.length}</span>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <p class="intro">
      {#if rows.length || squadron}
        {t('extensions.updatesIntro', { count: rows.length + (squadron ? 1 : 0) })}
      {:else}
        {t('extensions.catalogError', { error: catalog.error })}
      {/if}
    </p>
    {#if catalog.error && (rows.length || squadron)}<p class="intro bad">{t('extensions.catalogError', { error: catalog.error })}</p>{/if}
    <div class="list">
      {#if squadron}
        {#if rows.length}<span class="group">Squadron</span>{/if}
        <div class="row" class:joined={open === SQUADRON}>
          <span class="brand" aria-hidden="true"><span class="triangle"></span></span>
          <span class="what">
            <span class="l1">
              <strong>Squadron</strong>
              <span class="version">{squadron.currentVersion}{' → '}<b>{squadron.version}</b></span>
            </span>
            {#if update.error}
              <small class="bad">{update.error}</small>
            {:else}
              <small>{t('extensions.squadronRestarts')}</small>
            {/if}
          </span>
          {#if squadron.body}
            <button class="log" aria-expanded={open === SQUADRON} onclick={() => (open = open === SQUADRON ? null : SQUADRON)}>{t('extensions.whatsNew')}</button>
          {/if}
          {#if update.progress !== null}
            <span class="act busy"><span class="spin" aria-hidden="true"></span>{number(update.progress, { style: 'percent', maximumFractionDigits: 0 })}</span>
          {:else if update.error}
            <button class="act retry" disabled={busy} onclick={installSquadron}>{t('common.retry')}</button>
          {:else}
            <button class="act go" disabled={busy} onclick={installSquadron}>{t('extensions.updateNow')}</button>
          {/if}
        </div>
        {#if open === SQUADRON && squadron.body}
          <div class="notes"><Markdown text={squadron.body} /></div>
        {/if}
        {#if rows.length}<span class="group">{t('settings.tabExtensions')}</span>{/if}
      {/if}
      {#each rows as row (row.e.id)}
        {@const step = steps[row.e.id]}
        {@const releases = notes(row)}
        <div class="row" class:joined={open === row.e.id}>
          <ExtensionLogo id={row.e.id} name={nameOf(row.e)} logo={row.e.logo} />
          <span class="what">
            <span class="l1">
              <strong>{nameOf(row.e)}</strong>
              <span class="version">{row.from}{' → '}<b>{row.entry.version}</b></span>
              <span class="pill {fromOf(row.e)}">{fromOf(row.e) === 'official' ? `✓ ${t('extensions.official')}` : `⇄ ${t('extensions.imported', { owner: ownerOf(row.entry.source) })}`}</span>
            </span>
            {#if step?.status === 'error'}
              <small class="bad">{step.error}</small>
            {:else if row.asking.length && step?.status !== 'done'}
              <small class="asks">{t('extensions.willAsk', { list: list(row.asking) })}</small>
            {/if}
          </span>
          {#if releases.length}
            <button class="log" aria-expanded={open === row.e.id} onclick={() => (open = open === row.e.id ? null : row.e.id)}>{t('extensions.whatsNew')}</button>
          {/if}
          {#if step?.status === 'busy'}
            <span class="act busy"><span class="spin" aria-hidden="true"></span>{t('extensions.updating')}</span>
          {:else if step?.status === 'done'}
            <span class="act done"><Icon name="check" />{t('extensions.updated')}</span>
          {:else if step?.status === 'error'}
            <button class="act retry" disabled={busy} onclick={() => one(row)}>{t('common.retry')}</button>
          {:else}
            <button class="act go" disabled={busy} onclick={() => one(row)}>{t('extensions.updateNow')}</button>
          {/if}
        </div>
        {#if open === row.e.id}
          <div class="notes">
            {#each releases as release (release.version)}
              <div class="release">
                <div class="head"><span class="mono">{release.version}</span><span class="date">{release.date}</span></div>
                <Markdown text={lines(release.notes)} />
              </div>
            {/each}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
  {#if asking.length}
    <p class="note">{t('extensions.newAccessNote', { count: asking.length, list: list(asking.map((r) => nameOf(r.e))) })}</p>
  {/if}
  <footer>
    <button class="btn btn-ghost manage" onclick={manage}><Icon name="grid" />{t('extensions.manageTitle')}</button>
    <span class="grow"></span>
    <button class="btn btn-secondary" onclick={onclose}>{t('common.close')}</button>
    {#if left.length || squadron}
      <button class="btn btn-primary" disabled={busy} onclick={all}>{squadron ? t('extensions.updateAllRestart') : t('extensions.updateAll')}</button>
    {/if}
  </footer>
</Dialog>

<style>
  .intro {
    margin: 0;
    padding: 0 22px;
    color: var(--text-dim);
    font-size: 13px;
  }

  .intro.bad {
    margin-top: 6px;
    color: var(--err-text);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: min(520px, calc(100vh - 300px));
    overflow-y: auto;
    padding: 14px 22px 18px;
  }

  .group {
    margin: 10px 0 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .group:first-child {
    margin-top: 0;
  }

  .brand {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 7px;
    background: var(--surface-3);
  }

  .triangle {
    width: 16px;
    height: 16px;
    background: var(--accent);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 10px 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .row.joined {
    border-radius: 6px 6px 0 0;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
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
    color: var(--text-dim);
    font-size: 12px;
  }

  .what small.asks {
    color: var(--accent-soft);
  }

  .what small.bad {
    color: var(--err-text);
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

  .log {
    flex: none;
    height: 30px;
    padding: 0 10px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .log:hover,
  .log[aria-expanded='true'] {
    background: var(--surface-3);
    color: var(--text);
  }

  .act {
    flex: none;
    width: 104px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: 4px;
    font-size: 13px;
  }

  .act.go {
    background: var(--surface-4);
    color: var(--text);
  }

  .act.go:hover:not(:disabled) {
    background: var(--surface-5);
  }

  .act:disabled {
    opacity: 0.5;
  }

  .act.busy {
    color: var(--text-dim);
  }

  .act.done {
    color: var(--ok-text);
  }

  .act.retry {
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
  }

  .spin {
    width: 12px;
    height: 12px;
    border: 2px solid var(--surface-5);
    border-top-color: var(--ok);
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .notes {
    margin-top: -4px;
    padding: 12px 14px 4px 58px;
    border-radius: 0 0 6px 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 12.5px;
  }

  .release + .release {
    margin-top: 10px;
  }

  .release .head {
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
  }

  .release .mono {
    font-family: var(--mono);
    font-weight: 600;
    color: var(--text);
  }

  .release .date {
    color: var(--text-faint);
  }

  .release :global(ul) {
    margin: 4px 0 8px;
    padding-left: 16px;
    color: var(--text-muted);
  }

  .note {
    margin: 0 22px 20px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--wait-note);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-muted);
    font-size: 12.5px;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  .grow {
    flex: 1;
  }

  .manage:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  footer .btn {
    height: 38px;
    padding: 0 16px;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 18px 22px 26px;
    text-align: center;
  }

  .seal {
    position: relative;
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    margin-bottom: 16px;
    border-radius: 50%;
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--done);
    animation: seal-in 0.45s cubic-bezier(0.3, 1.5, 0.6, 1) both;
  }

  .seal :global(path) {
    stroke-width: 2.4;
    stroke-dasharray: 22;
    stroke-dashoffset: 22;
    animation: draw 0.4s 0.3s ease-out forwards;
  }

  .seal::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    box-shadow: 0 0 0 1px var(--done);
    opacity: 0;
    animation: halo 0.9s 0.55s ease-out;
  }

  .hero h2 {
    margin: 0 0 4px;
    font-size: 16px;
    font-weight: 700;
    color: var(--text-strong);
  }

  .summary {
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .summary .mono {
    font-size: 12px;
    color: var(--text-2);
  }

  .crew {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 18px;
  }

  .crew > span {
    position: relative;
    display: flex;
    animation: rise 0.35s cubic-bezier(0.3, 1.4, 0.6, 1) both;
    animation-delay: calc(0.6s + var(--i) * 70ms);
  }

  .crew .brand {
    width: 30px;
    height: 30px;
    border-radius: 6px;
  }

  .crew .triangle {
    width: 13px;
    height: 13px;
  }

  .more {
    align-items: center;
    height: 30px;
    padding: 0 8px;
    border-radius: 6px;
    background: var(--surface-3);
    color: var(--text-2);
    font: 11.5px var(--mono);
  }

  .tick {
    position: absolute;
    right: -4px;
    bottom: -4px;
    display: grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--done);
    box-shadow: 0 0 0 2px var(--bg-main);
    color: var(--ink);
    animation: pop 0.3s cubic-bezier(0.3, 1.6, 0.6, 1) both;
    animation-delay: calc(0.8s + var(--i) * 70ms);
  }

  .tick :global(path) {
    stroke-width: 3.4;
  }

  @keyframes seal-in {
    from {
      opacity: 0;
      transform: scale(0.5);
    }
  }

  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }

  @keyframes halo {
    from {
      opacity: 0.8;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(1.8);
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.4);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spin,
    .seal,
    .seal::after,
    .crew > span,
    .tick {
      animation: none;
    }

    .seal :global(path) {
      animation: none;
      stroke-dashoffset: 0;
    }
  }
</style>
