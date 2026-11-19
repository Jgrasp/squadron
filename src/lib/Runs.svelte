<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from './agents.svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import { isCheck, pullOf } from './checks'
  import { ago, duration, time } from './format'
  import { editProject, type Project } from './projects.svelte'
  import { active, dismiss, shown, type Run } from './pipelines.svelte'
  import { hostName as host, lists, lookUpLists } from './pulls.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { t, type Key } from './i18n/index.svelte'

  let { project }: { project: Project } = $props()
  let open = $state(false)
  let pinned = $state(false)
  let menu = $state<string | null>(null)
  let filter = $state<'all' | 'pipelines' | 'checks'>('all')
  let leaving: ReturnType<typeof setTimeout> | undefined

  const list = $derived(shown(project))
  const pulls = $derived(lists[project.path] ? [...lists[project.path].mine, ...lists[project.path].review, ...lists[project.path].others] : [])
  const checks = $derived(list.filter((r) => isCheck(r, pulls)).length)
  const filtered = $derived(filter === 'all' ? list : list.filter((r) => isCheck(r, pulls) === (filter === 'checks')))
  const running = $derived(filtered.filter(active))
  const ended = $derived(filtered.filter((r) => !active(r)).sort((a, b) => Number(b.status === 'failure') - Number(a.status === 'failure')))
  const filters = $derived([
    ['all', 'runs.all', list.length],
    ['pipelines', 'runs.pipelines', list.length - checks],
    ['checks', 'runs.checks', checks],
  ] as const)
  const count = (status: Run['status'][]) => list.filter((r) => status.includes(r.status)).length
  const tone = $derived(count(['failure']) ? 'failure' : list.some(active) ? 'running' : count(['success']) ? 'success' : 'cancelled')
  const lead = $derived(
    ((count(['failure']) && 'failure') || (count(['running']) && 'running') || (count(['queued']) && 'queued') || (count(['success']) && 'success') || 'cancelled') as Run['status'],
  )

  const STATUS: Record<Run['status'], Key> = {
    queued: 'runs.queued',
    running: 'runs.running',
    success: 'runs.success',
    failure: 'runs.failure',
    cancelled: 'runs.cancelled',
    skipped: 'runs.cancelled',
  }

  const SUMMARY: [Run['status'][], Key][] = [
    [['running'], 'runs.countRunning'],
    [['queued'], 'runs.countQueued'],
    [['failure'], 'runs.countFailed'],
    [['success'], 'runs.countPassed'],
    [['cancelled', 'skipped'], 'runs.countStopped'],
  ]

  const summary = $derived(
    SUMMARY.filter(([status]) => count(status))
      .map(([status, key]) => t(key, { count: count(status) }))
      .join(' · '),
  )

  const took = (run: Run) => duration(Math.max(0, (run.finished ? Date.parse(run.finished) : clock.now) - Date.parse(run.started)))

  function enter() {
    clearTimeout(leaving)
    if (!open) lookUpLists(project.path)
    open = true
  }

  function leave() {
    if (pinned || menu) return
    leaving = setTimeout(() => (open = false), 220)
  }

  function close() {
    open = pinned = false
    menu = null
  }

  function hideHere(run: Run) {
    menu = null
    editProject(project.path, { runs_hidden: [...project.runs_hidden, run.name] })
  }

  function hideEverywhere(run: Run) {
    menu = null
    saveSettings({ ...$state.snapshot(settings), runs_hidden: [...settings.runs_hidden, run.name] })
  }
</script>

<svelte:window
  onclick={(e) => {
    const target = e.target as HTMLElement
    if (!target.closest('[data-menu]')) menu = null
    if (!target.closest('.runs')) close()
  }}
  onkeydown={(e) => e.key === 'Escape' && open && close()}
/>

