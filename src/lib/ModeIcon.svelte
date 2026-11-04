<script lang="ts">
  import type { PanelMode } from './layout.svelte'

  let { mode, size = 17 }: { mode: PanelMode; size?: number } = $props()

  const reach = $derived(mode === 'side' ? 13 : 28)
  const rows = $derived(mode === 'bottom' ? [4.5, 7.5] : mode === 'full' ? [] : [4.5, 7.5, 10.5, 13.5])
</script>

<svg class="mode" class:big={size > 30} width={size} height={Math.round((size * 20) / 32)} viewBox="0 0 32 20" fill="none" aria-hidden="true">
  <rect class="frame" x=".6" y=".6" width="30.8" height="18.8" rx="2.4" />
  {#each rows as y (y)}
    <path class="line" d="M4 {y}H{reach}" />
  {/each}
  {#if mode === 'slide'}
    <rect class="pane float" x="15" y="2.6" width="14.4" height="14.8" rx="1.4" />
  {:else if mode === 'side'}
    <rect class="pane" x="16.4" y="1.2" width="14.4" height="17.6" rx="1" />
  {:else if mode === 'bottom'}
    <rect class="pane" x="1.2" y="10" width="29.6" height="8.8" rx="1" />
  {:else}
    <rect class="pane" x="1.2" y="1.2" width="29.6" height="17.6" rx="1.8" />
  {/if}
</svg>

<style>
  .mode {
    flex: none;
  }

  .frame {
    stroke: currentColor;
    stroke-width: 1.2;
  }

  .line {
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    opacity: 0.45;
  }

  .pane {
    fill: currentColor;
  }

  .float {
    stroke: var(--bg-sidebar);
    stroke-width: 1.2;
  }

  .big .frame {
    stroke: var(--border-strong);
    stroke-width: 0.8;
    fill: var(--surface-1);
  }

  .big .line {
    stroke-width: 0.8;
  }

  .big .pane {
    fill: color-mix(in srgb, var(--accent) 30%, var(--surface-2));
    stroke: var(--accent);
    stroke-width: 0.8;
  }

  .big .float {
    filter: drop-shadow(-1.5px 1px 1.5px rgba(0, 0, 0, 0.6));
  }
</style>
