<script lang="ts">
  import { tick } from 'svelte'
  import Icon from './Icon.svelte'
  import ItemIcon from './ItemIcon.svelte'
  import { extensionItem } from './extensions.svelte'
  import { all, detailOf, hidden, labelOf, place, sortable, toggle } from './layout-slots.svelte'
  import { SLOTS, type Slot } from './slots'
  import { t } from './i18n/index.svelte'

  let { slot }: { slot: Slot } = $props()

  const ICONS: Record<string, string> = { artifacts: 'artifact', pulls: 'pull', runs: 'pipeline' }
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

<div class="options">
  <div class="title"><strong>{t(SLOTS[slot].title)}</strong><small>{t(SLOTS[slot].hint)}</small></div>
  <div class="list" role="list" aria-label={t(SLOTS[slot].title)} bind:this={box} use:sortable={{ enabled: true, onplace: (id, index) => place(slot, id, index) }}>
    {#each all(slot) as id, i (id)}
      {@const off = hidden(slot, id)}
      {@const item = extensionItem(slot, id)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        role="listitem"
        class="row"
        class:off
        data-item={id}
        tabindex="0"
        aria-label={t(off ? 'settings.slotItemHidden' : 'settings.slotItem', { item: labelOf(slot, id) })}
        onkeydown={(e) => keys(e, id, i)}
      >
        <span class="grip" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i><i></i></span>
        {#if bar}
          <span class="icon">{#if item}<ItemIcon icon={item.icon} extension={item.extension} fallback="plugins" />{:else}<Icon name={ICONS[id] ?? id} />{/if}</span>
        {/if}
        <span class="label">
          <strong>{labelOf(slot, id)}</strong>
          {#if !bar && detailOf(slot, id)}<small>{detailOf(slot, id)}</small>{/if}
        </span>
        {#if SLOTS[slot].locked?.includes(id)}
          <span class="lock">{t('settings.slotLocked')}</span>
        {:else}
          <input type="checkbox" class="switch" data-keep checked={!off} aria-label={labelOf(slot, id)} onchange={() => toggle(slot, id)} />
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }

  .title strong {
    font-size: 13px;
    font-weight: 600;
  }

  .title small {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-dim);
  }

  .list {
    display: flex;
    flex-direction: column;
  }

  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 6px 14px 6px 8px;
    background: var(--surface-1);
    cursor: grab;
    user-select: none;
    touch-action: none;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row:last-child {
    border-radius: 0 0 6px 6px;
  }

  .row:hover {
    background: var(--surface-2);
  }

  .row:focus-visible {
    outline: 2px solid var(--border-focus);
    outline-offset: -2px;
  }

  .row:global(.dragging) {
    z-index: 2;
    cursor: grabbing;
    background: var(--surface-2);
    filter: drop-shadow(0 6px 14px rgba(0, 0, 0, 0.5));
  }

  .grip {
    width: 14px;
    flex: none;
    display: grid;
    grid-template-columns: repeat(2, 3px);
    gap: 3px 4px;
    justify-content: center;
  }

  .grip i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .icon {
    width: 26px;
    height: 26px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .label strong {
    font-size: 13px;
    font-weight: 600;
  }

  .label small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .off .icon,
  .off .label {
    opacity: 0.45;
  }

  .lock {
    flex: none;
    font-size: 12px;
    color: var(--text-faint);
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }
</style>
