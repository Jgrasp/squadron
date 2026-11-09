<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { cubicOut } from 'svelte/easing'
  import { Tween } from 'svelte/motion'
  import ContextPanel from './ContextPanel.svelte'
  import { fetchDetails } from './plugins.svelte'
  import { tokens } from './format'
  import { t } from './i18n/index.svelte'
  import { layout } from './layout.svelte'
  import type { Agent } from './stream'

  let { agent }: { agent: Agent } = $props()

  let open = $state(false)

  $effect(() => {
    fetchDetails((agent.plugins ?? []).filter((r) => !r.id.endsWith('@inline')).map((r) => r.id))
  })

  const max = $derived(agent.context?.max ?? 0)
  const threshold = $derived(agent.context?.threshold ?? null)
  const left = $derived(threshold === null ? null : Math.max(0, threshold - agent.tokens))
  type Bite = { id: number; x: number; to: number; amount: number }

  let eaten = $state(0)
  let eating = $state(false)
  let bites = $state<Bite[]>([])
  let pending = 0
  let seen = { id: '', tokens: 0 }
  let round = 0
  let bite = 0
  let closing: ReturnType<typeof setTimeout> | undefined
  const still = matchMedia('(prefers-reduced-motion: reduce)')

  const fill = $derived(max ? Math.min(1, eaten / max) : 0)
  const tone = $derived(fill < 0.5 ? 'low' : fill < 0.8 ? 'mid' : 'high')
  const level = Tween.of(() => fill, { duration: 350, easing: cubicOut })

  $effect(() => {
    const tokens = agent.tokens
    const add = tokens - seen.tokens
    const same = seen.id === agent.id
    seen = { id: agent.id, tokens }
    if (!same || add <= 0 || still.matches) {
      round++
      clearTimeout(closing)
      bites = []
      pending = 0
      eating = false
      eaten = tokens
      return
    }
    clearTimeout(closing)
    eating = true
    pending += add
    const count = Math.max(1, Math.min(10, Math.round(add / 2000)))
    const now = round
    for (let i = 0; i < count; i++)
      setTimeout(() => {
        if (now !== round) return
        if (bites.length >= 10) return swallow(add / count)
        bites.push({ id: bite++, x: 13 + Math.random() * 4, to: 5 + 20 * (1 - level.current) - 1.4, amount: add / count })
      }, 300 + i * 150)
  })

  function swallow(amount: number) {
    eaten = Math.min(agent.tokens, eaten + amount)
    pending -= amount
    if (pending < 1) closing = setTimeout(() => (eating = false), 380)
  }

  function fly(node: SVGRectElement, b: Bite) {
    node
      .animate(
        [
          { transform: `translate(${b.x}px, 1px)`, opacity: 0 },
          { opacity: 1, offset: 0.15 },
          { transform: `translate(${b.x}px, ${b.to}px)`, opacity: 1 },
        ],
        { duration: 520, easing: 'cubic-bezier(.5, 0, 1, .6)', fill: 'forwards' },
      )
      .finished.then(() => {
        bites = bites.filter((x) => x.id !== b.id)
        swallow(b.amount)
      })
      .catch(() => {})
  }
  const when = $derived(
    left !== null ? t('agent.autoCompactAt', { at: tokens(threshold!), left: tokens(left) }) : max ? t('agent.autoCompactOff') : t('agent.contextLater'),
  )

  function toggle() {
    open = !open
    if (open) invoke('context_usage', { id: agent.id, start: true }).catch(() => {})
  }

</script>

<svelte:window onclick={(e) => open && !(e.target as HTMLElement).closest('[data-context]') && (open = false)} onkeydown={(e) => e.key === 'Escape' && (open = false)} />

<span class="anchor" data-context>
  <button class="gauge {tone}" aria-haspopup="dialog" aria-expanded={open} title={when} onclick={toggle}>
    <svg class="ring" class:eating width="22" height="22" viewBox="3 3 24 24" aria-hidden="true">
      <clipPath id="context-{agent.id}"><circle cx="15" cy="15" r="9.6" /></clipPath>
      <rect class="level" clip-path="url(#context-{agent.id})" x="4" y={5 + 20 * (1 - level.current)} width="22" height="21" />
      <circle class="rim" cx="15" cy="15" r="11" transform="rotate(-90 15 15)" />
      {#each bites as b (b.id)}
        <rect class="bite" x="-1.1" y="-1.1" width="2.2" height="2.2" rx="0.5" opacity="0" use:fly={b} />
      {/each}
    </svg>
    <span class="mono">{!max || layout.contextShown === 'tokens' ? tokens(Math.round(eaten)) : layout.contextShown === 'both' ? t('agent.contextBoth', { tokens: tokens(Math.round(eaten)), percent: Math.round(fill * 100) }) : t('agent.contextPercent', { percent: Math.round(fill * 100) })}</span>
  </button>
  {#if open}
    <div class="panel" role="dialog" aria-label={t('agent.context')}>
      <ContextPanel {agent} onclose={() => (open = false)} />
    </div>
  {/if}
</span>

<style>
  .anchor {
    position: relative;
    display: inline-flex;
  }

  .gauge {
    --level: #4ade80;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 6px;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 14px;
    white-space: nowrap;
  }

  .gauge.mid {
    --level: #fb923c;
  }

  .gauge.high {
    --level: #f87171;
  }

  .gauge.mid,
  .gauge.high {
    color: var(--level);
  }

  .gauge:hover,
  .gauge[aria-expanded='true'] {
    background: var(--surface-3);
  }

  .ring {
    flex: none;
    overflow: visible;
  }

  .ring .level,
  .ring .bite {
    fill: var(--level);
  }

  .ring .rim {
    fill: none;
    stroke: var(--level);
    stroke-opacity: 0.6;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-dasharray: 69.1 0;
    stroke-dashoffset: 0;
    transition:
      stroke-dasharray 0.4s cubic-bezier(0.3, 1.3, 0.5, 1),
      stroke-dashoffset 0.4s cubic-bezier(0.3, 1.3, 0.5, 1);
  }

  .eating .rim {
    stroke-dasharray: 53.1 16;
    stroke-dashoffset: -8;
  }

  .panel {
    position: absolute;
    top: 34px;
    left: 0;
    z-index: 30;
    width: 460px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    font-size: 13px;
    color: var(--text);
    white-space: normal;
  }

</style>
