<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { send } from './agents.svelte'
  import { tokens } from './format'
  import { t } from './i18n/index.svelte'
  import { layout, setContextShown } from './layout.svelte'
  import type { Agent } from './stream'

  let { agent }: { agent: Agent } = $props()

  let open = $state(false)

  const max = $derived(agent.context?.max ?? 0)
  const share = $derived(max ? Math.min(1, agent.tokens / max) : 0)
  const percent = $derived(Math.round(share * 100))
  const threshold = $derived(agent.context?.threshold ?? null)
  const left = $derived(threshold === null ? null : Math.max(0, threshold - agent.tokens))
  const near = $derived(left !== null && threshold !== null && left < threshold * 0.1)
  const busy = $derived(agent.status === 'working' || agent.status === 'waiting')
  const tone = $derived(share < 0.5 ? 'low' : share < 0.8 ? 'mid' : 'high')
  const level = $derived(agent.tokens ? 18 - 16 * share : 20)
  const when = $derived(
    left !== null ? t('agent.autoCompactAt', { at: tokens(threshold!), left: tokens(left) }) : max ? t('agent.autoCompactOff') : t('agent.contextLater'),
  )

  function toggle() {
    open = !open
    if (open) invoke('context_usage', { id: agent.id }).catch(() => {})
  }

  function compact() {
    open = false
    send(agent.id, '/compact')
  }
</script>

<svelte:window onclick={(e) => open && !(e.target as HTMLElement).closest('[data-context]') && (open = false)} onkeydown={(e) => e.key === 'Escape' && (open = false)} />

<span class="anchor" data-context>
  <button class="gauge {tone}" aria-haspopup="dialog" aria-expanded={open} title={when} onclick={toggle}>
    <svg class="ring" class:moving={agent.status === 'working'} width="20" height="20" viewBox="0 0 20 20" aria-hidden="true">
      <clipPath id="context-{agent.id}"><circle cx="10" cy="10" r="8" /></clipPath>
      <g clip-path="url(#context-{agent.id})">
        <path class="back" d="M-10 {level - 0.6}q2.5 1.6 5 0t5 0 5 0 5 0 5 0 5 0V20H-10Z" />
        <path class="front" d="M-10 {level}q2.5-1.6 5 0t5 0 5 0 5 0 5 0 5 0V20H-10Z" />
      </g>
      <circle class="rim" cx="10" cy="10" r="8.75" />
    </svg>
    <span class="mono">{!max || layout.contextShown === 'tokens' ? tokens(agent.tokens) : layout.contextShown === 'both' ? t('agent.contextBoth', { tokens: tokens(agent.tokens), percent }) : t('agent.contextPercent', { percent })}</span>
  </button>
  {#if open}
    <div class="panel" role="dialog" aria-label={t('agent.context')}>
      <strong>{t('agent.context')}</strong>
      {#if max}
        <div class="track" aria-hidden="true">
          <span class="fill" class:near style:width="{share * 100}%"></span>
          {#if threshold}<span class="mark" style:left="{(threshold / max) * 100}%"></span>{/if}
        </div>
        <p>{t('agent.contextOf', { used: tokens(agent.tokens), max: tokens(max), percent })}</p>
      {:else}
        <p>{t('agent.contextTokens', { tokens: tokens(agent.tokens) })}</p>
      {/if}
      <p class="when" class:near>{when}</p>
      <div class="shown">
        <span id="context-shown-{agent.id}">{t('agent.contextShown')}</span>
        <div class="segments" role="group" aria-labelledby="context-shown-{agent.id}">
          {#each [['percent', 'agent.contextShownPercent'], ['tokens', 'agent.contextShownTokens'], ['both', 'agent.contextShownBoth']] as const as [shown, label] (shown)}
            <button aria-pressed={layout.contextShown === shown} onclick={() => setContextShown(shown)}>{t(label)}</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <button class="btn btn-secondary" disabled={busy || !agent.tokens} onclick={compact}>{t('agent.compactNow')}</button>
        {#if busy}<span class="hint">{t('agent.compactWhenIdle')}</span>{/if}
      </div>
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
  }

  .ring .front {
    fill: var(--level);
  }

  .ring .back {
    fill: var(--level);
    opacity: 0.4;
  }

  .ring .rim {
    fill: none;
    stroke: var(--level);
    stroke-opacity: 0.55;
    stroke-width: 1.5;
  }

  .moving .front {
    animation: wave 1.6s linear infinite;
  }

  .moving .back {
    animation: wave 2.4s linear infinite reverse;
  }

  @keyframes wave {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(10px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .moving path {
      animation: none;
    }
  }

  .panel {
    position: absolute;
    top: 34px;
    left: 0;
    z-index: 30;
    width: 340px;
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

  .panel strong {
    font-weight: 600;
    color: var(--text-strong);
  }

  .panel p {
    margin: 0;
  }

  .track {
    position: relative;
    height: 8px;
    border-radius: 4px;
    background: var(--border-strong);
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 4px;
    background: var(--text-muted);
  }

  .fill.near {
    background: var(--accent);
  }

  .mark {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--accent);
  }

  .when {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .when.near {
    color: var(--accent-link);
  }

  .shown {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 6px 10px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .segments {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    height: 26px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-2);
    font-size: 12.5px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .row .btn {
    height: 30px;
    padding: 0 12px;
  }

  .hint {
    font-size: 12px;
    color: var(--text-dim);
  }
</style>
