<script lang="ts">
  import { workOf } from './artifacts.svelte'
  import { modelLabel } from './format'
  import { t } from './i18n/index.svelte'
  import { labelOf } from './layout-slots.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import { subagentOf, type Agent, type Subagent } from './stream'

  let { agent, sub }: { agent: Agent; sub: Subagent } = $props()

  const asks = $derived(agent.asks.filter((a) => subagentOf(agent, a.tool_use_id) === sub))
  const work = $derived(workOf(agent, sub)?.[1])
  const p = $derived(work?.progress)
  const state = $derived(asks.length ? 'ask' : sub.status === 'running' ? 'run' : sub.status === 'done' ? 'fin' : 'err')
  const name = $derived(sub.name.slice(sub.name.lastIndexOf(':') + 1))
  const last = $derived(sub.log.findLast((b) => b.type === 'text')?.text.split('\n').find((l) => l.trim()) ?? '')
  const cells = $derived(p ? Array.from({ length: p.total }, (_, i) => (state === 'fin' || i < p.done ? 'on' : i === p.done && state !== 'err' ? 'now' : '')) : state === 'run' || state === 'ask' ? ['now'] : [])
  const from = $derived([work && t('conversation.forkFrom', { what: labelOf('artifact.actions', work.destination) }), sub.model && modelLabel(sub.model)].filter(Boolean).join(' · '))
  const label = $derived(state === 'fin' || state === 'err' ? last : p?.message || t('conversation.forkWorking'))
</script>

<section class="fork {state}" aria-label={name}>
  <div class="head">
    <span class="who" aria-hidden="true">
      {#if state === 'fin'}
        <svg width="14" height="14" viewBox="0 0 24 24" style="stroke-width: 3"><path d="M5 12.5l4.5 4.5L19 7" /></svg>
      {:else if state === 'err'}
        <svg width="12" height="12" viewBox="0 0 24 24" style="stroke-width: 3"><path d="M5 5l14 14M19 5 5 19" /></svg>
      {:else}
        <svg width="16" height="16" viewBox="0 0 24 24"><circle cx="6" cy="5" r="2.2" /><circle cx="18" cy="17" r="2.2" /><path d="M6 7.5v3.5a4 4 0 0 0 4 4h5.5" /></svg>
      {/if}
    </span>
    <span class="what">
      <strong>{state === 'ask' ? t('conversation.forkAsks', { name }) : name}<span class="tag">{t('conversation.forkTag')}</span></strong>
      {#if from}<span class="from">{from}</span>{/if}
    </span>
    {#if p}<span class="count">{state === 'fin' ? `${p.total}/${p.total}` : `${Math.min(p.done + 1, p.total)}/${p.total}`}</span>{/if}
  </div>
  {#if cells.length}
    <span class="track" aria-hidden="true">{#each cells as cell, c (c)}<i class={cell}></i>{/each}</span>
  {/if}
  {#if label}
    {#key label}<span class="label"><span>{label}</span></span>{/key}
  {/if}
  {#each asks as ask (ask.id)}
    <div class="inner" data-fork-ask><PermissionCard {agent} {ask} nested /></div>
  {/each}
</section>

<style>
  .fork {
    --tone: var(--background);
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    transition: background 0.35s;
  }

  .fork::after {
    content: '';
    position: absolute;
    inset: 0;
    padding: 1.5px;
    border-radius: 6px;
    pointer-events: none;
    background: conic-gradient(
      from var(--snake),
      color-mix(in srgb, var(--tone) 25%, transparent) 0deg 230deg,
      var(--tone) 300deg,
      color-mix(in srgb, var(--tone) 15%, white) 345deg,
      color-mix(in srgb, var(--tone) 25%, transparent) 360deg
    );
    mask:
      linear-gradient(black 0 0) content-box,
      linear-gradient(black 0 0);
    mask-composite: exclude;
    animation: snake 2.4s linear infinite;
  }

  .fork.ask {
    --tone: var(--accent);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    animation: glow 2.4s ease-in-out infinite;
  }

  .fork.ask::after {
    animation-duration: 3.4s;
  }

  .fork.fin {
    --tone: var(--ok);
  }

  .fork.err {
    --tone: var(--err);
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
  }

  .fork.fin::after,
  .fork.err::after {
    display: none;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .who {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 4px;
    background: var(--background-bg);
    box-shadow: inset 0 0 0 1px var(--background-line);
    color: var(--tone);
  }

  .who svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .ask .who {
    background: var(--accent);
    box-shadow: none;
    color: var(--accent-ink);
    animation: ring 2.4s ease-in-out infinite;
  }

  .fin .who,
  .err .who {
    background: color-mix(in srgb, var(--tone) 14%, transparent);
    box-shadow: none;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .what strong {
    font-weight: 600;
    color: var(--text-strong);
  }

  .ask .what strong {
    color: var(--wait-text);
  }

  .tag {
    display: inline-block;
    margin-left: 8px;
    padding: 1px 7px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--tone) 14%, transparent);
    color: var(--tone);
    font: 600 11px var(--sans);
    vertical-align: 1px;
  }

  .ask .tag {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent-soft);
  }

  .from {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .ask .from {
    color: var(--wait-muted);
  }

  .count {
    flex: none;
    font: 600 12px var(--mono);
    color: var(--tone);
  }

  .track {
    display: flex;
    gap: 3px;
    height: 4px;
  }

  .track i {
    position: relative;
    flex: 1;
    overflow: hidden;
    border-radius: 2px;
    background: color-mix(in srgb, var(--tone) 16%, transparent);
    transition: background 0.4s;
  }

  .track i.on {
    background: var(--tone);
  }

  .track i.now {
    background: color-mix(in srgb, var(--tone) 30%, transparent);
  }

  .track i.now::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--tone), transparent) 0 0 / 60% 100% no-repeat;
    animation: run 1.3s ease-in-out infinite;
  }

  .ask .track i.now::after {
    background: var(--tone);
    animation: pulse 1.6s ease-in-out infinite;
  }

  .label {
    position: relative;
    display: block;
    height: 19px;
    overflow: hidden;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .err .label {
    color: var(--err-text);
  }

  .label span {
    position: absolute;
    inset: 0 0 auto;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    animation: rise 0.45s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  .inner {
    animation: open 0.4s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  @keyframes run {
    from {
      background-position: -100% 0;
    }
    to {
      background-position: 200% 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      filter: blur(2px);
      transform: translateY(70%);
    }
    to {
      opacity: 1;
      filter: none;
      transform: none;
    }
  }

  @keyframes glow {
    50% {
      box-shadow:
        inset 0 0 0 1px var(--wait-border),
        0 0 24px 0 color-mix(in srgb, var(--accent) 24%, transparent);
    }
  }

  @keyframes open {
    from {
      opacity: 0;
      clip-path: inset(0 0 100% 0);
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      clip-path: inset(0);
      transform: none;
    }
  }
</style>
