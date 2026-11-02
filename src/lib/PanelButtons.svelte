<script lang="ts">
  import { displayOf, setDisplay, type PanelKind } from './layout.svelte'
  import { MODES, modeLabel } from './panels'
  import { t } from './i18n/index.svelte'
  import ModeIcon from './ModeIcon.svelte'

  let { kind, windowed = false, onclose }: { kind: PanelKind; windowed?: boolean; onclose: () => void } = $props()

  const mode = $derived(displayOf(kind).mode)
</script>

{#if !windowed}
  <div class="modes" role="group" aria-label={t('shell.panelPlacement')}>
    {#each MODES as each (each)}
      <button aria-pressed={mode === each} aria-label={t(modeLabel[each])} title={t(modeLabel[each])} onclick={() => setDisplay(kind, { mode: each })}>
        <ModeIcon mode={each} />
      </button>
    {/each}
  </div>
{/if}
<button class="icon" aria-label={t('shell.closePanel')} title={t('shell.closePanel')} onclick={onclose}>×</button>

<style>
  .modes {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 6px;
    background: var(--surface-2);
  }

  .modes button {
    width: 30px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .modes button:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .modes button[aria-pressed='true'] {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }

  .icon:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }
</style>
