<script lang="ts">
  import { agents, clock, ofProject, rename, waiting, waitingDetail } from './agents.svelte'
  import { ago, cost } from './format'
  import { layout, toggleSidebar } from './layout.svelte'
  import { home, type Project } from './projects.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import type { Agent } from './stream'

  let {
    project,
    current,
    onnew,
    onretire,
  }: { project: Project; current?: Agent; onnew: () => void; onretire: () => void } = $props()

  let editing = $state('')
  let newName = $state('')

  function startRename(agent: Agent) {
    newName = agent.name
    editing = agent.id
  }

  function finishRename(save: boolean) {
    if (save && editing) rename(editing, newName)
    editing = ''
  }

  let hovered = $state<{ agent: Agent; top: number; left: number } | null>(null)

  function hover(agent: Agent, event: Event) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    hovered = { agent, top: rect.top - 6, left: rect.right + 12 }
  }

  function short(name: string) {
    const words = name.split(/\s+/).filter(Boolean)
    const id = words.find((w) => /\d/.test(w))
    if (id) return (id.includes('-') && /^[A-Za-z]+-/.test(id) ? id.split('-')[0] : id).slice(0, 4)
    return words
      .slice(0, 3)
      .map((w) => w[0])
      .join('')
      .toUpperCase()
  }

  const list = $derived(ofProject(project.path))
  const pending = $derived(waiting(project.path))
  const total = $derived(list.reduce((sum, a) => sum + a.cost, 0))

  const blocked = (agent: Agent) => !!agent.attention && agent.status !== 'working' && agent.status !== 'error' && !agent.asks.length
  const look = (agent: Agent) => (blocked(agent) ? 'waiting' : agent.status)

  function label(agent: Agent) {
    if (blocked(agent)) return 'Waiting for you'
    switch (agent.status) {
      case 'working':
        return agent.action
      case 'done':
        return `Done ${ago(agent.since, clock.now)}`
      case 'interrupted':
        return 'Interrupted · waiting for your instruction'
      case 'waiting':
        return agent.action
      case 'error':
        return 'Error · session stopped'
    }
  }
</script>

