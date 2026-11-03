<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { agents } from './agents.svelte'
  import { t } from './i18n/index.svelte'
  import { projects } from './projects.svelte'
  import { boot, done } from './boot.svelte'
  import Trans from './Trans.svelte'

  type Step = 'settings' | 'projects' | 'agents' | 'pulls' | 'ready'
  const LABELS = { settings: 'shell.splashSettings', projects: 'shell.splashProjects', agents: 'shell.splashAgents', pulls: 'shell.splashPulls' } as const
  const SLOTS = 7

  const still = matchMedia('(prefers-reduced-motion: reduce)').matches
  let phase = $state<'loading' | 'ready' | 'fly' | 'out' | 'gone'>(still ? 'gone' : 'loading')
  let quick = $state(false)
  let step = $state<Step>('settings')
  let total = $state(0)
  let shown = $state(0)
  let first = $state('')
  let formation = $state<SVGSVGElement>()
  let label = $state<HTMLElement>()
  let lift = $state('')
  let pull = $state('')

  const tri = (x: number, y: number, h: number) => `${x},${y - h * 0.55} ${x + h * 0.5},${y + h * 0.45} ${x - h * 0.5},${y + h * 0.45}`
  const slots = $derived(
    Array.from({ length: Math.min(total, SLOTS) }, (_, i) => {
      const side = i % 2 ? 1 : -1
      const rank = Math.floor(i / 2) + 1
      const agent = agents.list[Math.floor((i * total) / Math.min(total, SLOTS))]
      return { x: 200 + side * rank * 40, y: 78 + rank * 30, color: projects.list.find((p) => p.path === agent?.project)?.color || 'var(--accent)' }
    }),
  )
  const ships = $derived(slots.filter((_, i) => shown >= Math.ceil(((i + 1) * total) / slots.length)).length)

  invoke<string | null>('account_name').then((found) => (first = found ?? ''), () => {})

  const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
  const until = (done: () => boolean, cap = Infinity) =>
    new Promise<void>((resolve) => {
      const start = performance.now()
      const look = () => (done() || performance.now() - start > cap ? resolve() : setTimeout(look, 30))
      look()
    })
  const live = () => phase !== 'out' && phase !== 'gone'

  async function run() {
    await Promise.all([wait(250), until(() => boot.settings, 3000)])
    if (!live()) return
    step = 'projects'
    await Promise.all([wait(380), until(() => boot.projects, 5000)])
    await wait(150)
    if (!live()) return
    step = 'agents'
    await until(() => boot.agents >= 0, 5000)
    total = Math.max(boot.agents, agents.list.length)
    for (let i = 1; i <= total && live(); i++) {
      await wait(total <= SLOTS ? 230 + ((i * 137) % 5) * 70 : Math.max(2600 / total, 16))
      shown = i
    }
    await wait(520)
    if (!live()) return
    step = 'pulls'
    await Promise.all([wait(300), until(() => boot.pulls, 1500)])
    if (!live()) return
    step = 'ready'
    phase = 'ready'
    await wait(700)
    if (!live() || !formation || !label) return
    const box = label.getBoundingClientRect()
    lift = `translateY(${-formation.getBoundingClientRect().bottom - 20}px)`
    pull = `translateY(${innerHeight / 2 - (box.top + box.height / 2)}px)`
    phase = 'fly'
    await wait(2000)
    if (!live()) return
    phase = 'out'
    await wait(500)
    phase = 'gone'
  }

  if (!still) run()

  $effect(() => {
    if (phase === 'gone') done()
  })

  function skip(event: Event) {
    if (!live()) return
    event.preventDefault()
    event.stopPropagation()
    quick = true
    phase = 'out'
    setTimeout(() => (phase = 'gone'), 220)
  }
</script>

<svelte:window onkeydowncapture={skip} onpointerdowncapture={skip} />

{#if phase !== 'gone'}
  <div class="splash" class:fly={phase === 'fly' || phase === 'out'} class:out={phase === 'out'} class:quick>
    <svg viewBox="0 0 400 230" aria-hidden="true" bind:this={formation} style:transform={lift}>
      {#each slots as slot, i (i)}
        <polygon class="slot" class:in={i < ships} points={tri(slot.x, slot.y, 20)} />
      {/each}
      {#each slots as slot, i (i)}
        <g class="ship" class:in={i < ships}>
          <polygon class="bob" style:animation-delay="-{i * 0.37}s" style:fill={slot.color} points={tri(slot.x, slot.y, 20)} />
        </g>
      {/each}
      <polygon class="bob" fill="var(--accent)" points={tri(200, 70, 44)} />
    </svg>
    <div class="label" class:done={step === 'ready'} role="status" bind:this={label} style:transform={pull}>
      {#key step}
        <span class="swap">
          {#if step !== 'ready'}
            <b>{t(LABELS[step])}</b>
          {:else if first}
            <Trans k="shell.splashReadyName">{#snippet name()}<b>{first}</b>{/snippet}</Trans>
          {:else}
            {t('shell.splashReady')}
          {/if}
        </span>
      {/key}
    </div>
    <div class="count">
      {#if step === 'agents'}
        {shown} / {total}
      {:else if step === 'ready'}
        {t('shell.splashAgentCount', { count: agents.list.length })} · {t('shell.splashProjectCount', { count: projects.list.length })}
      {:else}
        &nbsp;
      {/if}
    </div>
  </div>
{/if}

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    font-size: clamp(13px, 1.45vw, 22px);
    background: radial-gradient(ellipse at 50% 42%, #221a16 0%, var(--bg-app) 70%);
    transition: opacity 0.5s;
  }

  svg {
    width: min(46vw, 660px);
    overflow: visible;
    transition: transform 0.75s cubic-bezier(0.55, 0, 0.8, 0.2);
  }

  .ship {
    transform: translateY(60px);
    opacity: 0;
    transition:
      transform 0.55s cubic-bezier(0.2, 0.9, 0.3, 1.2),
      opacity 0.35s;
  }

  .ship.in {
    transform: none;
    opacity: 1;
  }

  .slot {
    fill: none;
    stroke: var(--border-strong);
    stroke-dasharray: 3 3;
    transition: opacity 0.3s;
  }

  .slot.in {
    opacity: 0;
  }

  .bob {
    animation: bob 2.4s ease-in-out infinite;
  }

  .label {
    margin-top: 2.2em;
    height: 1.4em;
    font-size: 0.95em;
    color: var(--text-muted);
  }

  .label b {
    color: var(--text);
    font-weight: 600;
  }

  .label.done :global(b) {
    color: var(--accent-soft);
  }

  .swap {
    display: inline-block;
    animation: swap 0.25s;
  }

  .count {
    margin-top: 0.5em;
    font-family: var(--mono);
    font-size: 0.75em;
    color: var(--text-faint);
    transition: opacity 0.3s;
  }

  .fly .label {
    transition: transform 1.1s cubic-bezier(0.55, 0, 0.15, 1.12) 0.12s;
  }

  .fly .count {
    opacity: 0;
  }

  .out {
    opacity: 0;
  }

  .quick {
    transition: opacity 0.2s;
  }

  @keyframes bob {
    50% {
      transform: translateY(-3px);
    }
  }

  @keyframes swap {
    from {
      opacity: 0;
    }
  }
</style>
