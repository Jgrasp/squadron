<script lang="ts">
  import { toolLabel } from './agents.svelte'
  import { tokens } from './format'
  import { t, type Key } from './i18n/index.svelte'
  import { pluginColor } from './plugins'
  import { openUsage } from './plugins.svelte'
  import { server, type Kind, type Use } from './pluginuse'
  import { settings } from './settings.svelte'
  import type { Agent, ToolCall } from './stream'

  let { agent, call, use, first = true, last = true }: { agent: Agent; call: ToolCall; use: Use; first?: boolean; last?: boolean } = $props()

  const NAMES: Record<Kind, Key> = { skill: 'plugins.kindSkill', agent: 'plugins.kindAgent', mcp: 'plugins.kindMcp' }
  const color = $derived(pluginColor(agent.plugins?.find((p) => p.name === use.plugin)?.id ?? use.plugin))
  const tool = $derived(use.kind === 'mcp' ? server(call, use.plugin) : toolLabel(call.name))
  const open = () => openUsage(agent.id, { use: use.id })
  const mode = $derived(settings.plugin_lines === 'card' && settings.plugin_card === 'minimal' ? 'minimal' : settings.plugin_lines)
</script>

{#snippet logo(size: string)}
  <span class="logo {size}" style:background={color} aria-hidden="true">{use.plugin.charAt(0).toUpperCase()}</span>
{/snippet}

{#snippet cost()}
  <span class="cost" class:hot={use.tokens >= 1000}>+{tokens(use.tokens)}</span>
{/snippet}

{#if mode === 'card'}
  <button class="card" style:--plugin={color} data-call={call.id} onclick={open}>
    {@render logo('md')}
    <span class="text">
      <span>{t('plugins.activated', { name: use.plugin, kind: t(NAMES[use.kind]) })}</span>
      <code>{use.label}</code>
    </span>
    <span class="big">+{tokens(use.tokens)}<small>tokens</small></span>
    <span class="go" aria-hidden="true">›</span>
  </button>
{:else if mode === 'minimal'}
  <button class="row rail" style:--plugin={color} data-call={call.id} onclick={open}>
    {@render logo('sm')}
    <span class="name"><b>{use.plugin}</b> › {tool}</span>
    <span class="arg">{use.label}</span>
    {@render cost()}
  </button>
{:else if mode === 'gutter'}
  <button class="gutter" style:--plugin={color} data-call={call.id} onclick={open}>
    <span class="lane" class:first class:last>{#if first}{@render logo('sm')}{/if}</span>
    <span class="row inner"><span class="name">{tool}</span><span class="arg">{use.label}</span></span>
    {@render cost()}
  </button>
{:else}
  <button class="row" data-call={call.id} onclick={open}>
    <span class="chip"><i style:background={color}></i>{use.plugin}</span>
    <span class="name">{tool}</span>
    <span class="arg">{use.label}</span>
    {@render cost()}
  </button>
{/if}

<style>
  button {
    width: 100%;
    text-align: left;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    border-radius: 4px;
    font-size: 12.5px;
  }

  button.row:hover,
  .gutter:hover .inner {
    background: var(--menu);
  }

  .name {
    flex: none;
    min-width: 40px;
    white-space: nowrap;
    color: var(--text-dim);
  }

  .arg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }

  .chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 8px;
    border-radius: 10px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 11.5px;
    white-space: nowrap;
  }

  .chip i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .cost {
    flex: none;
    min-width: 46px;
    text-align: right;
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .cost.hot {
    color: var(--accent-soft);
  }

  .logo {
    flex: none;
    display: grid;
    place-items: center;
    color: var(--ink);
    font-weight: 700;
  }

  .logo.sm {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    font-size: 9.5px;
  }

  .logo.md {
    width: 28px;
    height: 28px;
    border-radius: 7px;
    font-size: 13px;
  }

  .rail {
    margin: 2px 0;
    background: color-mix(in srgb, var(--plugin) 7%, transparent);
    box-shadow: inset 2px 0 0 var(--plugin);
  }

  button.rail:hover {
    background: color-mix(in srgb, var(--plugin) 13%, transparent);
  }

  .rail .name {
    color: var(--text-muted);
  }

  .rail .name b {
    color: var(--plugin);
    font-weight: 600;
  }

  .card {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 6px 0;
    padding: 10px 12px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--plugin) 6%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--plugin) 35%, transparent);
  }

  .card:hover {
    background: color-mix(in srgb, var(--plugin) 11%, var(--surface-1));
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .text code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim);
  }

  .big {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    font: 600 14px var(--mono);
    color: var(--accent-soft);
  }

  .big small {
    font: 10.5px var(--sans);
    color: var(--text-dim);
  }

  .go {
    color: var(--text-faint);
    font-size: 16px;
  }

  .gutter {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto;
    align-items: stretch;
    column-gap: 8px;
  }

  .inner {
    border-radius: 4px;
  }

  .lane {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .lane::before {
    content: '';
    position: absolute;
    left: 50%;
    top: -16px;
    bottom: -16px;
    width: 2px;
    margin-left: -1px;
    background: color-mix(in srgb, var(--plugin) 55%, transparent);
  }

  .lane.first::before {
    top: 50%;
  }

  .lane.last::before {
    bottom: 50%;
  }

  .lane.first.last::before {
    display: none;
  }

  .lane .logo {
    position: relative;
  }

  .gutter .cost {
    align-self: center;
  }
</style>
