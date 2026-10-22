<script lang="ts">
  import { clock } from './agents.svelte'
  import { cost, tokens } from './format'
  import SidePanel from './SidePanel.svelte'
  import SubagentLog from './SubagentLog.svelte'
  import type { Agent, Subagent } from './stream'
  import { asking, count, LABEL, lookOf, type Look } from './subagents'

  let {
    agent,
    viewing,
    onview,
    onprefill,
    onquote,
    onclose,
  }: {
    agent: Agent
    viewing: string | null
    onview: (id: string | null) => void
    onprefill: (text: string) => void
    onquote: (text: string) => void
    onclose: () => void
  } = $props()

  let selected = $state<string | null>(null)
  let size = $state<'panel' | 'wide' | 'screen'>('panel')

  $effect(() => {
    if (!viewing) size = 'panel'
  })
  const open = $derived(agent.subagents.find((s) => s.id === viewing))

  $effect(() => {
    if (viewing) selected = viewing
  })

  type Filter = 'all' | Look

  const FILTERS: [Filter, string][] = [
    ['all', 'All'],
    ['waiting', 'Waiting'],
    ['error', 'Failed'],
    ['running', 'Running'],
    ['done', 'Done'],
  ]

  let filter = $state<Filter>('all')
  let doneOpen = $state(false)

  const waiting = $derived(asking(agent))
  const subs = $derived(agent.subagents)
  const counts = $derived(count(subs, waiting))
  const total = $derived(subs.reduce((t, s) => t + s.tokens, 0))
  const types = $derived([...new Set(subs.map((s) => s.type))].join(', '))
  const failed = $derived(subs.filter((s) => s.status === 'error'))
  const shows = (look: Look) => filter === 'all' || filter === look || (filter === 'done' && look === 'stopped')

  const sections = $derived(
    [
      { id: 'handle', title: 'To handle', rows: subs.filter((s) => ['waiting', 'error'].includes(lookOf(s, waiting))).sort((a, b) => Number(!waiting.has(a.id)) - Number(!waiting.has(b.id)) || a.num - b.num) },
      { id: 'running', title: 'Running', rows: subs.filter((s) => lookOf(s, waiting) === 'running') },
      { id: 'done', title: 'Done', rows: subs.filter((s) => ['done', 'stopped'].includes(lookOf(s, waiting))) },
    ]
      .map((section) => ({ ...section, rows: section.rows.filter((s) => shows(lookOf(s, waiting))) }))
      .filter((section) => section.rows.length),
  )

  const two = (n: number) => String(n).padStart(2, '0')
  const elapsed = (s: Subagent) => {
    const ms = s.status === 'running' ? clock.now - s.started : s.ms || (s.ended ?? s.started) - s.started
    const sec = Math.max(0, Math.round(ms / 1000))
    return `${Math.floor(sec / 60)}:${two(sec % 60)}`
  }
  const doing = (s: Subagent, look: Look) =>
    look === 'waiting'
      ? `Waiting for you · ${s.last}`
      : s.status === 'done' || s.status === 'error'
        ? (s.result?.split('\n').find((l) => l.trim()) ?? s.last)
        : s.last

  function relaunch() {
    const list = failed.map((s) => `#${s.num} ${s.name}`).join(', ')
    onprefill(`Relaunch the subagent${failed.length > 1 ? 's' : ''} that failed: ${list}.`)
  }
</script>

