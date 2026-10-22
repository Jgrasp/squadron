<script lang="ts">
  import { agents, clock, ofProject, show, todayCost, waitingAgents, waitingDetail } from './agents.svelte'
  import { ago, cost, modelLabel } from './format'
  import { ui } from './actions.svelte'
  import { home, projects } from './projects.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import type { Agent } from './stream'

  let finished = $state(true)

  const order = { waiting: 0, error: 1, working: 2, interrupted: 3, done: 4 }
  const rank = (a: Agent) => (a.attention && a.status !== 'working' ? -1 : order[a.status])
  const look = (a: Agent) => (a.attention && a.status !== 'working' && a.status !== 'error' ? 'waiting' : a.status)
  const groups = $derived(
    projects.list
      .map((project) => ({
        project,
        list: ofProject(project.path)
          .filter((a) => finished || a.status !== 'done')
          .sort((a, b) => rank(a) - rank(b) || b.since - a.since),
        all: ofProject(project.path),
      }))
      .sort((a, b) => Number(b.all.some((x) => x.attention)) - Number(a.all.some((x) => x.attention))),
  )
  const pending = $derived(waitingAgents().length)

  function label(agent: Agent) {
    if (look(agent) === 'waiting' && agent.status !== 'waiting') return 'Waiting for you'
    switch (agent.status) {
      case 'waiting':
        return agent.asks[0]?.tool_name === 'AskUserQuestion' ? 'Waiting for your answer' : 'Waiting for your approval'
      case 'working':
        return 'Working'
      case 'done':
        return `Done · ${ago(agent.since, clock.now)}`
      case 'interrupted':
        return 'Interrupted'
      case 'error':
        return 'Error'
    }
  }

  function last(agent: Agent) {
    if (agent.status === 'waiting') return waitingDetail(agent)
    if (agent.attention) return agent.attention
    if (agent.status === 'done' && agent.summary) return agent.summary
    return agent.action || agent.summary
  }

  function open(project: string) {
    projects.overview = false
    projects.current = project
  }
</script>

<main>
  <div class="top">
    <div>
      <h1>Overview</h1>
      <span>
        Agents across all your projects{#if pending}&nbsp;· <strong>{pending} waiting for you</strong>{/if}
        · Today <span class="mono">{cost(todayCost())}</span>
      </span>
    </div>
    <label><input type="checkbox" bind:checked={finished} /> Show finished agents</label>
  </div>

  <div class="scroll">
    <table>
      <thead>
        <tr>
          <th scope="col">State</th>
          <th scope="col">Agent</th>
          <th scope="col">Last action</th>
          <th scope="col">Progress</th>
          <th scope="col">Cost</th>
          <th scope="col"><span class="sr-only">Actions</span></th>
        </tr>
      </thead>
      {#each groups as group (group.project.path)}
        {@const waiting = group.all.filter((a) => a.attention).length}
        <tbody>
          <tr class="project">
            <th scope="colgroup" colspan="6">
              <span class="name"><span class="square" aria-hidden="true" style:background={group.project.color}></span>{group.project.name}</span>
              <span class="path">{home(group.project.path)}</span>
              <span class="meta">
                {group.all.length
                  ? `${group.all.length} agent${group.all.length > 1 ? 's' : ''}${waiting ? ` · ${waiting} waiting for you` : ''} · ${cost(group.all.reduce((s, a) => s + a.cost, 0))}`
                  : 'No agents'}
              </span>
              <button class="link" onclick={() => open(group.project.path)}>Open project</button>
            </th>
          </tr>
          {#if !group.list.length}
            <tr class="none">
              <td colspan="6">
                {group.all.length ? 'No agent running' : 'No agents yet'}
                <button class="action" disabled={!group.project.exists} onclick={() => (open(group.project.path), (ui.creating = true))}>New agent</button>
              </td>
            </tr>
          {/if}
          {#each group.list as agent (agent.id)}
            {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
            <tr class="agent {look(agent)}">
              <td><span class="state"><StatusIcon status={look(agent)} />{label(agent)}</span></td>
              <td>
                <span class="who">{agent.name}</span>
                <span class="opts">{modelLabel(agent.options.model)} · {agent.options.effort}</span>
              </td>
              <td class="last">{last(agent)}</td>
              <td>
                {#if agent.tasks.length}
                  <span class="progress">
                    <span class="bar" aria-hidden="true"><span style:width="{(done / agent.tasks.length) * 100}%"></span></span>
                    <span class="mono">{done}/{agent.tasks.length}</span>
                  </span>
                {/if}
              </td>
              <td class="mono">{cost(agent.cost)}</td>
              <td>
                <button class="action" class:answer={!!agent.attention} onclick={() => show(agent)}>
                  {agent.attention ? 'Answer' : 'Open'}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      {/each}
    </table>
    {#if !agents.list.length}
      <p class="empty">No agents yet. Open a project and start one.</p>
    {/if}
  </div>
</main>

<style>
  main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-main);
  }

  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 24px 32px 16px;
  }

  h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 700;
  }

  .top span {
    color: var(--text-dim);
  }

  .top strong {
    color: var(--accent-soft);
  }

  .top label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .top input {
    accent-color: var(--accent);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 32px 32px;
  }

  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
  }

  thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 10px 12px;
    background: var(--bg-main);
    border-bottom: 1px solid var(--border);
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .project th {
    padding: 22px 12px 8px;
    text-align: left;
    font-weight: 400;
    border-bottom: 1px solid var(--border);
  }

  .project th > * {
    margin-right: 12px;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
  }

  .square {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--accent);
  }

  .path {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .meta {
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .link {
    color: var(--accent-link);
    font-size: 12.5px;
  }

  .link:hover {
    color: var(--accent-soft);
  }

  td {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  .agent.waiting td {
    background: rgba(59, 34, 22, 0.55);
  }

  .state {
    display: flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .waiting .state {
    color: var(--accent-soft);
    font-weight: 600;
  }

  .error .state {
    color: var(--err-text);
    font-weight: 600;
  }

  .who {
    display: block;
    font-weight: 600;
  }

  .opts {
    font-size: 12px;
    color: var(--text-dim);
  }

  .last {
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 12px var(--mono);
    color: var(--text-3);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 110px;
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
    background: var(--ok);
  }

  .mono {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .action {
    height: 30px;
    padding: 0 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
  }

  .action:hover {
    background: var(--surface-4);
  }

  .none td {
    padding: 10px 14px;
    color: var(--text-dim);
  }

  .none .action {
    margin-left: 12px;
  }

  .action:disabled {
    opacity: 0.4;
  }

  .action.answer {
    background: var(--accent);
    box-shadow: none;
    color: var(--accent-ink);
    font-weight: 700;
  }

  .empty {
    color: var(--text-dim);
  }
</style>
