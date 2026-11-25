<script lang="ts">
  import { untrack } from 'svelte'
  import { t } from './i18n/index.svelte'
  import { hint } from './settings.svelte'
  import {
    CHAPTERS,
    STOPS,
    back,
    bodyOf,
    chapterName,
    closeEnd,
    current,
    go,
    hasOwnProjects,
    leave,
    next,
    restartTour,
    skipChapter,
    titleOf,
    tour,
  } from './tour.svelte'

  type Rect = { x: number; y: number; w: number; h: number }

  const PAD = 6
  const GAP = 14

  let rect = $state<Rect | null>(null)
  let card = $state<HTMLElement>()
  let place = $state({ left: 0, top: 0, side: 'center' as 'below' | 'above' | 'left' | 'right' | 'center', arrow: 0 })
  let view = $state({ w: innerWidth, h: innerHeight })

  const stop = $derived(tour.on ? current() : undefined)
  const chapters = $derived(CHAPTERS.filter((ch) => STOPS.some((s) => s.ch === ch)))
  const inChapter = $derived(stop ? STOPS.filter((s) => s.ch === stop.ch) : [])

  function measure() {
    view = { w: innerWidth, h: innerHeight }
    const el = stop?.el ? document.querySelector(stop.el) : null
    const r = el?.getBoundingClientRect()
    const next = r && r.width && r.height ? { x: r.left - PAD, y: r.top - PAD, w: r.width + PAD * 2, h: r.height + PAD * 2 } : null
    if (JSON.stringify(next) !== JSON.stringify(rect)) rect = next
    if (card) position(card.offsetWidth, card.offsetHeight)
  }

  function position(w: number, h: number) {
    const W = view.w
    const H = view.h
    const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v))
    if (!rect) {
      place = { left: (W - w) / 2, top: (H - h) / 2, side: 'center', arrow: 0 }
      return
    }
    const { x, y, w: rw, h: rh } = rect
    let side: typeof place.side
    if (rh > 300 && x + rw + GAP + w < W - 8) side = 'right'
    else if (rh > 300 && x - GAP - w > 8) side = 'left'
    else if (y + rh + GAP + h < H - 8) side = 'below'
    else if (y - GAP - h > 8) side = 'above'
    else if (x + rw + GAP + w < W - 8) side = 'right'
    else if (x - GAP - w > 8) side = 'left'
    else side = 'center'
    if (side === 'center') {
      place = { left: (W - w) / 2, top: (H - h) / 2, side, arrow: 0 }
      return
    }
    if (side === 'below' || side === 'above') {
      const left = clamp(x + rw / 2 - w / 2, 12, W - w - 12)
      place = { left, top: side === 'below' ? y + rh + GAP : y - GAP - h, side, arrow: clamp(x + rw / 2 - left - 5, 14, w - 24) }
    } else {
      const top = clamp(y + rh / 2 - h / 2, 12, H - h - 12)
      place = { left: side === 'right' ? x + rw + GAP : x - GAP - w, top, side, arrow: clamp(y + rh / 2 - top - 5, 14, h - 24) }
    }
  }

  $effect(() => {
    if (!tour.on) return
    let frame = requestAnimationFrame(function loop() {
      measure()
      frame = requestAnimationFrame(loop)
    })
    return () => cancelAnimationFrame(frame)
  })

  $effect(() => {
    if (!tour.toast) return
    const timer = setTimeout(() => (tour.toast = ''), 8000)
    return () => clearTimeout(timer)
  })

  $effect(() => {
    void tour.i
    untrack(() => card?.querySelector<HTMLElement>('.btn-primary')?.focus({ preventScroll: true }))
  })

  function keys(event: KeyboardEvent) {
    if (!tour.on && !tour.end) return
    if ((event.target as HTMLElement).closest?.('.tour-card, .tour-end') && (event.key === 'Enter' || event.key === ' ' || event.key === 'Tab')) return
    event.stopPropagation()
    if (tour.end) {
      if (event.key === 'Escape') closeEnd()
      return
    }
    if (event.key === 'ArrowRight' && !tour.confirm) (event.preventDefault(), next())
    else if (event.key === 'ArrowLeft' && !tour.confirm) (event.preventDefault(), back())
    else if (event.key === 'Escape') (event.preventDefault(), (tour.confirm = !tour.confirm))
  }

  const fill = (ch: string) => {
    const idx = STOPS.map((s, i) => (s.ch === ch ? i : -1)).filter((i) => i >= 0)
    if (idx[0] > tour.i) return 0
    if (idx[idx.length - 1] <= tour.i) return 100
    return ((tour.i - idx[0] + 1) / idx.length) * 100
  }

  const seenIn = (ch: string) => STOPS.filter((s) => s.ch === ch && tour.seen.includes(s.id)).length
  const firstMissing = (ch: string) => STOPS.findIndex((s) => s.ch === ch && !tour.seen.includes(s.id))
  const check = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--done)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>'

  function replayChapter(ch: string) {
    restartTourAt(firstMissing(ch))
  }

  function restartTourAt(i: number) {
    tour.end = false
    tour.on = true
    go(Math.max(0, i))
  }