<SidePanel kind="subagents" title="Subagents" screen={size === 'screen' && !!open} wide={size === 'wide' && !!open} {onclose}>
  {#snippet sub()}
    {#if open}#{open.num} · {open.name}{:else}{subs.length} launched by {agent.name} · {types}{/if}
  {/snippet}

  {#if open}
    <SubagentLog {agent} sub={open} {onview} {onquote} {size} onsize={(next) => (size = next)} />
  {:else}

  <div class="filters">
    <div class="chips" role="group" aria-label="Filter by state">
      {#each FILTERS as [id, label] (id)}
        <button aria-pressed={filter === id} onclick={() => (filter = id)}>
          <span class="dot {id}"></span>{label}<span class="n">{id === 'all' ? subs.length : id === 'done' ? counts.done + counts.stopped : counts[id]}</span>
        </button>
      {/each}
    </div>
    <div class="bar" aria-hidden="true">
      {#each ['done', 'running', 'waiting', 'error', 'stopped'] as const as look (look)}
        {#if counts[look]}<span class={look} style:flex={counts[look]}></span>{/if}
      {/each}
    </div>
  </div>

  <div class="columns" aria-hidden="true"><span></span><span>#</span><span>Subagent</span><span>Last action</span><span>Time</span><span>Tokens</span></div>

  <div class="list" role="group" aria-label="Subagents">
    {#each sections as section (section.id)}
      {#if section.id === 'done'}
        <button class="section {section.id}" aria-expanded={doneOpen || filter === 'done'} onclick={() => (doneOpen = !doneOpen)}>
          Done <span class="n">{section.rows.length}</span><span class="grow"></span>
          <span class="toggle">{doneOpen || filter === 'done' ? 'Hide' : `Show the ${section.rows.length} ›`}</span>
        </button>
      {:else}
        <div class="section {section.id}">{section.title} <span class="n">{section.rows.length}</span></div>
      {/if}
      {#if section.id !== 'done' || doneOpen || filter === 'done'}
        {#each section.rows as s (s.id)}
          {@const look = lookOf(s, waiting)}
          <button
            class="row {look}"
            class:picked={s.id === selected}
            aria-label="Subagent #{s.num} · {s.name} · {LABEL[look]}"
            title="Open the log"
            onclick={() => onview(s.id)}
          >
            <span class="dot {look}"></span>
            <span class="num">#{s.num}</span>
            <span class="name">{s.name}</span>
            <span class="last">{doing(s, look)}</span>
            <span class="right">{elapsed(s)}</span>
            <span class="right">{tokens(s.tokens)}</span>
          </button>
        {/each}
      {/if}
    {/each}
  </div>

  <div class="foot">
    <span class="totals">{subs.length} subagents · {tokens(total)} tokens <span>· included in the {cost(agent.cost)} of {agent.name}</span></span>
    {#if failed.length}
      <button class="relaunch" title="Fills in a message to {agent.name}" onclick={relaunch}>Ask to relaunch the {failed.length} failed</button>
    {/if}
  </div>
  {/if}
</SidePanel>

<style>
  .filters {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chips button {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 500;
  }

  .chips button[aria-pressed='true'] {
    background: var(--surface-5);
    box-shadow: inset 0 0 0 1px var(--border-focus);
    color: var(--text-strong);
  }

  .n {
    font: 500 11.5px var(--mono);
  }

  .dot {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .chips .dot {
    width: 8px;
    height: 8px;
  }

  .dot.all {
    background: var(--text-dim);
  }

  .running {
    --look: var(--sub-running);
  }

  .waiting {
    --look: var(--sub-waiting);
  }

  .error {
    --look: var(--sub-error);
  }

  .done {
    --look: var(--sub-done);
  }

  .stopped {
    --look: var(--sub-stopped);
  }

  .dot:not(.all),
  .bar span {
    background: var(--look);
  }

  .dot.waiting {
    box-shadow: 0 0 6px rgba(224, 138, 91, 0.8);
  }

  .dot.error {
    box-shadow: inset 0 0 0 2px var(--err-bg);
  }

  .bar {
    display: flex;
    height: 4px;
    border-radius: 2px;
    overflow: hidden;
    background: var(--surface-3);
  }

  .columns,
  .row {
    display: grid;
    grid-template-columns: 10px 34px minmax(80px, 180px) 1fr 44px 64px;
    align-items: center;
    gap: 10px;
  }

  .columns {
    flex: none;
    padding: 8px 28px 6px;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .columns span:nth-child(n + 5),
  .right {
    text-align: right;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0 16px 12px;
  }

  .section {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 10px 4px 6px;
    background: var(--bg-sidebar);
    font-size: 12px;
    font-weight: 700;
    text-align: left;
  }

  .section.handle {
    color: var(--accent-soft);
  }

  .section.running {
    color: var(--sub-running);
  }

  .section.done {
    color: var(--done);
  }

  .grow {
    flex: 1;
  }

  .toggle {
    font-weight: 500;
    color: var(--text-muted);
  }

  .row {
    width: 100%;
    height: 36px;
    margin-bottom: 2px;
    padding: 0 12px;
    border-radius: 4px;
    color: var(--text);
    text-align: left;
  }

  .row:hover {
    background: var(--surface-2);
  }

  .row.waiting {
    background: var(--wait-note);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .row.error {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
  }

  .row.done,
  .row.stopped {
    color: var(--text-3);
  }

  .row.picked {
    box-shadow: inset 0 0 0 2px var(--text);
  }

  .num,
  .last,
  .right {
    font: 11.5px var(--mono);
    color: var(--text-muted);
  }

  .name,
  .last {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    font-weight: 500;
  }

  .row.waiting .name,
  .row.error .name {
    font-weight: 600;
  }

  .row.waiting .last {
    color: var(--accent-soft);
  }

  .row.error .last {
    color: var(--err-text);
  }

  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-muted);
  }

  .totals {
    flex: 1;
    font-family: var(--mono);
  }

  .totals span {
    font-family: var(--sans);
  }

  .relaunch {
    height: 30px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
    font-weight: 600;
  }
</style>
