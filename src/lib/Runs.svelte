<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from './agents.svelte'
  import { ago, duration } from './format'
  import { editProject, type Project } from './projects.svelte'
  import { active, dismiss, shown, type Run } from './pipelines.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { t, type Key } from './i18n/index.svelte'

  let { project }: { project: Project } = $props()
  let menu = $state<string | null>(null)

  const list = $derived(shown(project))

  const STATUS: Record<Run['status'], Key> = {
    queued: 'runs.queued',
    running: 'runs.running',
    success: 'runs.success',
    failure: 'runs.failure',
    cancelled: 'runs.cancelled',
    skipped: 'runs.cancelled',
  }

  const since = (run: Run) =>
    active(run) ? duration(Math.max(0, clock.now - Date.parse(run.started))) : ago(Date.parse(run.finished ?? run.started), clock.now)

  function hideHere(run: Run) {
    menu = null
    editProject(project.path, { runs_hidden: [...project.runs_hidden, run.name] })
  }

  function hideEverywhere(run: Run) {
    menu = null
    saveSettings({ ...$state.snapshot(settings), runs_hidden: [...settings.runs_hidden, run.name] })
  }
</script>

<svelte:window onclick={(e) => !(e.target as HTMLElement).closest('[data-menu]') && (menu = null)} />

{#if list.length}
  <div class="runs" style:--project={project.color} role="region" aria-label={t('runs.title')}>
    {#each list as run (run.id)}
      <div class="run {run.status}" class:active={active(run)}>
        <button class="main" title={run.url} onclick={() => invoke('open_link', { url: run.url })}>
          <span class="state" role="img" aria-label={t(STATUS[run.status])}>
            {#if run.status === 'running'}
              <span class="spinner"></span>
            {:else if run.status === 'queued'}
              <svg width="12" height="12" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></svg>
            {:else if run.status === 'success'}
              <svg width="12" height="12" viewBox="0 0 24 24"><path d="M5 12l5 5 9-10" /></svg>
            {:else if run.status === 'failure'}
              <svg width="12" height="12" viewBox="0 0 24 24"><path d="M6 6l12 12M18 6L6 18" /></svg>
            {:else}
              <svg width="12" height="12" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M6 18L18 6" /></svg>
            {/if}
          </span>
          <strong>{run.name || `#${run.number}`}</strong>
          {#if run.title}<span class="title">{run.title}</span>{/if}
          {#if run.branch}<code>{run.branch}</code>{/if}
          <span class="time">{t(STATUS[run.status])} · {since(run)}</span>
        </button>
        <div class="anchor" data-menu>
          <button class="icon" aria-label={t('runs.more', { name: run.name })} aria-haspopup="menu" aria-expanded={menu === run.id} onclick={() => (menu = menu === run.id ? null : run.id)}>
            <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.5" /><circle cx="12" cy="12" r="1.5" /><circle cx="19" cy="12" r="1.5" /></svg>
          </button>
          {#if menu === run.id}
            <div class="menu" role="menu">
              <button role="menuitem" onclick={() => hideHere(run)}>{t('runs.hideHere', { name: run.name })}</button>
              <button role="menuitem" onclick={() => hideEverywhere(run)}>{t('runs.hideEverywhere', { name: run.name })}</button>
              <button role="menuitem" onclick={() => ((menu = null), editProject(project.path, { runs: false }))}>{t('runs.offHere')}</button>
            </div>
          {/if}
        </div>
        {#if !active(run)}
          <button class="icon" aria-label={t('runs.dismiss')} title={t('runs.dismiss')} onclick={() => dismiss(project, run)}>
            <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .runs {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
  }

  .run {
    flex: none;
    display: flex;
    align-items: center;
    max-width: 520px;
    height: 30px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
  }

  .run.running {
    box-shadow: inset 0 0 0 1px var(--project, var(--accent));
  }

  .run.success {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 2px var(--ok);
    color: var(--ok-text);
    animation: landed 1.6s ease-out 3;
    --glow: var(--ok);
  }

  .run.failure {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 2px var(--err);
    color: var(--err-text);
    animation: landed 1.6s ease-out 3;
    --glow: var(--err);
  }

  .run.cancelled {
    background: var(--surface-3);
    color: var(--text-muted);
  }

  @keyframes landed {
    0% {
      box-shadow:
        inset 0 0 0 2px var(--glow),
        0 0 0 0 var(--glow);
    }
    100% {
      box-shadow:
        inset 0 0 0 2px var(--glow),
        0 0 0 8px transparent;
    }
  }

  .main {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    padding: 0 6px 0 10px;
    white-space: nowrap;
  }

  .state {
    flex: none;
    display: inline-flex;
  }

  .state svg,
  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon circle {
    fill: currentColor;
    stroke: none;
  }

  .spinner {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    border-top-color: var(--project, var(--accent));
    animation: spin 0.9s linear infinite;
  }

  strong {
    font-weight: 600;
    color: inherit;
  }

  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .success .title,
  .failure .title {
    color: inherit;
    opacity: 0.8;
  }

  code {
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .time {
    font-size: 12px;
    color: var(--text-dim);
  }

  .success .time,
  .failure .time {
    color: inherit;
  }

  .anchor {
    position: relative;
    height: 100%;
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 100%;
    color: inherit;
    opacity: 0.7;
  }

  .icon:last-child {
    margin-right: 4px;
  }

  .icon:hover {
    opacity: 1;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: -8px;
    z-index: 20;
    width: max-content;
    max-width: 420px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .menu button {
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
  }
</style>