</script>

<svelte:window onkeydowncapture={keys} onresize={measure} />

{#if tour.on && stop}
  <div class="tour" role="presentation">
    {#if rect && !tour.confirm}
      <div class="spot" style:left="{rect.x}px" style:top="{rect.y}px" style:width="{rect.w}px" style:height="{rect.h}px"></div>
    {:else}
      <div class="scrim"></div>
    {/if}
    {#if rect && stop.you && !tour.allowed && !tour.confirm}
      <div class="block" style:left="0" style:top="0" style:width="100%" style:height="{rect.y}px"></div>
      <div class="block" style:left="0" style:top="{rect.y + rect.h}px" style:width="100%" style:bottom="0"></div>
      <div class="block" style:left="0" style:top="{rect.y}px" style:width="{rect.x}px" style:height="{rect.h}px"></div>
      <div class="block" style:left="{rect.x + rect.w}px" style:top="{rect.y}px" style:right="0" style:height="{rect.h}px"></div>
    {:else}
      <div class="block" style:inset="0"></div>
    {/if}

    <section class="tour-card" bind:this={card} aria-label={titleOf(stop)} style:left="{place.left}px" style:top="{place.top}px">
      {#if place.side !== 'center'}
        <span
          class="arrow {place.side}"
          style:left={place.side === 'below' || place.side === 'above' ? `${place.arrow}px` : place.side === 'right' ? '-5px' : undefined}
          style:right={place.side === 'left' ? '-5px' : undefined}
          style:top={place.side === 'left' || place.side === 'right' ? `${place.arrow}px` : place.side === 'below' ? '-5px' : undefined}
          style:bottom={place.side === 'above' ? '-5px' : undefined}
        ></span>
      {/if}
      {#if tour.confirm}
        <div class="head"><span class="chapter">{t('tour.leave')}</span></div>
        <h3>{t('tour.leaveTitle')}</h3>
        <p>{t('tour.leaveBody', { n: tour.i + 1, total: STOPS.length })}</p>
        <div class="foot spaced">
          <span class="grow"></span>
          <button class="btn btn-ghost" onclick={() => (tour.confirm = false)}>{t('tour.keepGoing')}</button>
          <button class="btn btn-primary" onclick={leave}>{t('tour.leave')}</button>
        </div>
      {:else}
        <div class="head">
          <span class="chapter">{chapterName(stop.ch)}<span>{inChapter.indexOf(stop) + 1}/{inChapter.length}</span></span>
          <span class="grow"></span>
          <span class="count">{t('tour.count', { n: tour.i + 1, total: STOPS.length })}</span>
          <button class="x" aria-label={t('tour.leaveAria')} onclick={() => (tour.confirm = true)}>×</button>
        </div>
        <h3>{titleOf(stop)}</h3>
        <p>{bodyOf(stop)}</p>
        {#if stop.you}
          <p class="try">{#if tour.allowed}{@html check} {t('tour.permDone')}{:else}{t('tour.permTry')}{/if}</p>
        {/if}
        {#if stop.key}
          <div class="keys">
            {#each hint(stop.key).split(/\s*\+\s*|\s+/).filter(Boolean) as part (part)}<kbd>{part}</kbd>{/each}
            {#if stop.id === 'copper'}<span>{t('tour.copperKeys')}</span>{/if}
          </div>
        {/if}
        <div class="bar" role="group" aria-label={t('tour.count', { n: tour.i + 1, total: STOPS.length })}>
          {#each chapters as ch (ch)}
            {@const n = STOPS.filter((s) => s.ch === ch).length}
            <button
              class:cur={ch === stop.ch}
              style:flex={n}
              title="{chapterName(ch)} · {t('tour.stops', { count: n })}"
              aria-label={chapterName(ch)}
              onclick={() => go(STOPS.findIndex((s) => s.ch === ch))}><i style:width="{fill(ch)}%"></i></button
            >
          {/each}
        </div>
        <div class="foot">
          {#if stop.ch !== chapters[chapters.length - 1]}<button class="link" onclick={skipChapter}>{t('tour.skipChapter')}</button>{/if}
          <span class="grow"></span>
          <button class="btn btn-ghost" disabled={tour.i === 0} onclick={back}>{t('tour.back')}</button>
          <button class="btn btn-primary" onclick={next}>{tour.i === STOPS.length - 1 ? t('tour.finish') : t('tour.next')}</button>
        </div>
      {/if}
    </section>
  </div>
{/if}

{#if tour.end}
  {@const seen = STOPS.filter((s) => tour.seen.includes(s.id)).length}
  <div class="tour">
    <div class="scrim"></div>
    <section class="tour-end" aria-label={t('tour.endTitle')}>
      <div class="seal">{@html check.replace('width="14" height="14"', 'width="30" height="30"')}</div>
      <h3>{t('tour.endTitle')}</h3>
      <p>{t(hasOwnProjects() ? 'tour.endBack' : 'tour.endFresh', { seen, total: STOPS.length })}</p>
      <div class="list">
        {#each CHAPTERS as ch (ch)}
          {@const n = STOPS.filter((s) => s.ch === ch).length}
          {@const k = seenIn(ch)}
          <div class="row">
            {#if k === n}{@html check}{:else}<span class="m">{k}/{n}</span>{/if}
            <span class="grow" class:dim={k < n}>{chapterName(ch)}</span>
            {#if k === n}<span class="m">{t('tour.stops', { count: n })}</span>{:else}<button class="btn btn-ghost" onclick={() => replayChapter(ch)}>{t('tour.seeSkipped')}</button>{/if}
          </div>
        {/each}
      </div>
      <div class="foot">
        <button class="btn btn-ghost" onclick={restartTour}>{t('tour.startOver')}</button>
        <span class="grow"></span>
        <button class="btn btn-primary" onclick={closeEnd}>{hasOwnProjects() ? t('tour.backToProjects') : t('tour.startWorking')}</button>
      </div>
    </section>
  </div>
{/if}

{#if tour.toast}
  <div class="toast" role="status">{tour.toast}<button class="link" onclick={() => (tour.toast = '')}>{t('tour.ok')}</button></div>
{/if}

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 900;
    pointer-events: none;
  }

  .scrim,
  .block,
  .tour-card,
  .tour-end {
    pointer-events: auto;
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--overlay);
  }

  .spot {
    position: absolute;
    border-radius: 6px;
    pointer-events: none;
    box-shadow:
      0 0 0 2px var(--accent-soft),
      0 0 0 9999px var(--overlay);
    transition:
      left 0.35s cubic-bezier(0.2, 0.8, 0.2, 1),
      top 0.35s cubic-bezier(0.2, 0.8, 0.2, 1),
      width 0.35s cubic-bezier(0.2, 0.8, 0.2, 1),
      height 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .block {
    position: absolute;
  }

  .tour-card {
    position: absolute;
    width: 340px;
    padding: 14px 16px 12px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    transition:
      left 0.35s cubic-bezier(0.2, 0.8, 0.2, 1),
      top 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  .arrow {
    position: absolute;
    width: 10px;
    height: 10px;
    background: var(--menu);
    transform: rotate(45deg);
    box-shadow: inset 1px 1px 0 0 var(--border-strong);
  }

  .arrow.above {
    box-shadow: inset -1px -1px 0 0 var(--border-strong);
  }

  .arrow.right {
    box-shadow: inset 1px -1px 0 0 var(--border-strong);
  }

  .arrow.left {
    box-shadow: inset -1px 1px 0 0 var(--border-strong);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 20px;
  }

  .chapter {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .chapter span {
    margin-left: 4px;
    font: 600 11.5px var(--mono);
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-faint);
  }

  .grow {
    flex: 1;
  }

  .count {
    font: 500 11.5px var(--mono);
    color: var(--text-faint);
  }

  .x {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 14px;
  }

  .x:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  h3 {
    margin: 8px 0 4px;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-strong);
  }

  p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .try {
    margin-top: 8px;
    color: var(--text);
  }

  .try :global(svg) {
    vertical-align: -2px;
    margin-right: 2px;
  }

  .keys {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 10px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .keys kbd {
    height: 20px;
    min-width: 20px;
    padding: 0 6px;
    display: inline-grid;
    place-items: center;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-2);
    font: 500 11.5px var(--mono);
  }

  .keys span {
    margin-left: 4px;
  }

  .bar {
    display: flex;
    gap: 3px;
    margin: 14px 0 12px;
  }

  .bar button {
    position: relative;
    height: 4px;
    min-width: 8px;
    border-radius: 2px;
    background: var(--surface-5);
  }

  .bar button::after {
    content: '';
    position: absolute;
    inset: -6px 0;
  }

  .bar button:hover {
    background: var(--text-faint);
  }

  .bar i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: 2px;
    background: var(--text-faint);
    transition: width 0.3s;
  }

  .bar .cur i {
    background: var(--text);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .foot.spaced {
    margin-top: 14px;
  }

  .foot .btn {
    height: 30px;
    padding: 0 14px;
    font-size: 12.5px;
  }

  .foot .btn-ghost {
    padding: 0 10px;
  }

  .foot .btn-ghost:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .link {
    height: 30px;
    padding: 0 4px;
    border-radius: 4px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .link:hover {
    color: var(--text-2);
  }

  .tour-end {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 480px;
    transform: translate(-50%, -50%);
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
    text-align: center;
  }

  .seal {
    width: 64px;
    height: 64px;
    margin: 26px auto 12px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
  }

  .tour-end h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
  }

  .tour-end > p {
    margin: 4px 26px 0;
    color: var(--text-dim);
  }

  .list {
    max-height: 340px;
    margin: 16px 22px 18px;
    overflow: auto;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  .row {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    font-size: 13px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row .dim {
    color: var(--text-muted);
  }

  .row .m {
    min-width: 14px;
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .row .btn-ghost {
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
  }

  .tour-end .foot {
    padding: 14px 22px;
    border-top: 1px solid var(--border);
  }

  .tour-end .foot .btn {
    height: 32px;
  }

  .toast {
    position: fixed;
    left: 50%;
    bottom: 46px;
    z-index: 900;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    transform: translateX(-50%);
    font-size: 12.5px;
    color: var(--text-2);
    white-space: nowrap;
  }

  .toast .link {
    height: 24px;
  }

  @media (prefers-reduced-motion: reduce) {
    .spot,
    .tour-card,
    .bar i {
      transition: none;
    }
  }
</style>
