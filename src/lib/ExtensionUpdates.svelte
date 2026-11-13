<script lang="ts">
  import { untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { ui } from './actions.svelte'
  import { errorText, list, locale, t } from './i18n/index.svelte'
  import { releasesOf, updatesOf, type CatalogEntry, type Extension } from './extensions'
  import { asks, catalog, commandLine, extensions, install } from './extensions.svelte'
  import { fromOf, nameOf } from './extension-pages.svelte'

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

  let steps = $state<Record<string, Step>>({})
  let open = $state<string | null>(null)

  const busy = $derived(Object.values(steps).some((s) => s.status === 'busy'))
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
  }

  const notes = (row: Row) => releasesOf(row.entry, row.from)

  const lines = (notes: Record<string, string[]>) => (notes[locale()] ?? notes.en).map((l) => `- ${l}`).join('\n')

  function manage() {
    onclose()
    ui.settings = 'extensions'
  }
</script>

<Dialog title={t('extensions.updatesTitle')} width={600} {onclose}>
  <p class="intro">{rows.length ? t('extensions.updatesIntro', { count: rows.length }) : t('extensions.noUpdates')}</p>
  <div class="list">
    {#each rows as row (row.e.id)}
      {@const step = steps[row.e.id]}
      {@const releases = notes(row)}
      <div class="row" class:joined={open === row.e.id}>
        <ExtensionLogo id={row.e.id} name={nameOf(row.e)} />
        <span class="what">
          <span class="l1">
            <strong>{nameOf(row.e)}</strong>
            <span class="version">{row.from}{' → '}<b>{row.entry.version}</b></span>
            <span class="pill {fromOf(row.e)}">{fromOf(row.e) === 'official' ? `✓ ${t('extensions.official')}` : `⇄ ${t('extensions.imported', { owner: row.entry.source.split('/')[0] })}`}</span>
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
  {#if asking.length}
    <p class="note">{t('extensions.newAccessNote', { count: asking.length, list: list(asking.map((r) => nameOf(r.e))) })}</p>
  {/if}
  <footer>
    <button class="link" onclick={manage}>{t('extensions.manageTitle')}</button>
    <span class="grow"></span>
    <button class="btn btn-secondary" onclick={onclose}>{t('common.close')}</button>
    <button class="btn btn-primary" disabled={busy || !left.length} onclick={all}>{t('extensions.updateAll')}</button>
  </footer>
</Dialog>

<style>
  .intro {
    margin: 0;
    padding: 0 22px;
    color: var(--text-dim);
    font-size: 13px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: min(520px, calc(100vh - 300px));
    overflow-y: auto;
    padding: 14px 22px 18px;
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

  .link {
    font-size: 12.5px;
    color: var(--accent-link);
  }

  .link:hover {
    color: var(--accent-soft);
  }

  footer .btn {
    height: 38px;
    padding: 0 16px;
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
