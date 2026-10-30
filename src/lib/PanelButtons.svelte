<script lang="ts">
  import { layout, toggleFull } from './layout.svelte'
  import { t } from './i18n/index.svelte'

  let { windowed = false, onclose }: { windowed?: boolean; onclose: () => void } = $props()

  const full = $derived(!!layout.full.panel)
</script>

{#if !windowed}
  <button
    class="icon"
    aria-label={full ? t('shell.exitFullScreen') : t('shell.fullScreen')}
    aria-pressed={full}
    title={full ? t('shell.exitFullScreen') : t('shell.fullScreen')}
    onclick={toggleFull}
  >
    <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
      {#if full}
        <path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7" />
      {:else}
        <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
      {/if}
    </svg>
  </button>
{/if}
<button class="icon" aria-label={t('shell.closePanel')} title={t('shell.closePanel')} onclick={onclose}>×</button>

<style>
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

  .icon:hover,
  .icon[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
