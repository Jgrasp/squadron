<script lang="ts">
  import { clock } from './agents.svelte'
  import { duration, tokens } from './format'
  import { t } from './i18n/index.svelte'
  import type { Agent, Subagent } from './stream'
  import { asking, count, counted, label, lookOf, LOOKS } from './subagents'

  let {
    agent,
    ids,
    onopen,
    onpanel,
  }: { agent: Agent; ids: string[]; onopen?: (sub: Subagent) => void; onpanel?: () => void } = $props()

  const subs = $derived(agent.subagents.filter((s) => ids.includes(s.id)))
  const waiting = $derived(asking(agent))
  const counts = $derived(count(subs, waiting))
  const types = $derived([...new Set(subs.map((s) => s.type))].join(', '))
  const total = $derived(subs.reduce((t, s) => t + s.tokens, 0))
  const running = $derived(subs.some((s) => s.status === 'running'))
  const span = $derived(
    (running ? clock.now : Math.max(...subs.map((s) => s.ended ?? s.started))) - Math.min(...subs.map((s) => s.started)),
  )

  let picked = $state<string | null>(null)
  const selected = $derived(
    subs.find((s) => s.id === picked) ??
      subs.find((s) => waiting.has(s.id)) ??
      subs.find((s) => s.status === 'error') ??
      subs.find((s) => s.status === 'running') ??
      subs.at(-1),
  )
  const doing = (s: Subagent) => (s.status === 'done' ? (s.result?.split('\n').find((l) => l.trim()) ?? t('conversation.resultHandedBack')) : s.last)
</script>

<section class="subagents" aria-label={t('conversation.subagentsLaunched')}>
  <div class="head">
    <span class="icon" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 24 24"><circle cx="6" cy="5" r="2" /><circle cx="6" cy="19" r="2" /><circle cx="18" cy="12" r="2" /><path d="M6 7v10M6 12h10" /></svg>
    </span>
    <span class="what">
      <strong>{t('conversation.launchedCount', { count: subs.length })} <span class="type">· {types}</span></strong>
      <span class="names">{subs.map((s) => s.name).join(' · ')}</span>
    </span>
    <span class="mono">{counts.done}/{subs.length} · {duration(span)}</span>
  </div>

  {#if subs.length > 1}
    <div class="grid" role="group" aria-label={t('conversation.subagentMap')}>
      {#each subs as s (s.id)}
        {@const look = lookOf(s, waiting)}
        <button
          class="cell {look}"
          aria-label={t('conversation.subagentLabel', { num: s.num, name: s.name, state: label(look) })}
          title="#{s.num} · {s.name} · {label(look)}"
          aria-pressed={s === selected}
          onclick={() => (picked = s.id)}
        ></button>
      {/each}
    </div>
    <div class="legend">
      {#each LOOKS as look (look)}
        {#if counts[look]}<span><span class="dot {look}"></span>{counted(look, counts[look])}</span>{/if}
      {/each}
      <span class="grow"></span>
      <span class="mono">{t('conversation.tokens', { n: tokens(total) })}</span>
    </div>
  {/if}

  {#if selected}
    {@const look = lookOf(selected, waiting)}
    <div class="pick">
      <span class="dot {look}"></span>
      <span class="about">
        <span><strong>#{selected.num} · {selected.name}</strong> <span class="state">· {label(look)}</span></span>
        <span class="last">{doing(selected)}</span>
      </span>
      {#if subs.length === 1}<span class="mono">{t('conversation.tokens', { n: tokens(selected.tokens) })}</span>{/if}
      {#if onopen}<button class="link" onclick={() => onopen(selected)}>{t('conversation.openLog')}</button>{/if}
    </div>
  {/if}

  {#if onpanel && subs.length > 1}
    <div class="foot">
      <span>{t('conversation.clickSquare')}</span>
      <button class="btn btn-secondary" onclick={onpanel}>{t('conversation.openPanel')}</button>
    </div>
  {/if}
</section>

<style>
  .subagents {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    flex: none;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--surface-3);
  }

  .icon svg {
    fill: none;
    stroke: var(--text);
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .type {
    font-weight: 400;
    color: var(--text-dim);
  }

  .names {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .mono {
    flex: none;
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 3px;
  }

  .cell {
    width: 14px;
    height: 14px;
    padding: 0;
    border-radius: 2px;
  }

  .cell[aria-pressed='true'] {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }

  .running {
    background: var(--sub-running);
  }

  .waiting {
    background: var(--sub-waiting);
    box-shadow: 0 0 8px rgba(224, 138, 91, 0.8);
  }

  .error {
    background: var(--sub-error);
    box-shadow: inset 0 0 0 3px var(--err-bg);
  }

  .dot.error {
    box-shadow: inset 0 0 0 2px var(--err-bg);
  }

  .done {
    background: var(--sub-done-cell);
  }

  .stopped {
    background: var(--sub-stopped);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 14px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .legend > span {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .grow {
    flex: 1;
  }

  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .pick {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .about {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .state {
    color: var(--text-muted);
  }

  .last {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .link {
    flex: none;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-link);
  }

  .link:hover {
    color: var(--accent-soft);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .foot .btn {
    height: 30px;
    padding: 0 12px;
  }
</style>
