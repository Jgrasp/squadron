<script lang="ts">
  import Icon from './Icon.svelte'
  import { t } from './i18n/index.svelte'
  import { CHAPTERS, chapterName, minutes, offered, startTour, tour, type Chapter } from './tour.svelte'

  const ICONS: Record<Chapter, string> = {
    projects: 'folder',
    agents: 'persona',
    turn: 'ok',
    tools: 'editor',
    kanban: 'kanban',
    pulls: 'pull',
    find: 'jump',
    usage: 'usage',
    settings: 'gear',
    extensions: 'plugins',
  }
  const SHIPS = ['#4FA3E0', '#7CC47C', '#E3B341', '#6FA0D0', '#3CC39A', '#E07A9B']
  const tri = (x: number, y: number, h: number) => `${x},${y - h * 0.55} ${x + h * 0.5},${y + h * 0.45} ${x - h * 0.5},${y + h * 0.45}`
  const slots = SHIPS.map((color, i) => ({ x: 200 + (i % 2 ? 1 : -1) * (Math.floor(i / 2) + 1) * 40, y: 78 + (Math.floor(i / 2) + 1) * 30, color }))
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches
  const total = minutes()

  let landed = $state(still)
  let shown = $state(still ? total : 1)

  $effect(() => {
    if (!tour.welcome || still) return
    landed = false
    shown = 1
    const raf = requestAnimationFrame(() => requestAnimationFrame(() => (landed = true)))
    const start = performance.now() + 250
    let frame = requestAnimationFrame(function count(now) {
      const p = Math.min(1, Math.max(0, (now - start) / 1100))
      shown = Math.max(1, Math.round(total * (1 - Math.pow(1 - p, 3))))
      if (p < 1) frame = requestAnimationFrame(count)
    })
    return () => (cancelAnimationFrame(raf), cancelAnimationFrame(frame))
  })

  function keys(event: KeyboardEvent) {
    if (!tour.welcome) return
    if (event.key === 'Enter') (event.preventDefault(), event.stopPropagation(), startTour(0))
    else if (event.key === 'Escape') (event.preventDefault(), event.stopPropagation(), offered())
  }
</script>

<svelte:window onkeydowncapture={keys} />

{#if tour.welcome}
  <div class="welcome">
    <div class="scrim"></div>
    <section class="box" aria-labelledby="welcome-title">
      <div class="left">
        <svg class="formation" class:in={landed} viewBox="0 0 400 190" aria-hidden="true">
          {#each slots as slot, i (i)}<polygon class="slot" points={tri(slot.x, slot.y, 20)} />{/each}
          {#each slots as slot, i (i)}
            <g class="ship" style:transition-delay="{0.15 + i * 0.16}s"><polygon class="bob" style:animation-delay="-{i * 0.37}s" style:fill={slot.color} points={tri(slot.x, slot.y, 20)} /></g>
          {/each}
          <polygon class="bob" fill="var(--accent)" points={tri(200, 70, 44)} />
        </svg>
        <h2 id="welcome-title">{t('tour.welcomeTitle')}</h2>
        <p>{t('tour.welcomeLead')}</p>
        <button class="btn btn-primary" onclick={() => startTour(0)}>{t('tour.takeTour')} <kbd>↵</kbd></button>
        <button class="btn btn-secondary" onclick={offered}>{t('tour.skipStart')}</button>
      </div>
      <div class="right">
        <span class="ttl">{t('tour.whatShows')}</span>
        <div class="tiles">
          {#each CHAPTERS as ch (ch)}
            <div class="tile"><span class="ic"><Icon name={ICONS[ch]} size={15} /></span><b>{chapterName(ch)}</b></div>
          {/each}
        </div>
        <div class="eta">
          <span class="clock" aria-hidden="true">
            <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path class="minute" d="M12 12V6.5" /><path class="hour" d="M12 12h3.5" /></svg>
          </span>
          <b>{t('tour.about', { n: shown })}</b>
          <span class="track"><i class:full={landed}></i></span>
        </div>
      </div>
    </section>
  </div>
{/if}

<style>
  .welcome {
    position: fixed;
    inset: 0;
    z-index: 900;
  }

  .scrim {
    position: absolute;
    inset: 0;
    background: var(--overlay);
  }

  .box {
    position: absolute;
    left: 50%;
    top: 50%;
    width: min(900px, calc(100vw - 48px));
    display: grid;
    grid-template-columns: 330px minmax(0, 1fr);
    transform: translate(-50%, -50%);
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
    overflow: hidden;
  }

  .left {
    display: flex;
    flex-direction: column;
    padding: 26px 26px 24px;
    border-right: 1px solid var(--border);
    background: radial-gradient(ellipse at 50% 18%, #221a16 0%, var(--bg-main) 70%);
  }

  .formation {
    width: 250px;
    height: 112px;
    margin: 0 auto 10px;
    overflow: visible;
  }

  .slot {
    fill: none;
    stroke: var(--border-strong);
    stroke-dasharray: 3 3;
    transition: opacity 0.3s;
  }

  .ship {
    transform: translateY(50px);
    opacity: 0;
    transition:
      transform 0.55s cubic-bezier(0.2, 0.9, 0.3, 1.2),
      opacity 0.35s;
  }

  .formation.in .ship {
    transform: none;
    opacity: 1;
  }

  .formation.in .slot {
    opacity: 0;
  }

  .bob {
    animation: bob 2.4s ease-in-out infinite;
  }

  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-strong);
  }

  .left p {
    flex: 1;
    margin: 8px 0 18px;
    font-size: 13.5px;
    line-height: 1.6;
    color: var(--text-muted);
  }

  .left .btn {
    width: 100%;
    margin-top: 8px;
  }

  kbd {
    padding: 1px 6px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--ink) 12%, transparent);
    font: 600 11px var(--mono);
  }

  .right {
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 22px 24px;
  }

  .ttl {
    margin-bottom: 10px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .tile {
    height: 50px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .ic {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 6px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .tile b {
    min-width: 0;
    overflow: hidden;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .eta {
    margin-top: 14px;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .clock {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--accent-soft);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .clock svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .clock path {
    transform-box: view-box;
    transform-origin: 12px 12px;
  }

  .minute {
    animation: spin 3s linear infinite;
  }

  .hour {
    animation: spin 36s linear infinite;
  }

  .eta b {
    font-size: 16px;
    font-weight: 700;
    color: var(--text-strong);
    font-variant-numeric: tabular-nums;
  }

  .track {
    position: relative;
    flex: 1;
    height: 4px;
    margin-left: 6px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--surface-3);
  }

  .track i {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 0;
    border-radius: 2px;
    background: var(--accent-soft);
    transition: width 1.1s cubic-bezier(0.2, 0.8, 0.2, 1) 0.25s;
  }

  .track i.full {
    width: 100%;
    animation: glow 2.4s ease-in-out 1.4s infinite;
  }

  @keyframes bob {
    50% {
      transform: translateY(-3px);
    }
  }

  @keyframes glow {
    50% {
      opacity: 0.55;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ship,
    .slot,
    .track i {
      transition: none;
    }

    .bob,
    .minute,
    .hour,
    .track i.full {
      animation: none;
    }
  }
</style>
