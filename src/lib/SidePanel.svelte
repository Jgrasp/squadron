<script module lang="ts">
  const live = new Set<HTMLElement>()
</script>

<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import { cubicIn, quintOut } from 'svelte/easing'
  import { displayOf, MIN_PANEL, openPanel, requestedPanel, resizePanel, resizeShare, setDisplay, type PanelKind } from './layout.svelte'
  import { projects } from './projects.svelte'
  import { covers } from './panels'
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

  const display = $derived(displayOf(kind))
  const mode = $derived(screen || wide || windowed ? null : display.mode)

  let panel: HTMLElement
  onMount(() => {
    for (const other of live) other.style.zIndex = '20'
    live.add(panel)
    panel.style.zIndex = '21'
    return () => live.delete(panel)
  })

  const move = (axis: string) => (t: number) => `transform: translate${axis}(${(1 - t) * 100}%)`

  function arrive(_: Element) {
    if (windowed || !requestedPanel(kind)) return { duration: 0 }
    return { duration: 380, easing: quintOut, css: move(mode === 'bottom' ? 'Y' : 'X') }
  }

  function depart(_: Element) {
    if (windowed) return { duration: 0 }
    const next = openPanel(projects.current)
    if (next && next !== kind && mode && !panel.parentElement?.closest('.panel') && covers(displayOf(next), display)) return { duration: 380 }
    return { duration: 280, easing: cubicIn, css: move(mode === 'bottom' ? 'Y' : 'X') }
  }

  const host = () => panel.parentElement!
  const widest = () => host().clientWidth || window.innerWidth
  const room = () => {
    const box = host().getBoundingClientRect()
    return box.height - (parseFloat(getComputedStyle(host()).getPropertyValue('--panel-top')) || 0)
  }

  function drag(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const box = panel.getBoundingClientRect()
    const space = room()
    handle.setPointerCapture(event.pointerId)
    host().classList.add('resizing')
    const resize = (e: PointerEvent) =>
      mode === 'bottom' ? resizeShare(kind, ((box.bottom - e.clientY) / space) * 100) : resizePanel(kind, box.right - e.clientX, widest())
    const step = (e: PointerEvent) => (e.buttons ? resize(e) : up())
    const up = () => {
      host().classList.remove('resizing')
      handle.removeEventListener('pointermove', step)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', step)
    handle.addEventListener('pointerup', up)
  }

  function nudge(event: KeyboardEvent) {
    if (mode === 'bottom' && event.key === 'ArrowUp') resizeShare(kind, display.height + 2)
    else if (mode === 'bottom' && event.key === 'ArrowDown') resizeShare(kind, display.height - 2)
    else if (mode === 'slide' && event.key === 'ArrowLeft') resizePanel(kind, panel.offsetWidth + 16, widest())
    else if (mode === 'slide' && event.key === 'ArrowRight') resizePanel(kind, panel.offsetWidth - 16, widest())
  }
</script>

<section
  class="panel"
  class:slide={mode === 'slide'}
  class:full={mode === 'full'}
  class:bottom={mode === 'bottom'}
  class:screen
  class:wide
  class:windowed
  style:width={mode === 'slide' ? `${display.width}px` : null}
  style:height={mode === 'bottom' ? `calc((100% - var(--panel-top, 0px)) * ${display.height / 100})` : null}
  style:left={wide ? `${left}px` : null}
  aria-label={title}
  bind:this={panel}
  in:arrive|global
  out:depart
>
  {#if mode === 'slide' || mode === 'bottom'}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="resizer"
      role="separator"
      aria-orientation={mode === 'bottom' ? 'horizontal' : 'vertical'}
      aria-label={t('shell.resizePanel', { title: title.toLowerCase() })}
      aria-valuenow={mode === 'bottom' ? display.height : display.width}
      aria-valuemin={mode === 'bottom' ? 20 : MIN_PANEL}
      tabindex="0"
      title={t('shell.dragToResize')}
      onpointerdown={drag}
      onkeydown={nudge}
      ondblclick={() => setDisplay(kind, { mode: 'full' })}
    ></div>
  {/if}
  {#if sub}
  <div class="top">
    <div class="title">
      <h2>{title}</h2>
      <span>{@render sub?.()}</span>
    </div>
    {@render actions?.()}
    <PanelButtons {kind} {windowed} {onclose} />
  </div>
  {/if}
  {@render children()}
</section>

<style>
  .panel {
    position: absolute;
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);
  }

  .panel.slide {
    top: var(--panel-top, 0);
    right: 0;
    bottom: 0;
    min-width: min(420px, 100%);
    max-width: 100%;
    border-left: 1px solid var(--border-strong);
    box-shadow: -18px 0 40px rgba(0, 0, 0, 0.45);
  }

  .panel.full {
    top: var(--panel-top, 0);
    right: 0;
    bottom: 0;
    left: 0;
  }

  .panel.bottom {
    right: 0;
    bottom: 0;
    left: 0;
    border-top: 1px solid var(--border-strong);
    box-shadow: 0 -14px 34px rgba(0, 0, 0, 0.35);
  }

  .panel.screen,
  .panel.windowed {
    position: fixed;
    inset: 0;
  }

  .panel.wide {
    position: fixed;
    inset: 54px 0 0 0;
    z-index: 40 !important;
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

  .bottom .resizer {
    top: -4px;
    right: 0;
    bottom: auto;
    left: 0;
    width: auto;
    height: 6px;
    cursor: row-resize;
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
