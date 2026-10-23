<script lang="ts">
  import type { Snippet } from 'svelte'
  import { layout, MIN_PANEL, resizePanel, toggleFull, type PanelKind } from './layout.svelte'

  let {
    kind,
    size = kind,
    title,
    sub,
    actions,
    children,
    screen = false,
    wide = false,
    onclose,
  }: {
    kind: PanelKind
    size?: string
    title: string
    sub: Snippet
    actions?: Snippet
    children: Snippet
    screen?: boolean
    wide?: boolean
    onclose: () => void
  } = $props()

  let left = $state(0)
  $effect(() => {
    if (wide) left = panel.parentElement?.getBoundingClientRect().left ?? 0
  })

  const full = $derived(!!layout.full[kind])
  const width = $derived(layout.widths[size])

  let panel: HTMLElement
  const widest = () => (panel.parentElement?.clientWidth ?? window.innerWidth) - 360

  function drag(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const right = panel.getBoundingClientRect().right
    handle.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => (e.buttons ? resizePanel(size, right - e.clientX, widest()) : up())
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }

  function nudge(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') resizePanel(size, panel.offsetWidth + 16, widest())
    else if (event.key === 'ArrowRight') resizePanel(size, panel.offsetWidth - 16, widest())
  }
</script>

<section
  class="panel"
  class:full
  class:screen
  class:wide
  style:width={full || screen || wide ? null : `${width}px`}
  style:left={wide ? `${left}px` : null}
  aria-label={title}
  bind:this={panel}
>
  {#if !full}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the {title.toLowerCase()} panel"
      aria-valuenow={width}
      aria-valuemin={MIN_PANEL}
      tabindex="0"
      title="Drag to resize, double-click for full screen"
      onpointerdown={drag}
      onkeydown={nudge}
      ondblclick={() => toggleFull(kind)}
    ></div>
  {/if}
  <div class="top">
    <div class="title">
      <h2>{title}</h2>
      <span>{@render sub()}</span>
    </div>
    {@render actions?.()}
    <button
      class="icon"
      aria-label={full ? 'Exit full screen' : 'Full screen'}
      aria-pressed={full}
      title={full ? 'Exit full screen' : 'Full screen'}
      onclick={() => toggleFull(kind)}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
        {#if full}
          <path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7" />
        {:else}
          <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
        {/if}
      </svg>
    </button>
    <button class="icon" aria-label="Close panel" onclick={onclose}>×</button>
  </div>
  {@render children()}
</section>

<style>
  .panel {
    position: relative;
    min-width: 420px;
    flex: 0 1 auto;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .panel.full {
    position: absolute;
    inset: 0;
    z-index: 20;
    min-width: 0;
    border-left: none;
  }

  .panel.screen,
  .panel.wide {
    position: fixed;
    inset: 54px 0 0 0;
    z-index: 40;
    min-width: 0;
    border-left: none;
  }

  .panel.wide {
    border-left: 1px solid var(--border);
  }

  .resizer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    z-index: 2;
    width: 6px;
    cursor: col-resize;
  }

  .resizer:hover,
  .resizer:focus-visible,
  .resizer:active {
    background: var(--border-strong);
    outline: none;
  }

  .top {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 0 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
  }

  .title span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--text-dim);
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
