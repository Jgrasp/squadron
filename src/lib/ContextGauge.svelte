<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { send } from './agents.svelte'
  import { tokens } from './format'
  import { t } from './i18n/index.svelte'
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
  <button class="gauge" class:near aria-haspopup="dialog" aria-expanded={open} title={when} onclick={toggle}>
    {#if max}
      <span class="bar" aria-hidden="true"><span style:width="{share * 100}%"></span></span>
      <span class="mono">{t('agent.contextUsed', { used: tokens(agent.tokens), max: tokens(max), percent })}</span>
      {#if near}<span>· {t('agent.compactSoon', { left: tokens(left!) })}</span>{/if}
    {:else}
      <span class="mono">{t('agent.contextTokens', { tokens: tokens(agent.tokens) })}</span>
    {/if}
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: -2px -4px;
    padding: 2px 4px;
    border-radius: 3px;
    color: inherit;
    font-size: 12px;
    white-space: nowrap;
  }

  .gauge:hover,
  .gauge[aria-expanded='true'] {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .gauge.near {
    color: var(--accent-link);
  }

  .bar {
    width: 44px;
    height: 5px;
    overflow: hidden;
    border-radius: 3px;
    background: var(--border-strong);
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--text-muted);
  }

  .near .bar span {
    background: var(--accent);
  }

  .panel {
    position: absolute;
    top: 24px;
    left: -4px;
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
