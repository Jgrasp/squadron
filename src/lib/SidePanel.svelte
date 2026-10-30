<script module lang="ts">
  let live = 0
  let closedAt = 0
</script>

<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import { layout, MIN_PANEL, resizePanel, toggleFull, type PanelKind } from './layout.svelte'
  import { t } from './i18n/index.svelte'
  import PanelButtons from './PanelButtons.svelte'

  let {
    kind,
    title,
    sub,
    actions,
    children,
    screen = false,
    wide = false,
    windowed = false,
    onclose,
  }: {
    kind: PanelKind
    size?: string
    title: string
    sub?: Snippet
    actions?: Snippet
    children: Snippet
    screen?: boolean
    wide?: boolean
    windowed?: boolean
    onclose: () => void
  } = $props()

  let left = $state(0)
  $effect(() => {
    if (wide) left = panel.parentElement?.getBoundingClientRect().left ?? 0
  })

  const full = $derived(!!layout.full.panel)
  const width = $derived(layout.widths.panel)

  let panel: HTMLElement
  const slide = live === 0 && performance.now() - closedAt > 150
  onMount(() => {
    live++
    return () => (live--, (closedAt = performance.now()))
  })
  const widest = () => panel.parentElement?.clientWidth ?? window.innerWidth

  function drag(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const right = panel.getBoundingClientRect().right
    handle.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => (e.buttons ? resizePanel('panel', right - e.clientX, widest()) : up())
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }

  function nudge(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') resizePanel('panel', panel.offsetWidth + 16, widest())
    else if (event.key === 'ArrowRight') resizePanel('panel', panel.offsetWidth - 16, widest())
  }
</script>

<section
  class="panel"
  class:slide
  class:leave={layout.leaving}
  class:full
  class:screen
  class:wide
  class:windowed
  style:width={full || screen || wide || windowed ? null : `${width}px`}
  style:left={wide ? `${left}px` : null}
  aria-label={title}
  bind:this={panel}
>
  {#if !full && !windowed}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={t('shell.resizePanel', { title: title.toLowerCase() })}
      aria-valuenow={width}
      aria-valuemin={MIN_PANEL}
      tabindex="0"
      title={t('shell.dragToResize')}
      onpointerdown={drag}
      onkeydown={nudge}
      ondblclick={toggleFull}
    ></div>
  {/if}
  {#if sub}
  <div class="top">
    <div class="title">
      <h2>{title}</h2>
      <span>{@render sub?.()}</span>
    </div>
    {@render actions?.()}
    <PanelButtons {windowed} {onclose} />
  </div>
  {/if}
  {@render children()}
</section>

<style>
  .panel {
    position: absolute;
    top: var(--panel-top, 0);
    right: 0;
    bottom: 0;
    z-index: 20;
    min-width: min(420px, 100%);
    max-width: 100%;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border-strong);
    background: var(--bg-sidebar);
    box-shadow: -18px 0 40px rgba(0, 0, 0, 0.45);
  }

  .panel.slide {
    animation: arrive 0.38s cubic-bezier(0.22, 1, 0.36, 1);
  }

  @keyframes arrive {
    from {
      transform: translateX(100%);
    }
  }

  .panel.leave {
    animation: depart 0.28s cubic-bezier(0.4, 0, 1, 1) forwards;
  }

  @keyframes depart {
    to {
      transform: translateX(100%);
    }
  }

  .panel.full {
    left: 0;
    min-width: 0;
    border-left: none;
    box-shadow: none;
  }

  .panel.screen,
  .panel.windowed {
    position: fixed;
    inset: 0;
    min-width: 0;
    border-left: none;
  }

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
</style>
