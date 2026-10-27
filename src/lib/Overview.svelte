<script lang="ts">
  import { agents, calling, clock, ofProject, show, waitingAgents, waitingDetail } from './agents.svelte'
  import { ago, effortLabel, modelLabel } from './format'
  import { ui } from './actions.svelte'
  import { home, projects } from './projects.svelte'
  import PullTag from './PullTag.svelte'
  import { lists, needsYou } from './pulls.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { activity, type Agent } from './stream'
  import { t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'

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
    if (look(agent) === 'waiting' && agent.status !== 'waiting') return t('shell.waitingForYou')
    if (!agent.entries.length) return t('shell.readyForFirst')
    switch (agent.status) {
      case 'waiting':
        return agent.asks[0]?.tool_name === 'AskUserQuestion' ? t('shell.waitingForAnswer') : t('shell.waitingForApproval')
      case 'working':
        return t('shell.working')
      case 'done':
        return t('shell.doneDot', { ago: ago(agent.since, clock.now) })
      case 'interrupted':
        return t('shell.interrupted')
      case 'error':
        return t('shell.error')
    }
  }

  function last(agent: Agent) {
    if (agent.status === 'waiting') return waitingDetail(agent)
    if (agent.attention) return agent.attention
    if (agent.status === 'done' && agent.summary) return agent.summary
    return (agent.status === 'working' ? activity(agent) : agent.action) || agent.summary
  }

  function open(project: string) {
    projects.overview = projects.pulls = projects.board = false
    projects.current = project
  }

  function openPulls(project: string) {
    open(project)
    projects.board = false
    projects.pulls = true
  }

  const open_pulls = (project: string) => (lists[project] ? lists[project].review.length + lists[project].mine.length : 0)
  const pullsWaiting = $derived(projects.list.reduce((n, p) => n + needsYou(p.path), 0))
</script>

<main>
  <div class="top">
    <div>
      <h1>{t('shell.overview')}</h1>
      <span>
        {t('shell.acrossProjects')}{#if pending}&nbsp;· <strong>{t('shell.waitingCount', { count: pending })}</strong>{/if}{#if pullsWaiting}&nbsp;·
          <strong>{t('shell.pullsForYou', { count: pullsWaiting })}</strong>{/if}
      </span>
    </div>
    <label><input type="checkbox" bind:checked={finished} /> {t('shell.showFinished')}</label>
  </div>

  <div class="scroll">
    <table>
      <thead>
        <tr>
          <th scope="col">{t('shell.columnState')}</th>
          <th scope="col">{t('shell.columnAgent')}</th>
          <th scope="col">{t('shell.columnLastAction')}</th>
          <th scope="col">{t('shell.columnProgress')}</th>
          <th scope="col"><span class="sr-only">{t('shell.columnActions')}</span></th>
        </tr>
      </thead>
      {#each groups as group (group.project.path)}
        {@const waiting = group.all.filter(calling).length}
        <tbody>
          <tr class="project">
            <th scope="colgroup" colspan="5">
              <span class="name"><span class="square" aria-hidden="true" style:background={group.project.color}></span>{group.project.name}</span>
              <span class="path">{home(group.project.path)}</span>
              <span class="meta">
                {group.all.length
                  ? [t('shell.agentCount', { count: group.all.length }), ...(waiting ? [t('shell.waitingCount', { count: waiting })] : [])].join(' · ')
                  : t('shell.noAgents')}
              </span>
              {#if open_pulls(group.project.path)}
                {@const waiting = needsYou(group.project.path)}
                <button
                  class="pulls"
                  class:urgent={waiting > 0}
                  title={t('shell.pullsOfProject', { name: group.project.name })}
                  onclick={() => openPulls(group.project.path)}
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"
                    ><circle cx="6" cy="6" r="2" /><circle cx="6" cy="18" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8M18 16V9a3 3 0 0 0-3-3h-4M13 3l-3 3 3 3" /></svg
                  >
                  {waiting ? t('shell.pullsForYou', { count: waiting }) : t('shell.pullCount', { count: open_pulls(group.project.path) })}
                </button>
              {/if}
              <button class="link" onclick={() => open(group.project.path)}>{t('shell.openProject')}</button>
            </th>
          </tr>
          {#if !group.list.length}
            <tr class="none">
              <td colspan="5">
                {group.all.length ? t('shell.noAgentRunning') : t('shell.noAgentsYet')}
                <button class="action" disabled={!group.project.exists} onclick={() => (open(group.project.path), (ui.creating = true))}>{t('common.newAgent')}</button>
              </td>
            </tr>
          {/if}
          {#each group.list as agent (agent.id)}
            {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
            <tr class="agent {look(agent)}">
              <td><span class="state"><StatusIcon status={look(agent)} />{label(agent)}</span></td>
              <td>
                <span class="who">{agent.name}</span>
                <span class="opts">{modelLabel(agent.options.model)} · {effortLabel(agent.options.effort)}</span>
                <PullTag {agent} />
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
              <td>
                <button class="action" class:answer={!!agent.attention} onclick={() => show(agent)}>
                  {agent.attention ? t('shell.answer') : t('common.open')}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      {/each}
    </table>
    {#if !agents.list.length}
      <p class="empty">{t('shell.overviewEmpty')}</p>
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

  .pulls {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 9px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12px;
    vertical-align: middle;
  }

  .pulls svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .pulls:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .pulls.urgent {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 600;
  }

  .pulls.urgent:hover {
    background: var(--accent-soft);
    color: var(--accent-ink);
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
