<script lang="ts" module>
  export type Option = { value: string; label: string; detail?: string; disabled?: boolean }
</script>

<script lang="ts">
  let { value, options, label, marked = false, onpick }: { value: string; options: Option[]; label: string; marked?: boolean; onpick: (value: string) => void } = $props()

  const id = $props.id()
  let root: HTMLSpanElement
  let list = $state<HTMLDivElement>()
  let open = $state(false)
  let active = $state(0)

  const current = $derived(options.find((o) => o.value === value))

  function show() {
    active = Math.max(0, options.findIndex((o) => o.value === value))
    open = true
  }

  function pick(option: Option) {
    if (option.disabled) return
    open = false
    if (option.value !== value) onpick(option.value)
  }

  function step(by: number) {
    let next = active
    for (let i = 0; i < options.length; i++) {
      next = (next + by + options.length) % options.length
      if (!options[next].disabled) break
    }
    active = next
  }

  function keys(event: KeyboardEvent) {
    if (!open) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(event.key)) {
        event.preventDefault()
        show()
      }
      return
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      step(event.key === 'ArrowDown' ? 1 : -1)
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      pick(options[active])
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      open = false
    } else if (event.key === 'Tab') open = false
  }

  $effect(() => {
    if (open) list?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
  })
</script>

<svelte:window onclick={(e) => open && !root.contains(e.target as Node) && (open = false)} />

<span class="select" bind:this={root}>
  <button
    type="button"
    class="trigger"
    class:marked
    class:inherit={value === ''}
    role="combobox"
    aria-label={label}
    aria-expanded={open}
    aria-controls="{id}-list"
    aria-activedescendant={open ? `${id}-${active}` : undefined}
    onclick={() => (open ? (open = false) : show())}
    onkeydown={keys}
  >
    <span class="text">{current?.label ?? value}</span>
    <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
  </button>
  {#if open}
    <div class="list" id="{id}-list" role="listbox" aria-label={label} bind:this={list}>
      {#each options as option, i (option.value)}
        <button
          type="button"
          id="{id}-{i}"
          role="option"
          tabindex="-1"
          aria-selected={option.value === value}
          aria-disabled={option.disabled}
          data-active={i === active}
          class:inherit={option.value === ''}
          onmousemove={() => !option.disabled && (active = i)}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => pick(option)}
        >
          <span class="name">{option.label}{#if option.value === value}<svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>{/if}</span>
          {#if option.detail}<span class="detail">{option.detail}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</span>

<style>
  .select {
    position: relative;
    display: block;
    min-width: 0;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    text-align: left;
  }

  .trigger:hover {
    background: var(--surface-3);
  }

  .trigger[aria-expanded='true'] {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .trigger.marked {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
    font-weight: 600;
  }

  .trigger.marked[aria-expanded='true'] {
    box-shadow: inset 0 0 0 2px var(--accent);
  }

  .trigger.inherit .text {
    color: var(--text-muted);
    font-style: italic;
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .trigger svg {
    opacity: 0.7;
  }

  .list {
    position: absolute;
    z-index: 30;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 288px;
    overflow-y: auto;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .list button {
    display: flex;
    flex-direction: column;
    gap: 1px;
    width: 100%;
    min-height: 34px;
    padding: 7px 10px;
    border-radius: 4px;
    color: var(--text-strong);
    text-align: left;
  }

  .list button[data-active='true'] {
    background: var(--menu-hover);
  }

  .list button[aria-disabled='true'] {
    opacity: 0.4;
    cursor: default;
  }

  .list button.inherit .name {
    color: var(--text-muted);
    font-style: italic;
  }

  .name {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .list button[aria-selected='true'] .name {
    color: var(--accent-soft);
    font-weight: 600;
  }

  .detail {
    font-size: 12px;
    color: var(--text-dim);
  }
</style>
