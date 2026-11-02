<script lang="ts">
  import type { PanelMode } from './layout.svelte'

  let { mode, size = 17 }: { mode: PanelMode; size?: number } = $props()
</script>

<svg width={size} height={Math.round((size * 16) / 20)} viewBox="0 0 20 16" fill="none" stroke="currentColor" stroke-width="1.2" aria-hidden="true">
  <rect x=".6" y=".6" width="18.8" height="14.8" rx="2" />
  {#if mode === 'slide'}
    <rect x="11" y="1.4" width="7.6" height="13.2" fill="currentColor" stroke="none" />
  {:else if mode === 'full'}
    <rect x="1.4" y="1.4" width="17.2" height="13.2" fill="currentColor" stroke="none" />
  {:else}
    <rect x="1.4" y="9" width="17.2" height="5.6" fill="currentColor" stroke="none" />
  {/if}
</svg>
