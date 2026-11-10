<script lang="ts">
  import { tick } from 'svelte'
  import { KIND_ICONS, kindOf } from './Attachment.svelte'
  import { TONES } from './FileCard.svelte'
  import { t } from './i18n/index.svelte'

  let { row, paths }: { row: HTMLElement; paths: string[] } = $props()

  let open = $state(false)
  let button = $state<HTMLElement>()
  const names = $derived(paths.map((p) => p.split('/').pop() ?? p).join(', '))
  const kinds = $derived([...new Set(paths.map(kindOf))].slice(0, 3))

  $effect(() => {
    row.classList.toggle('folded', !open)
  })

  async function fold() {
    open = false
    await tick()
    button?.scrollIntoView({ block: 'nearest' })
  }
</script>

{#if open}
  <button type="button" class="less" aria-expanded="true" onclick={fold}>
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 15l6-6 6 6" /></svg>
    {t('conversation.showFewerFiles')}
  </button>
{:else}
  <button type="button" class="rest" aria-expanded="false" bind:this={button} onclick={() => (open = true)}>
    <span class="deck" style:--n={kinds.length} aria-hidden="true">
      {#each kinds as kind (kind)}
        <span class="tile" style:--kind={TONES[kind]}><svg viewBox="0 0 24 24">{@html KIND_ICONS[kind]}</svg></span>
      {/each}
    </span>
    <span class="text">
      <span class="name">{t('conversation.moreFiles', { count: paths.length })}</span>
      <span class="meta">{names}</span>
    </span>
    <span class="go">
      {t('conversation.showAll')}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </span>
  </button>
{/if}

<style>
  .rest {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 15px;
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow: 0 0 0 1px var(--border-strong);
    text-align: left;
    line-height: 1.4;
    --under: var(--bg-main);
  }

  .rest:hover {
    background: var(--surface-1);
    --under: var(--surface-1);
  }

  .deck {
    position: relative;
    flex: none;
    width: calc(30px + (var(--n) - 1) * 12px);
    height: 36px;
  }

  .tile {
    position: absolute;
    top: 0;
    width: 30px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    background: color-mix(in srgb, var(--kind) 16%, var(--under));
    box-shadow:
      0 0 0 2px var(--under),
      inset 0 0 0 1px color-mix(in srgb, var(--kind) 35%, transparent);
    color: var(--kind);
  }

  .tile:nth-child(1) {
    left: 0;
    z-index: 3;
  }

  .tile:nth-child(2) {
    left: 12px;
    z-index: 2;
  }

  .tile:nth-child(3) {
    left: 24px;
    z-index: 1;
  }

  .tile svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-weight: 600;
    color: var(--text-strong);
  }

  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .go {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 11px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-2);
  }

  .rest:hover .go {
    color: var(--text-strong);
  }

  .less {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 4px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .less:hover {
    color: var(--text-strong);
  }

  svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