{#snippet icon(status: Run['status'])}
  {#if status === 'running'}
    <span class="spinner" aria-hidden="true"></span>
  {:else}
    <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true">
      {#if status === 'queued'}<circle cx="12" cy="12" r="8" /><path d="M12 8v4l2.5 2" />
      {:else if status === 'success'}<path d="M5 12.5l4.5 4.5L19 7.5" />
      {:else if status === 'failure'}<path d="M7 7l10 10M17 7L7 17" />
      {:else}<circle cx="12" cy="12" r="8" /><path d="M7 17L17 7" />{/if}
    </svg>
  {/if}
{/snippet}

{#snippet card(run: Run)}
  {@const pull = pullOf(run, pulls)}
  {@const number = run.pull ?? pull?.number}
  <li class="card {run.status}" class:menu-open={menu === run.id}>
    {#if run.author}<span class="who" title={run.author}><Avatar name={run.author} src={run.author_avatar} size={24} /></span>{/if}
    <div class="body">
      <div class="top">
        <strong>{run.name || `#${run.number}`}</strong>
        <span class="number">#{run.number}</span>
      </div>
      {#if number || run.title}
        <p class="title">
          {#if number}<span class="pull" title={t('runs.checksOf', { number })}><Icon name="pull" size={11} />#{number}</span>{/if}
          <span class="text">{pull?.title || run.title}</span>
        </p>
      {/if}
      <div class="meta">
        {#if run.branch}<code>{run.branch}</code>{/if}
        {#if run.event}<span>{run.event}</span>{/if}
        {#if run.status === 'running'}
          <span>{t('runs.startedAt', { time: time(Date.parse(run.started)) })}</span>
        {:else if !active(run)}
          <span>{t('runs.endedAgo', { ago: ago(Date.parse(run.finished ?? run.started), clock.now) })}</span>
        {/if}
      </div>
    </div>
    <div class="side">
      <span class="state">{@render icon(run.status)}{t(STATUS[run.status])}</span>
      <span class="took">{took(run)}</span>
      <div class="actions">
        <button class="btn btn-ghost out" onclick={() => invoke('open_link', { url: run.url })}>{t('runs.openOn', { host: host(run) })}</button>
        <div class="anchor" data-menu>
          <button class="icon" aria-label={t('runs.more', { name: run.name })} aria-haspopup="menu" aria-expanded={menu === run.id} onclick={() => (menu = menu === run.id ? null : run.id)}>
            <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.6" /><circle cx="12" cy="12" r="1.6" /><circle cx="19" cy="12" r="1.6" /></svg>
          </button>
          {#if menu === run.id}
            <div class="menu" role="menu">
              <button role="menuitem" onclick={() => hideHere(run)}>{t('runs.hideHere', { name: run.name })}</button>
              <button role="menuitem" onclick={() => hideEverywhere(run)}>{t('runs.hideEverywhere', { name: run.name })}</button>
              <button role="menuitem" onclick={() => (close(), editProject(project.path, { runs: false }))}>{t('runs.offHere')}</button>
            </div>
          {/if}
        </div>
        {#if !active(run)}
          <button class="icon" title={t('runs.dismiss')} aria-label={t('runs.dismiss')} onclick={() => dismiss(project, run)}>
            <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7L7 17" /></svg>
          </button>
        {/if}
      </div>
    </div>
  </li>
{/snippet}

{#if list.length}
  <div class="runs" style:--project={project.color} role="region" aria-label={t('runs.title')} onmouseenter={enter} onmouseleave={leave}>
    <button
      class="btn btn-ghost summary {tone === 'running' ? '' : tone}"
      aria-label="{t('runs.title')} · {summary}"
      title={summary}
      aria-expanded={open}
      onclick={() => (pinned ? close() : ((pinned = true), enter()))}
    >
      <Icon name="pipeline" />
      {t('runs.button')}
      <span class="count {lead}">{#if lead === 'running'}<span class="spinner" aria-hidden="true"></span>{/if}{count(lead === 'cancelled' ? ['cancelled', 'skipped'] : [lead])}</span>
    </button>
    {#if open}
      <div class="panel" role="dialog" aria-label={t('runs.title')}>
        <header>
          <span class="name">{t('runs.title')}</span>
          <span class="project"><span class="square" aria-hidden="true"></span>{project.name}</span>
        </header>
        {#if checks && checks < list.length}
          <div class="filters" role="group">
            {#each filters as [id, key, n] (id)}
              <button aria-pressed={filter === id} onclick={() => (filter = id)}>{t(key)}<span class="n">{n}</span></button>
            {/each}
          </div>
        {/if}
        {#if running.length}
          <h3>{t('runs.sectionRunning')}<span>{running.length}</span></h3>
          <ul>{#each running as run (run.id)}{@render card(run)}{/each}</ul>
        {/if}
        {#if ended.length}
          <h3>{t('runs.sectionEnded', { minutes: settings.runs_linger })}<span>{ended.length}</span></h3>
          <ul>{#each ended as run (run.id)}{@render card(run)}{/each}</ul>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .runs {
    position: relative;
    display: flex;
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .spinner {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 1.5px solid var(--border-strong);
    border-top-color: var(--project, var(--accent));
    animation: spin 0.9s linear infinite;
  }

  .summary:hover,
  .summary[aria-expanded='true'] {
    background: var(--surface-3);
  }

  .summary.success {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok);
    animation: landed 1.6s ease-out 3;
    --glow: var(--ok);
  }

  .summary.failure {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err);
    animation: landed 1.6s ease-out 3;
    --glow: var(--err);
  }

  @keyframes landed {
    0% {
      box-shadow:
        inset 0 0 0 1px var(--glow),
        0 0 0 0 var(--glow);
    }
    100% {
      box-shadow:
        inset 0 0 0 1px var(--glow),
        0 0 0 7px transparent;
    }
  }

  .count {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 4px;
    font: 700 11px/18px var(--mono);
  }

  .count.running {
    background: color-mix(in srgb, var(--project, var(--accent)) 22%, transparent);
    color: var(--text-strong);
  }

  .count.running .spinner {
    width: 8px;
    height: 8px;
    border-color: color-mix(in srgb, var(--text-strong) 25%, transparent);
    border-top-color: var(--text-strong);
  }

  .count.queued,
  .count.cancelled {
    background: var(--surface-4);
    color: var(--text-2);
  }

  .count.failure {
    background: var(--err);
    color: var(--err-ink);
  }

  .count.success {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--ok-text);
  }

  .panel {
    position: absolute;
    top: 38px;
    right: 0;
    z-index: 20;
    text-align: left;
    width: 460px;
    max-height: min(560px, 70vh);
    overflow-y: auto;
    padding: 6px 6px 8px;
    border-radius: 8px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 18px 44px rgba(0, 0, 0, 0.55);
  }

  .panel header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 6px;
  }

  .panel header .name {
    font-weight: 600;
    color: var(--text-strong);
    white-space: nowrap;
  }

  .project {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .square {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .filters {
    display: inline-flex;
    gap: 2px;
    margin: 2px 10px 4px;
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .filters button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border-radius: 3px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .filters button:hover {
    color: var(--text-strong);
  }

  .filters button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .filters .n {
    font: 11px var(--mono);
    opacity: 0.7;
  }

  h3 {
    display: flex;
    justify-content: space-between;
    margin: 10px 10px 6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  h3 span {
    font: 11px var(--mono);
    letter-spacing: 0;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    --tone: var(--text-faint);
    position: relative;
    display: flex;
    gap: 12px;
    padding: 10px 12px 10px 14px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .card::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    border-radius: 6px 0 0 6px;
    background: var(--tone);
  }

  .card:hover,
  .card.menu-open {
    background: var(--surface-2);
  }

  .card.running {
    --tone: var(--project, var(--accent));
  }

  .card.success {
    --tone: var(--ok);
  }

  .card.failure {
    --tone: var(--err);
  }

  .card.cancelled,
  .card.skipped {
    --tone: var(--surface-5);
  }

  .who {
    flex: none;
    margin-top: 1px;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .top strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .number {
    flex: none;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    margin: 0;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .title .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pull {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 18px;
    padding: 0 6px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--remote-text) 12%, transparent);
    font: 600 11.5px var(--mono);
    color: var(--remote-text);
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .meta code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 11.5px;
    color: var(--text-2);
  }

  .side {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    justify-content: space-between;
    gap: 6px;
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--tone);
  }

  .card.queued .state,
  .card.cancelled .state,
  .card.skipped .state {
    color: var(--text-muted);
  }

  .took {
    font: 600 12.5px var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--text-strong);
  }

  .card.queued .took,
  .card.cancelled .took,
  .card.skipped .took {
    font-weight: 400;
    color: var(--text-muted);
  }

  .actions {
    display: none;
    align-items: center;
    gap: 2px;
  }

  .card:hover .actions,
  .card:focus-within .actions,
  .card.menu-open .actions {
    display: flex;
  }

  .card:hover .took,
  .card:focus-within .took,
  .card.menu-open .took {
    display: none;
  }

  .out {
    height: 24px;
    margin-right: 2px;
    padding: 0 8px;
    font-size: 12px;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .icon:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .icon circle {
    fill: currentColor;
    stroke: none;
  }

  .anchor {
    position: relative;
  }

  .menu {
    position: absolute;
    top: 30px;
    right: 0;
    z-index: 2;
    width: max-content;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .menu button {
    height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
  }
</style>
