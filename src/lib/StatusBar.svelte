<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { clock, limits, type Limit } from './agents.svelte'
  import { time } from './format'
  import { install, update } from './update.svelte'

  type Load = { cpu: number; used: number; total: number }

  let load = $state<Load | null>(null)

  onMount(() => {
    const read = () => invoke<Load | null>('system_load').then((l) => (load = l))
    read()
    const timer = setInterval(read, 2000)
    return () => clearInterval(timer)
  })

  const gb = (bytes: number) => (bytes / 2 ** 30).toFixed(1)
  const level = (ratio: number) => (ratio >= 0.9 ? 'high' : ratio >= 0.75 ? 'warm' : '')

  function resets(at: number) {
    const date = new Date(at)
    const soon = Math.abs(at - clock.now) < 20 * 3600_000
    return soon ? time(at) : `${date.toLocaleDateString('en-GB', { weekday: 'short' })} ${time(at)}`
  }

  const windows = $derived(
    (
      [
        ['Session', 'Current 5-hour window of your Claude plan', limits.five_hour],
        ['Week', 'Current 7-day window of your Claude plan', limits.seven_day],
      ] as [string, string, Limit | undefined][]
    ).filter((w): w is [string, string, Limit] => !!w[2]),
  )
</script>

{#snippet meter(ratio: number)}
  <span class="meter {level(ratio)}" aria-hidden="true"><span style:width="{Math.min(ratio, 1) * 100}%"></span></span>
{/snippet}

<footer class="status">
  {#if load}
    <span class="item" title="Processor load">
      CPU {@render meter(load.cpu / 100)}<span class="mono">{Math.round(load.cpu)}%</span>
    </span>
    <span class="item" title="Memory in use">
      Memory {@render meter(load.used / load.total)}<span class="mono">{gb(load.used)} / {gb(load.total)} GB</span>
    </span>
  {/if}
  {#if update.found}
    <button class="update" title={update.error || `Download ${update.found.version}, install it and restart Squadron`} onclick={install}>
      {#if update.progress !== null}
        Downloading {update.found.version}… {Math.round(update.progress * 100)}%
      {:else if update.error}
        Update failed · retry
      {:else}
        Squadron {update.found.version} is available · Update
      {/if}
    </button>
  {/if}
  <span class="spacer"></span>
  {#each windows as [label, hint, limit] (label)}
    {@const at = limit.resetsAt * 1000}
    <span class="item" title={hint}>
      {label}
      {#if at > clock.now}
        {@render meter(limit.utilization)}<span class="mono">{Math.round(limit.utilization * 100)}%</span>
        <span class="reset">resets {resets(at)}</span>
      {:else}
        <span class="reset">reset at {resets(at)}</span>
      {/if}
    </span>
  {:else}
    <span class="item reset">Plan usage shows up once an agent has run</span>
  {/each}
</footer>

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

  .update {
    height: 20px;
    padding: 0 8px;
    border-radius: 3px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 600;
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
