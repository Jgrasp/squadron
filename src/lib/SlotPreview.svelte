<script lang="ts">
  import { tick } from 'svelte'
  import Icon from './Icon.svelte'
  import { all, detailOf, hidden, labelOf, place, sortable, toggle } from './layout-slots.svelte'
  import { SLOTS, type Slot } from './slots'

  let { slot }: { slot: Slot } = $props()

  const bar = $derived(slot === 'agent.tools' || slot === 'agent.info')
  let box: HTMLElement

  async function keys(event: KeyboardEvent, id: string, index: number) {
    const step = { ArrowLeft: -1, ArrowUp: -1, ArrowRight: 1, ArrowDown: 1 }[event.key]
    if (!event.altKey || !step) return
    event.preventDefault()
    place(slot, id, index + step)
    await tick()
    box.querySelector<HTMLElement>(`[data-item="${id}"]`)?.focus()
  }
</script>

<div class="preview" class:bar class:menu={!bar} role="list" aria-label={SLOTS[slot].title} bind:this={box} use:sortable={{ slot, enabled: true }}>
  {#each all(slot) as id, i (id)}
    {@const off = hidden(slot, id)}
    {@const locked = SLOTS[slot].locked?.includes(id)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      role="listitem"
      class="entry"
      class:off
      data-item={id}
      tabindex="0"
      aria-label="{labelOf(slot, id)}{off ? ', hidden' : ''}. Drag it, or Alt and the arrows to move it."
      onkeydown={(e) => keys(e, id, i)}
    >
      {#if bar}
        <Icon name={id} />
        <span>{labelOf(slot, id)}</span>
      {:else}
        <strong>{labelOf(slot, id)}</strong>
        <small>{detailOf(slot, id)}</small>
      {/if}
      {#if !locked}
        <button class="eye" data-keep aria-label="{off ? 'Show' : 'Hide'} {labelOf(slot, id)}" title={off ? 'Show' : 'Hide'} onclick={() => toggle(slot, id)}>
          <Icon name={off ? 'eye-off' : 'eye'} size={12} />
        </button>
      {/if}
    </div>
  {/each}
</div>

<style>
  .preview {
    display: flex;
    align-self: flex-start;
    padding: 10px;
    border-radius: 6px;
    outline: 1px dashed var(--accent);
    outline-offset: 2px;
  }

  .bar {
    flex-wrap: wrap;
    gap: 10px 6px;
    background: var(--bg-main);
  }

  .menu {
    width: 300px;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    background: var(--menu);
  }

  .entry {
    position: relative;
    cursor: grab;
    user-select: none;
    touch-action: none;
    outline-offset: 2px;
  }

  .bar .entry {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font-weight: 500;
    white-space: nowrap;
  }

  .menu .entry {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 9px 34px 9px 10px;
    border-radius: 4px;
    background: var(--menu);
  }

  .menu .entry:hover {
    background: var(--menu-hover);
  }

  .menu strong {
    font-weight: 600;
  }

  .menu small {
    font-size: 12px;
    color: var(--text-muted);
  }

  .entry.off > :not(.eye) {
    opacity: 0.35;
  }

  .entry:global(.dragging) {
    z-index: 2;
    cursor: grabbing;
    filter: drop-shadow(0 6px 14px rgba(0, 0, 0, 0.5));
  }

  .menu .entry:global(.dragging) {
    background: var(--menu-hover);
  }

  .eye {
    position: absolute;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .bar .eye {
    top: -8px;
    right: -6px;
  }

  .menu .eye {
    top: 50%;
    right: 8px;
    transform: translateY(-50%);
  }
</style>
