<script lang="ts">
  import Dialog from './Dialog.svelte'
  import { ui } from './actions.svelte'
  import { t } from './i18n/index.svelte'
  import { ownerOf, type CatalogEntry } from './extensions'
  import { asks, install } from './extensions.svelte'
  import { attempt, named, work } from './extension-pages.svelte'

  let { entry, onclose }: { entry: CatalogEntry; onclose: () => void } = $props()

  const lines = $derived(asks(entry))

  function close() {
    work.error = ''
    onclose()
  }

  async function confirm() {
    if (await attempt(() => install(entry), entry.id)) {
      onclose()
      ui.settings = `ext:${entry.id}`
    }
  }
</script>

<Dialog title={t('extensions.installTitle', { name: named(entry.name) })} width={520} onclose={close}>
  <div class="box">
    <p class="sub"><span class="mono">{entry.version}</span> · {entry.official ? t('extensions.fromOfficial', { source: entry.source }) : t('extensions.fromImported', { source: entry.source })}</p>
    {#if entry.description}<p class="text">{named(entry.description)}</p>{/if}
    {#if !entry.official}<p class="warn">{t('extensions.notReviewed', { owner: ownerOf(entry.source) })}</p>{/if}
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

<style>
  .box {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 0 22px 20px;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
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
</style>