{#snippet card(agent: Agent)}
  <span class="line">
    <StatusIcon status={look(agent)} />
    {#if editing === agent.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="rename"
        aria-label="Agent name"
        bind:value={newName}
        autofocus
        onkeydown={(e) => (e.key === 'Enter' ? finishRename(true) : e.key === 'Escape' && finishRename(false))}
        onblur={() => finishRename(true)}
      />
    {:else}
      <span class="name">{agent.name}</span>
    {/if}
    <span class="cost">{cost(agent.cost)}</span>
  </span>
  <span class="state">{label(agent)}</span>
  {#if agent.status === 'error' && agent.action}
    <span class="detail">{agent.action}</span>
  {:else if agent.status === 'waiting'}
    <span class="detail">{waitingDetail(agent)}</span>
  {:else if blocked(agent)}
    <span class="detail">{agent.attention}</span>
  {/if}
  {#if agent.tasks.length && agent.status !== 'done'}
    {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
    <span class="progress">
      <span class="bar" aria-hidden="true"><span style:width="{(done / agent.tasks.length) * 100}%"></span></span>
      <span class="count">{done}/{agent.tasks.length} tasks</span>
    </span>
  {/if}
{/snippet}

{#if layout.collapsed}
  <aside class="folded" aria-label="Project agents, collapsed" style:--project={project.color}>
    <span class="square" title={project.name}></span>
    <button class="add" aria-label="New agent" title="New agent (Ctrl N)" disabled={!project.exists} onclick={onnew}>+</button>
    <span class="sep" aria-hidden="true"></span>
    <div class="tiles">
      {#each list as agent (agent.id)}
        {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
        <button
          class="tile {look(agent)}"
          aria-current={agent.id === current?.id ? 'true' : undefined}
          aria-label="{agent.name}, {label(agent)}"
          onclick={() => (agents.current[project.path] = agent.id)}
          onmouseenter={(e) => hover(agent, e)}
          onmouseleave={() => (hovered = null)}
          onfocus={(e) => hover(agent, e)}
          onblur={() => (hovered = null)}
        >
          {short(agent.name)}
          <span class="marker"><StatusIcon status={look(agent)} /></span>
          {#if agent.tasks.length}
            <span class="fill" style:width="{(done / agent.tasks.length) * 100}%"></span>
          {/if}
        </button>
      {/each}
    </div>
    <button class="toggle" aria-label="Expand the agent list" title="Expand the list (Ctrl B)" onclick={toggleSidebar}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
    </button>
    {#if hovered}
      <div class="tip agent {look(hovered.agent)}" role="tooltip" style:top="{hovered.top}px" style:left="{hovered.left}px">
        {@render card(hovered.agent)}
      </div>
    {/if}
  </aside>
{:else}
<aside aria-label="Project agents" style:width="{layout.sidebar}px" style:--project={project.color}>
  <div class="head">
    <div class="who">
      <span class="square" aria-hidden="true"></span>
      <div>
        <strong>{project.name}</strong>
        <small class:bad={!project.exists}>
          {#if !project.exists}
            Folder not found
          {:else if list.length}
            {list.length} agent{list.length > 1 ? 's' : ''}{#if pending}&nbsp;· {pending} waiting for you{/if}
          {:else}
            No agents
          {/if}
        </small>
      </div>
    </div>
    <div class="actions">
      <button class="new" disabled={!project.exists} onclick={onnew} title="New agent (Ctrl N)">+ Agent</button>
      <button class="fold" aria-label="Collapse the agent list" title="Collapse the list (Ctrl B)" onclick={toggleSidebar}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
    </div>
  </div>

  {#if list.length}
    <div class="list">
      {#each list as agent (agent.id)}
        {#if editing === agent.id}
          <div class="agent {look(agent)}" aria-current={agent.id === current?.id ? 'true' : undefined}>
            {@render card(agent)}
          </div>
        {:else}
          <button
            class="agent {look(agent)}"
            aria-current={agent.id === current?.id ? 'true' : undefined}
            title="Double-click or F2 to rename"
            onclick={() => (agents.current[project.path] = agent.id)}
            ondblclick={() => startRename(agent)}
            onkeydown={(e) => e.key === 'F2' && startRename(agent)}
          >
            {@render card(agent)}
          </button>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="empty">
      <svg width="28" height="28" viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="4" y="4" width="16" height="6" rx="2" /><rect x="4" y="14" width="16" height="6" rx="2" stroke-dasharray="3 3" /></svg
      >
      {project.exists
        ? `${project.name} agents will show up here, with their state and cost.`
        : 'You can create agents once the folder is found again.'}
    </div>
  {/if}

  <div class="info">
    <span class="mono">{home(project.path)}</span>
    <span>{project.git ? `Git · ${project.branch ?? 'repository'}` : 'No Git repository'}</span>
    <span>Project cost · {cost(total)}</span>
    <button class="retire" onclick={onretire}>Remove project</button>
  </div>
</aside>
{/if}

<style>
  aside {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 4px 10px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .who {
    min-width: 0;
  }

  .who div {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .who strong,
  .who small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .who small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .who small.bad {
    color: var(--err-text);
  }

  .square {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--project);
  }

  .new {
    flex: none;
    white-space: nowrap;
    height: 32px;
    padding: 0 12px;
    border: 1px dashed var(--border-dashed);
    border-radius: 4px;
    color: var(--text-2);
    font-weight: 500;
  }

  .new:not(:disabled):hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .new:disabled {
    opacity: 0.4;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0 -4px;
    padding: 2px 4px;
  }

  .agent {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 6px;
    text-align: left;
  }

  .agent:hover {
    background: var(--surface-1);
  }

  .agent[aria-current='true'] {
    background: var(--surface-1);
    box-shadow: 0 0 0 1px var(--border-strong);
  }

  .agent.waiting {
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
  }

  .agent.waiting[aria-current='true'] {
    box-shadow:
      0 0 0 2px var(--accent-soft),
      0 8px 24px rgba(201, 105, 58, 0.18);
  }

  .agent.error[aria-current='true'] {
    background: var(--err-bg);
    box-shadow:
      inset 0 0 0 1px var(--err-border),
      0 0 0 1px var(--err);
  }

  .line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .actions {
    display: flex;
    gap: 4px;
  }

  .fold,
  .toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-2);
  }

  .fold {
    width: 28px;
    height: 32px;
  }

  .fold:hover,
  .toggle:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .fold svg,
  .toggle svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .folded {
    width: 68px;
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 0;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
  }

  .folded .square {
    margin: 4px 0 6px;
  }

  .add {
    width: 44px;
    height: 44px;
    flex: none;
    border: 1px dashed var(--border-dashed);
    border-radius: 4px;
    color: var(--text-2);
    font-size: 18px;
  }

  .add:not(:disabled):hover {
    background: var(--surface-5);
  }

  .sep {
    width: 28px;
    height: 1px;
    margin: 2px 0;
    background: var(--border);
  }

  .tiles {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    width: 100%;
  }

  .tile {
    position: relative;
    width: 44px;
    height: 44px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 500 12px var(--mono);
  }

  .tile.done,
  .tile.interrupted {
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-dim);
  }

  .tile.waiting {
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
    color: var(--wait-text);
  }

  .tile.error {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
  }

  .tile[aria-current='true'] {
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 0 0 3px var(--bg-app),
      0 0 0 4px var(--text);
  }

  .tile.waiting[aria-current='true'] {
    box-shadow:
      0 0 0 1px var(--wait-border),
      0 0 0 3px var(--bg-app),
      0 0 0 4px var(--text),
      0 0 16px rgba(224, 138, 91, 0.35);
  }

  .marker {
    position: absolute;
    top: 4px;
    right: 4px;
    display: flex;
  }

  .fill {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    background: var(--ok);
  }

  .waiting .fill {
    background: var(--accent);
  }

  .toggle {
    width: 44px;
    height: 36px;
    flex: none;
    background: var(--surface-2);
  }

  .tip {
    position: fixed;
    z-index: 20;
    width: 280px;
    pointer-events: none;
    background: var(--menu);
    box-shadow:
      0 0 0 1px #4e443e,
      0 16px 40px rgba(0, 0, 0, 0.55);
  }

  .tip.waiting {
    background: var(--wait-bg);
    box-shadow:
      0 0 0 1px var(--wait-border),
      0 16px 40px rgba(0, 0, 0, 0.55);
  }

  .rename {
    flex: 1;
    min-width: 0;
    height: 24px;
    margin: -3px 0;
    padding: 0 6px;
    border: none;
    border-radius: 3px;
    outline: none;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 2px var(--border-focus);
    font-weight: 600;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .done .name {
    color: var(--text-3);
  }

  .waiting .name {
    color: var(--wait-text);
  }

  .cost {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .waiting .cost {
    color: #ddbba8;
  }

  .state,
  .detail {
    padding-left: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .done .state,
  .interrupted .state {
    font: 12px var(--sans);
  }

  .waiting .state {
    font: 600 12.5px var(--sans);
    color: var(--accent-soft);
  }

  .error .state {
    font: 600 12.5px var(--sans);
    color: var(--err-text);
  }

  .waiting .detail {
    color: #ddbba8;
  }

  .error .detail {
    color: var(--err-muted);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0 0 18px;
  }

  .bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--ok);
  }

  .waiting .bar {
    background: #5a3322;
  }

  .waiting .bar span {
    background: var(--accent);
  }

  .count {
    font: 11px var(--mono);
    color: var(--text-dim);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 24px;
    text-align: center;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .empty svg {
    fill: none;
    stroke: var(--text-faint);
    stroke-width: 1.6;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border-radius: 4px;
    background: var(--surface-1);
    font-size: 12px;
    color: var(--text-dim);
  }

  .info .mono {
    color: var(--text-3);
    overflow-wrap: anywhere;
  }

  .retire {
    align-self: flex-start;
    margin-top: 4px;
    color: var(--text-dim);
    text-decoration: underline;
  }

  .retire:hover {
    color: var(--err-text);
  }
</style>
