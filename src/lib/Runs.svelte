<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from './agents.svelte'
  import { ago, duration, time } from './format'
  import { editProject, type Project } from './projects.svelte'
  import { active, dismiss, shown, type Run } from './pipelines.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { t, type Key } from './i18n/index.svelte'

  let { project, compact = false }: { project: Project; compact?: boolean } = $props()
  let open = $state(false)
  let pinned = $state(false)
  let menu = $state<string | null>(null)
  let leaving: ReturnType<typeof setTimeout> | undefined

  const list = $derived(shown(project))
  const running = $derived(list.filter(active))
  const ended = $derived(list.filter((r) => !active(r)).sort((a, b) => Number(b.status === 'failure') - Number(a.status === 'failure')))
  const count = (status: Run['status'][]) => list.filter((r) => status.includes(r.status)).length
  const tone = $derived(count(['failure']) ? 'failure' : running.length ? 'running' : count(['success']) ? 'success' : 'cancelled')

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

  const took = (run: Run) => duration(Math.max(0, (run.finished ? Date.parse(run.finished) : clock.now) - Date.parse(run.started)))
  const host = (run: Run) => (run.host === 'github' ? 'GitHub' : 'Bitbucket')

  function enter() {
    clearTimeout(leaving)
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
    <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
      {#if status === 'queued'}<circle cx="12" cy="12" r="8" /><path d="M12 8v4l2.5 2" />
      {:else if status === 'success'}<path d="M5 12.5l4.5 4.5L19 7.5" />
      {:else if status === 'failure'}<path d="M7 7l10 10M17 7L7 17" />
      {:else}<circle cx="12" cy="12" r="8" /><path d="M7 17L17 7" />{/if}
    </svg>
  {/if}
{/snippet}

{#snippet item(run: Run)}
  <li class="item {run.status}">
    <span class="badge">{@render icon(run.status)}</span>
    <div class="body">
      <div class="head">
        <strong>{run.name || `#${run.number}`}</strong>
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
        <span class="state">{t(STATUS[run.status])}</span>
      </div>
      {#if run.title}<p class="title">{run.title}</p>{/if}
      <div class="meta">
        {#if run.branch}<code>{run.branch}</code>{/if}
        {#if run.event}<span>{run.event}</span>{/if}
        <span>#{run.number}</span>
        <span class="when">
          {#if run.status === 'queued'}
            {t('runs.waitingSince', { duration: took(run) })}
          {:else if active(run)}
            {t('runs.startedAt', { time: time(Date.parse(run.started)), duration: took(run) })}
          {:else}
            {t('runs.endedAgo', { ago: ago(Date.parse(run.finished ?? run.started), clock.now), duration: took(run) })}
          {/if}
        </span>
      </div>
    </div>
  </li>
{/snippet}

{#if list.length}
  <div class="runs" style:--project={project.color} role="region" aria-label={t('runs.title')} onmouseenter={enter} onmouseleave={leave}>
    <button class="btn btn-ghost summary {tone}" class:compact aria-label={t('runs.title')} aria-expanded={open} onclick={() => (pinned ? close() : (pinned = open = true))}>
      {#each SUMMARY as [status, key] (key)}
        {#if count(status)}
          <span class="part {status[0]}">{@render icon(status[0])}{compact ? count(status) : t(key, { count: count(status) })}</span>
        {/if}
      {/each}
      {#if !compact}<svg class="chevron" width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>{/if}
    </button>
    {#if open}
      <div class="panel" role="dialog" aria-label={t('runs.title')}>
        <header>
          <span>{t('runs.title')}</span>
          <span class="project"><span class="square" aria-hidden="true"></span>{project.name}</span>
        </header>
        {#if running.length}
          <h3>{t('runs.sectionRunning')}</h3>
          <ul>{#each running as run (run.id)}{@render item(run)}{/each}</ul>
        {/if}
        {#if ended.length}
          <h3>{t('runs.sectionEnded', { minutes: settings.runs_linger })}</h3>
          <ul>{#each ended as run (run.id)}{@render item(run)}{/each}</ul>
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
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    border-top-color: var(--project, var(--accent));
    animation: spin 0.9s linear infinite;
  }

  .summary {
    display: inline-flex;
    align-items: center;
    gap: 14px;
  }

  .summary.compact {
    gap: 8px;
    padding: 0 9px;
  }

  .summary:hover,
  .summary[aria-expanded='true'] {
    background: var(--surface-3);
  }

  .summary.running {
    box-shadow: inset 0 0 0 1px var(--project, var(--accent));
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

  .part {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }

  .part.failure {
    color: var(--err-text);
  }

  .part.success {
    color: var(--ok-text);
  }

  .part.queued,
  .part.cancelled {
    color: var(--text-muted);
  }

  .chevron {
    margin-left: -6px;
    color: var(--text-dim);
  }

  .panel {
    position: absolute;
    top: 38px;
    right: 0;
    z-index: 20;
    text-align: left;
    width: 440px;
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
    justify-content: space-between;
    padding: 8px 10px 6px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .project {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-weight: 400;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .square {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  h3 {
    margin: 8px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .item {
    --tone: var(--text-muted);
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 11px;
    padding: 9px 8px 9px 10px;
    border-radius: 6px;
  }

  .item:hover {
    background: var(--menu-hover);
  }

  .item.running {
    --tone: var(--project, var(--accent));
  }

  .item.success {
    --tone: var(--ok);
  }

  .item.failure {
    --tone: var(--err);
  }

  .badge .spinner {
    width: 12px;
    height: 12px;
  }

  .badge {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-top: 1px;
    border-radius: 50%;
    color: var(--tone);
    background: color-mix(in srgb, var(--tone) 16%, transparent);
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .head {
    display: flex;
    align-items: center;
    height: 20px;
    gap: 8px;
  }

  .head strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .state {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 8px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--tone) 16%, transparent);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--tone);
  }

  .item.queued .state,
  .item.cancelled .state {
    background: var(--surface-3);
    color: var(--text-muted);
  }

  .title {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .meta code {
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 11px;
    color: var(--text-2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    margin: -4px 4px -4px auto;
    opacity: 0;
  }

  .item:hover .actions,
  .item:focus-within .actions {
    opacity: 1;
  }

  .out {
    height: 24px;
    margin-right: 4px;
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
