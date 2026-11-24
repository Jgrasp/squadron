<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { clock, limits, type Limit } from './agents.svelte'
  import { time } from './format'
  import { update } from './update.svelte'
  import { date, number, t, type Key } from './i18n/index.svelte'
  import Updates from './Updates.svelte'
  import { ui } from './actions.svelte'
  import { updatesOf } from './extensions'
  import { catalog, extensions } from './extensions.svelte'
  import { settings } from './settings.svelte'
  import { projects, toggleUsage } from './projects.svelte'
  import Icon from './Icon.svelte'

  type Load = { cpu: number; used: number; total: number }

  let load = $state<Load | null>(null)
  let version = $state('')
  let claude = $state('')
  let tested = $state('')

  const extensionUpdates = $derived(updatesOf(extensions.list, catalog.entries ?? [], extensions.origins).length)

  $effect(() => {
    const path = settings.claude
    const check = () =>
      invoke<string>('claude_version', { path })
        .then((v) => (claude = v.split(' ')[0]))
        .catch(() => (claude = ''))
    check()
    const timer = setInterval(check, 600_000)
    return () => clearInterval(timer)
  })

  onMount(() => {
    invoke<string>('tested_claude_version')
      .then((v) => (tested = v))
      .catch(() => {})
    getVersion()
      .then((v) => (version = v))
      .catch(() => {})
    const read = () => invoke<Load | null>('system_load').then((l) => (load = l))
    read()
    const timer = setInterval(read, 2000)
    return () => clearInterval(timer)
  })

  const gb = (bytes: number) => number(bytes / 2 ** 30, { minimumFractionDigits: 1, maximumFractionDigits: 1 })
  const percent = (ratio: number) => number(ratio, { style: 'percent', maximumFractionDigits: 0 })
  const level = (ratio: number) => (ratio >= 0.9 ? 'high' : ratio >= 0.75 ? 'warm' : '')

  function resets(at: number, soon: Key, later: Key) {
    return Math.abs(at - clock.now) < 20 * 3600_000 ? t(soon, { time: time(at) }) : t(later, { day: date(at, { weekday: 'short' }), time: time(at) })
  }

  const windows = $derived(
    (
      [
        [t('shell.limitSession'), t('shell.limitSessionHint'), limits.five_hour],
        [t('shell.limitWeek'), t('shell.limitWeekHint'), limits.seven_day],
      ] as [string, string, Limit | undefined][]
    ).filter((w): w is [string, string, Limit] => !!w[2]),
  )
</script>

{#snippet meter(ratio: number)}
  <span class="meter {level(ratio)}" aria-hidden="true"><span style:width="{Math.min(ratio, 1) * 100}%"></span></span>
{/snippet}

<footer class="status">
  {#if version}<span class="item">Squadron <span class="mono">{version}</span></span>{/if}
  {#if claude && tested && claude !== tested}
    <span class="item" title={t('shell.claudeUntestedTitle', { tested })}>Claude Code <span class="mono">{claude}</span><span class="untested">{t('shell.claudeUntested')}</span></span>
  {/if}
  {#if load}
    <span class="item" title={t('shell.processorLoad')}>
      {t('shell.cpu')} {@render meter(load.cpu / 100)}<span class="mono">{percent(load.cpu / 100)}</span>
    </span>
    <span class="item" title={t('shell.memoryInUse')}>
      {t('shell.memory')} {@render meter(load.used / load.total)}<span class="mono">{t('shell.gigabytes', { used: gb(load.used), total: gb(load.total) })}</span>
    </span>
  {/if}
  {#if update.found || extensionUpdates}
    <button class="updates" title={update.error || undefined} onclick={() => (ui.updates = true)}>
      {#if update.found}
        <span class="squadron">
          {#if update.progress !== null}
            {t('shell.downloading', { version: update.found.version, percent: percent(update.progress) })}
          {:else if update.error}
            {t('shell.updateFailed')}
          {:else if extensionUpdates}
            Squadron {update.found.version}
          {:else}
            {t('shell.updateAvailable', { version: update.found.version })}
          {/if}
        </span>
      {/if}
      {#if extensionUpdates}
        <span class="extensions">{update.found ? t('shell.moreExtensions', { count: extensionUpdates }) : t('shell.extensionUpdates', { count: extensionUpdates })}</span>
      {/if}
    </button>
  {/if}
  <span class="spacer"></span>
  {#if settings.usage}
    <button class="limits" aria-pressed={projects.usage} title={t('stats.openTitle')} onclick={toggleUsage}>
      {@render limitsOf()}
      <Icon name="next" size={12} />
    </button>
  {:else}
    {@render limitsOf()}
  {/if}
</footer>

{#snippet limitsOf()}
  {#each windows as [label, hint, limit] (label)}
    {@const at = limit.resetsAt * 1000}
    <span class="item" title={hint}>
      {label}
      {#if at > clock.now}
        {@render meter(limit.utilization)}<span class="mono">{percent(limit.utilization)}</span>
        <span class="reset">{resets(at, 'shell.resetsTime', 'shell.resetsDay')}</span>
      {:else}
        <span class="reset">{resets(at, 'shell.resetAtTime', 'shell.resetAtDay')}</span>
      {/if}
    </span>
  {:else}
    <span class="item reset">{t('shell.planUsageEmpty')}</span>
  {/each}
{/snippet}

{#if ui.updates}<Updates onclose={() => (ui.updates = false)} />{/if}

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 18px;
    flex-shrink: 0;
    height: 26px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-sidebar);
    color: var(--text-dim);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
  }

  .item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .mono {
    color: var(--text-2);
  }

  .updates {
    display: inline-flex;
    height: 20px;
    border-radius: 3px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    font-size: 11.5px;
    font-weight: 600;
  }

  .updates span {
    display: inline-flex;
    align-items: center;
    padding: 0 8px;
  }

  .squadron {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .extensions {
    background: var(--wait-bg);
    color: var(--wait-btn-text);
  }

  .updates:hover .squadron {
    background: var(--accent-soft);
  }

  .updates:hover .extensions {
    background: var(--wait-btn);
  }

  .untested {
    color: var(--err-text);
  }

  .limits {
    display: inline-flex;
    align-items: center;
    gap: 18px;
    height: 20px;
    margin-right: -8px;
    padding: 0 8px;
    border-radius: 3px;
    color: inherit;
    font: inherit;
  }

  .limits:hover,
  .limits[aria-pressed='true'] {
    background: var(--surface-2);
  }

  .limits > :global(.icon) {
    margin-left: -10px;
    color: var(--text-faint);
  }

  .limits:hover > :global(.icon) {
    color: var(--text-2);
  }

  .spacer {
    flex: 1;
  }

  .reset {
    color: var(--text-faint);
  }

  .meter {
    display: inline-block;
    width: 44px;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-2);
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--ok-line);
  }

  .meter.warm span {
    background: var(--accent);
  }

  .meter.high span {
    background: var(--err);
  }
</style>
