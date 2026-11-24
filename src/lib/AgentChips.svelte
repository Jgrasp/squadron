<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import { chipAt, chipClock, chipMotion } from './chips.svelte'
  import ChipKey from './ChipKey.svelte'
  import { brand } from './extension-pages.svelte'
  import { t } from './i18n/index.svelte'
  import type { Chip } from './newAgentDraft'

  let { chips, turns, onmore }: { chips: Chip[]; turns: boolean; onmore: (anchor: HTMLElement) => void } = $props()

  let paused = $state(false)
  let shown = $state(0)
  let was = $state(-1)
  const motion = $derived(chipMotion())
  const at = $derived(
    chipAt(chips.length, chipClock.tick, { err: chips.findIndex((c) => c.tone === 'err'), paused, shown, turns: turns && motion !== 'off' }),
  )

  $effect(() => {
    const next = at
    untrack(() => {
      if (next === shown) return
      was = shown
      shown = next
    })
  })

  function open(e: MouseEvent, url?: string) {
    if (!url) return
    e.stopPropagation()
    invoke('open_link', { url })
  }
</script>

<span class="chips" role="group" onpointerenter={() => (paused = true)} onpointerleave={() => (paused = false)}>
  <span class="slot {motion}">
    {#each chips as chip, i (i)}
      <span
        class="chip"
        class:on={i === shown}
        class:out={i === was}
        class:open={chip.url}
        role={chip.url ? 'link' : undefined}
        aria-hidden={i !== shown}
        title="{brand(chip.extension).name} · {chip.title}"
        onclick={(e) => open(e, chip.url)}><ChipKey {chip} /></span
      >
    {/each}
  </span>
  {#if chips.length > 1}
    <span
      class="more"
      role="button"
      tabindex="-1"
      aria-label={t('shell.chipsAll', { count: chips.length })}
      title={t('shell.chipsAll', { count: chips.length })}
      data-menu
      onclick={(e) => (e.stopPropagation(), onmore(e.currentTarget))}
      onkeydown={(e) => e.key === 'Enter' && (e.stopPropagation(), onmore(e.currentTarget))}>+{chips.length - 1}</span
    >
  {/if}
</span>

<style>
  .chips {
    float: right;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: 8px;
  }

  .slot {
    display: inline-grid;
    overflow: hidden;
  }

  .slot.down,
  .slot.up {
    perspective: 200px;
  }

  .chip {
    grid-area: 1 / 1;
    justify-self: end;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font: 500 11px var(--mono);
    color: var(--remote-text);
    visibility: hidden;
    opacity: 0;
  }

  .chip.on {
    visibility: visible;
    opacity: 1;
  }

  .chip.open.on:hover {
    text-decoration: underline;
  }

  .fade .chip {
    transition:
      opacity 0.45s,
      visibility 0s 0.45s;
  }

  .fade .chip.on {
    transition: opacity 0.45s;
  }

  .down {
    --dir: 1;
  }

  .up {
    --dir: -1;
  }

  .down .chip,
  .up .chip {
    transform: translateY(calc(var(--dir) * -100%)) rotateX(calc(var(--dir) * 70deg));
    transition:
      transform 0.45s cubic-bezier(0.3, 0.7, 0.3, 1),
      opacity 0.45s,
      visibility 0s 0.45s;
  }

  .down .chip.on,
  .up .chip.on {
    transform: none;
    transition:
      transform 0.45s cubic-bezier(0.3, 0.7, 0.3, 1),
      opacity 0.45s;
  }

  .down .chip.out,
  .up .chip.out {
    transform: translateY(calc(var(--dir) * 100%)) rotateX(calc(var(--dir) * -70deg));
  }

  .more {
    height: 16px;
    padding: 0 5px;
    display: inline-grid;
    place-items: center;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font: 600 11px var(--mono);
  }

  .more:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }
</style>
