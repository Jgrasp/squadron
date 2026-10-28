<script lang="ts">
  import { ask } from './asking.svelte'
  import { agents, archive, clock, fork, markUnread, ofProject, pin, remove, rename, waiting, waitingDetail, waitingFor } from './agents.svelte'
  import { ago, short } from './format'
  import { layout, orderAgents, resetSort, setSort, toggleSidebar, type AgentSort } from './layout.svelte'
  import { sortable } from './layout-slots.svelte'
  import { projects, type Project } from './projects.svelte'
  import { hint } from './settings.svelte'
  import ArchivePanel from './ArchivePanel.svelte'
  import ImportPanel from './ImportPanel.svelte'
  import PullTag from './PullTag.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { activity, type Agent } from './stream'
  import { asking, count, tally } from './subagents'
  import { list as join, t, type Key } from './i18n/index.svelte'

  let {
    project,
    current,
    onnew,
  }: { project: Project; current?: Agent; onnew: () => void } = $props()

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

  let menu = $state<{ agent: Agent; x: number; y: number } | null>(null)

  function open(agent: Agent, event: MouseEvent) {
    event.preventDefault()
    menu = { agent, x: event.clientX, y: event.clientY }
  }

  function act(action: (id: string) => unknown) {
    const id = menu?.agent.id
    menu = null
    if (id) action(id)
  }

  async function erase(agent: Agent) {
    menu = null
    const sure = await ask(t('shell.deleteAgentAsk', { name: agent.name }), {
      title: t('shell.deleteAgentTitle'),
      kind: 'warning',
      okLabel: t('common.delete'),
      cancelLabel: t('common.cancel'),
    })
    if (sure) remove(agent.id)
  }

  let hovered = $state<{ agent: Agent; top: number; left: number } | null>(null)

  function hover(agent: Agent, event: Event) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    hovered = { agent, top: rect.top - 6, left: rect.right + 12 }
  }

  const SORTS: [AgentSort['by'], Key][] = [
    ['none', 'shell.sortByHand'],
    ['updated', 'shell.sortLastActivity'],
    ['created', 'shell.sortCreated'],
    ['name', 'common.name'],
  ]

  let sorting = $state(false)
  let scope = $state<'all' | 'project'>('all')
  let showArchived = $state(false)
  let showImport = $state(false)

  const own = $derived(layout.sorts[project.path])
  const sort = $derived(own ?? layout.sort)
  const shownSort = $derived(scope === 'project' ? sort : layout.sort)

  function openSort() {
    scope = own ? 'project' : 'all'
    sorting = !sorting
  }

  const created = (a: Agent) => a.entries.find((e) => 'at' in e)?.at ?? 0
  const key: Record<Exclude<AgentSort['by'], 'none'>, (a: Agent, b: Agent) => number> = {
    updated: (a, b) => a.since - b.since,
    created: (a, b) => created(a) - created(b),
    name: (a, b) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' }),
  }
  const list = $derived.by(() => {
    const all = [...ofProject(project.path)]
    const by = sort.by
    if (by !== 'none') all.sort((a, b) => key[by](a, b) * (sort.desc ? -1 : 1))
    else {
      const order = layout.orders[project.path] ?? []
      const rank = (a: Agent) => (order.includes(a.id) ? order.indexOf(a.id) : order.length)
      all.sort((a, b) => rank(a) - rank(b))
    }
    return [...all.filter((a) => a.pinned), ...all.filter((a) => !a.pinned)]
  })
  const pinned = $derived(list.filter((a) => a.pinned).length)

  let chosen = $state<string[]>([])
  let anchor = ''
  const picked = $derived(list.filter((a) => chosen.includes(a.id)))

  $effect(() => {
    void project.path
    chosen = []
  })

  function sweep(event: PointerEvent, agent: Agent) {
    if (event.button !== 0 || !(event.ctrlKey || event.metaKey || event.shiftKey)) return
    event.preventDefault()
    const ids = list.map((a) => a.id)
    const base = [...chosen]
    const extend = event.shiftKey && ids.includes(anchor)
    const from = ids.indexOf(extend ? anchor : agent.id)
    const adding = extend || !chosen.includes(agent.id)
    if (!extend) anchor = agent.id
    const apply = (to: number) => {
      const range = ids.slice(Math.min(from, to), Math.max(from, to) + 1)
      chosen = adding ? [...new Set([...base, ...range])] : base.filter((id) => !range.includes(id))
    }
    apply(ids.indexOf(agent.id))
    const move = (e: PointerEvent) => {
      const item = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('.list [data-item]')
      if (item && ids.includes(item.dataset.item!)) apply(ids.indexOf(item.dataset.item!))
    }
    const up = () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  function openAgent(event: MouseEvent, agent: Agent) {
    if (event.ctrlKey || event.metaKey || event.shiftKey) return
    chosen = []
    agents.current[project.path] = agent.id
  }

  async function bulk(action: (id: string) => unknown) {
    const done = Promise.all(picked.map((a) => action(a.id)))
    chosen = []
    await done
  }

  async function eraseAll() {
    const names = picked.map((a) => a.name)
    const sure = await ask(t('shell.deleteAgentsAsk', { count: names.length, names: join(names) }), {
      title: t('shell.deleteAgentsTitle', { count: names.length }),
      kind: 'warning',
      okLabel: t('common.delete'),
      cancelLabel: t('common.cancel'),
    })
    if (sure) bulk(remove)
  }


  function drop(id: string, index: number) {
    const ids = list.map((a) => a.id).filter((a) => a !== id)
    ids.splice(index, 0, id)
    orderAgents(project.path, ids)
    if (sort.by !== 'none') setSort(project.path, { by: 'none' })
  }
  const pending = $derived(waiting(project.path))

  const blocked = (agent: Agent) => !!agent.attention && agent.status !== 'working' && agent.status !== 'error' && !agent.asks.length
  const look = (agent: Agent) => (blocked(agent) ? 'waiting' : agent.status)

  function label(agent: Agent) {
    if (blocked(agent)) return t('shell.waitingForYou')
    if (!agent.entries.length) return t('shell.readyForFirst')
    switch (agent.status) {
      case 'working':
        return activity(agent)
      case 'done':
        return t('shell.doneAgo', { ago: ago(agent.since, clock.now) })
      case 'interrupted':
        return t('shell.interruptedWaiting')
      case 'waiting':
        return agent.asks[0] ? waitingFor(agent.asks[0]) : agent.action
      case 'error':
        return t('shell.errorStopped')
    }
  }
</script>

{#snippet card(agent: Agent)}
  {@const ticket = projects.list.find((p) => p.path === agent.project)?.cards.find((c) => c.agent === agent.id)?.ticket}
  <span class="line">
    <StatusIcon status={look(agent)} />
    {#if editing === agent.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="rename"
        aria-label={t('shell.agentName')}
        bind:value={newName}
        autofocus
        onkeydown={(e) => (e.key === 'Enter' ? finishRename(true) : e.key === 'Escape' && finishRename(false))}
        onblur={() => finishRename(true)}
      />
    {:else}
      {#if agent.unread}<span class="unread" role="img" aria-label={t('shell.unread')} title={t('shell.unread')}></span>{/if}
      <span class="name">{agent.name}</span>
      {#if agent.pinned}<svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label={t('shell.pinned')}><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>{/if}
    {/if}
  </span>
  <span class="state">{#if ticket}<span class="ticket" title={t('tickets.linkTitle', { key: ticket.key })}><span class="logo" aria-hidden="true">J</span>{ticket.key}</span>{/if}{label(agent)}</span>
  <PullTag {agent} gutter />
  {#if agent.status === 'error' && agent.action}
    <span class="detail">{agent.action}</span>
  {:else if agent.status === 'waiting'}
    <span class="detail">{waitingDetail(agent)}</span>
  {:else if blocked(agent)}
    <span class="detail">{agent.attention}</span>
  {/if}
  {#if agent.progress}
    {@const { done, total, message, place } = agent.progress}
    <span class="reported {place}">
      <span class="bar" role="progressbar" aria-valuemin={0} aria-valuemax={total} aria-valuenow={done} aria-label={message || t('shell.columnProgress')}
        ><span style:width="{(done / total) * 100}%"></span></span
      >
      <span class="message">{message || `${done}/${total}`}</span>
    </span>
  {/if}
  {#if agent.tasks.length && agent.status !== 'done'}
    {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
    <span class="progress">
      <span class="bar" aria-hidden="true"><span style:width="{(done / agent.tasks.length) * 100}%"></span></span>
      <span class="count">{t('shell.tasksDone', { done, total: agent.tasks.length })}</span>
    </span>
  {/if}
  {#if agent.subagents.length}
    {@const waiting = asking(agent)}
    {@const counts = count(agent.subagents, waiting)}
    {#if counts.running || counts.waiting}
      <span class="subagents">
        <span>{agent.subagents.length} subagent{agent.subagents.length > 1 ? 's' : ''}</span>
        {#each tally(agent.subagents, waiting) as part (part.look)}<span class="part"><span class="dot {part.look}" aria-hidden="true"></span>{part.text}</span>{/each}
      </span>
    {/if}
  {/if}
{/snippet}

<svelte:window
  onclick={(e) => !(e.target as HTMLElement).closest('[data-menu]') && ((menu = null), (sorting = false))}
  onkeydown={(e) => e.key === 'Escape' && ((menu = null), (sorting = false))}
/>

{#if menu}
  {@const agent = menu.agent}
  <div class="menu" role="menu" aria-label={t('shell.actionsOf', { name: agent.name })} data-menu style:left="{menu.x}px" style:top="{menu.y}px">
    <button role="menuitem" onclick={() => ((pinned) => act((id) => pin(id, pinned)))(!agent.pinned)}>{agent.pinned ? t('shell.unpin') : t('shell.pinToTop')}</button>
    <button role="menuitem" onclick={() => ((value) => act((id) => markUnread(id, value)))(!agent.unread)}>{agent.unread ? t('shell.markRead') : t('shell.markUnread')}</button>
    <button role="menuitem" disabled={!agent.sessionId} onclick={() => act(fork)}>{t('shell.fork')}</button>
    <button role="menuitem" onclick={() => act(archive)}>{t('shell.archiveConversation')}</button>
    <button role="menuitem" class="danger" onclick={() => erase(agent)}>{t('common.deleteEllipsis')}</button>
  </div>
{/if}

{#if showImport}
  <ImportPanel {project} onclose={() => (showImport = false)} />
{/if}
{#if showArchived}
  <ArchivePanel {project} onclose={() => (showArchived = false)} />
{/if}

{#if layout.collapsed}
  <aside class="folded" aria-label={t('shell.agentsCollapsed')} style:--project={project.color}>
    <span class="square" title={project.name}></span>
    <button class="add" aria-label={t('common.newAgent')} title={t('shell.newAgentKey', { key: hint('newAgent') })} disabled={!project.exists} onclick={onnew}>+</button>
    <span class="sep" aria-hidden="true"></span>
    <div class="tiles" use:sortable={{ enabled: list.length > 1, onplace: drop }}>
      {#each list as agent, i (agent.id)}
        {@const done = agent.tasks.filter((t) => t.status === 'completed').length}
        {#if pinned && i === pinned}<span class="sep pins" aria-hidden="true"></span>{/if}
        <button
          class="tile {look(agent)}"
          data-item={agent.id}
          aria-current={agent.id === current?.id ? 'true' : undefined}
          aria-label={t('shell.tileLabel', { name: agent.name, state: label(agent) ?? '' })}
          onclick={() => (agents.current[project.path] = agent.id)}
          oncontextmenu={(e) => open(agent, e)}
          onmouseenter={(e) => hover(agent, e)}
          onmouseleave={() => (hovered = null)}
          onfocus={(e) => hover(agent, e)}
          onblur={() => (hovered = null)}
        >
          {short(agent.name)}
          <span class="marker"><StatusIcon status={look(agent)} /></span>
          {#if agent.progress}
            <span class="fill" style:width="{(agent.progress.done / agent.progress.total) * 100}%"></span>
          {:else if agent.tasks.length}
            <span class="fill" style:width="{(done / agent.tasks.length) * 100}%"></span>
          {/if}
        </button>
      {/each}
    </div>
    <button class="toggle" aria-label={t('shell.expandAgentList')} title={t('shell.expandList', { key: hint('sidebar') })} onclick={toggleSidebar}>
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
    </button>
    {#if hovered}
      <div class="tip agent {look(hovered.agent)}" role="tooltip" style:top="{hovered.top}px" style:left="{hovered.left}px">
        {@render card(hovered.agent)}
      </div>
    {/if}
  </aside>
{:else}
<aside aria-label={t('shell.projectAgents')} style:width="{layout.sidebar}px" style:--project={project.color}>
  <div class="head">
    <div class="who">
      <span class="square" aria-hidden="true"></span>
      <strong title={project.name}>{project.name}</strong>
      <button class="fold collapse" aria-label={t('shell.collapseAgentList')} title={t('shell.collapseList', { key: hint('sidebar') })} onclick={toggleSidebar}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
      </button>
    </div>
    <div class="under">
    <small class:bad={!project.exists}>
      {#if !project.exists}
        {t('shell.folderNotFound')}
      {:else if list.length}
        {t('shell.agentCount', { count: list.length })}{#if pending}&nbsp;· {t('shell.waitingCount', { count: pending })}{/if}
      {:else}
        {t('shell.noAgents')}
      {/if}
    </small>
    <div class="actions">
      <button class="new" disabled={!project.exists} onclick={onnew} title={t('shell.newAgentKey', { key: hint('newAgent') })}>{t('shell.addAgent')}</button>
      <span class="anchor loose" data-menu>
        <button class="fold" class:own aria-label={t('shell.sortAgents')} aria-haspopup="menu" aria-expanded={sorting} title={own ? t('shell.sortedFor', { name: project.name }) : t('shell.sortAgents')} onclick={openSort}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4v16M3 16l4 4 4-4M17 20V4M13 8l4-4 4 4" /></svg>
        </button>
        {#if sorting}
          <div class="menu sort" role="menu" aria-label={t('shell.sortAgents')}>
            <div class="scope" role="group" aria-label={t('shell.appliesTo')}>
              <button aria-pressed={scope === 'all'} onclick={() => (scope = 'all')}>{t('shell.allProjects')}</button>
              <button aria-pressed={scope === 'project'} onclick={() => (scope = 'project')}>{project.name}</button>
            </div>
            {#each SORTS as [by, label] (by)}
              <button role="menuitemradio" aria-checked={shownSort.by === by} onclick={() => setSort(scope === 'project' ? project.path : null, { by })}>{t(label)}</button>
            {/each}
            {#if shownSort.by !== 'none'}
              <span class="line-sep" aria-hidden="true"></span>
              <button role="menuitemradio" aria-checked={!shownSort.desc} onclick={() => setSort(scope === 'project' ? project.path : null, { desc: false })}
                >{shownSort.by === 'name' ? t('shell.aToZ') : t('shell.oldestFirst')}</button
              >
              <button role="menuitemradio" aria-checked={shownSort.desc} onclick={() => setSort(scope === 'project' ? project.path : null, { desc: true })}
                >{shownSort.by === 'name' ? t('shell.zToA') : t('shell.newestFirst')}</button
              >
            {/if}
            {#if scope === 'project'}
              <span class="line-sep" aria-hidden="true"></span>
              <button role="menuitem" disabled={!own} onclick={() => ((scope = 'all'), resetSort(project.path))}>
                {own ? t('shell.useSortOfAll') : t('shell.usesSortOfAll')}
              </button>
            {/if}
          </div>
        {/if}
      </span>
      <button class="fold" aria-label={t('shell.archivedAgents')} title={t('shell.archivedAgentsOf', { name: project.name })} onclick={() => (showArchived = true)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 4h18v4H3zM5 8v12h14V8M10 12h4" /></svg>
      </button>
      <button
        class="fold"
        aria-label={t('shell.importConversations')}
        title={t('shell.importConversationsOf', { name: project.name })}
        disabled={!project.exists}
        onclick={() => (showImport = true)}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v12M7 10l5 5 5-5M4 17v3h16v-3" /></svg>
      </button>
    </div>
    </div>
  </div>

  {#if list.length}
    <div class="list" use:sortable={{ enabled: list.length > 1 && !editing, onplace: drop }}>
      {#each list as agent, i (agent.id)}
        {#if pinned && i === pinned}<span class="divider" aria-hidden="true"></span>{/if}
        {#if editing === agent.id}
          <div class="agent {look(agent)}" data-item={agent.id} aria-current={agent.id === current?.id ? 'true' : undefined}>
            {@render card(agent)}
          </div>
        {:else}
          <button
            class="agent {look(agent)}"
            class:picked={chosen.includes(agent.id)}
            class:fresh={agent.status === 'done' && agent.unread}
            data-item={agent.id}
            aria-current={agent.id === current?.id ? 'true' : undefined}
            aria-pressed={chosen.length ? chosen.includes(agent.id) : undefined}
            title={t('shell.renameHint')}
            onpointerdown={(e) => sweep(e, agent)}
            onclick={(e) => openAgent(e, agent)}
            ondblclick={() => startRename(agent)}
            oncontextmenu={(e) => open(agent, e)}
            onkeydown={(e) => e.key === 'F2' && startRename(agent)}
          >
            {@render card(agent)}
          </button>
        {/if}
      {/each}
    </div>
    {#if picked.length}
      {@const pinAll = picked.some((a) => !a.pinned)}
      {@const readAll = picked.some((a) => a.unread)}
      <div class="bulk" role="toolbar" aria-label={t('shell.bulkActions')}>
        <span class="selection">{t('shell.selectedCount', { count: picked.length })}</span>
        <button onclick={() => ((value) => bulk((id) => pin(id, value)))(pinAll)}>{pinAll ? t('shell.pinToTop') : t('shell.unpin')}</button>
        <button onclick={() => ((value) => bulk((id) => markUnread(id, value)))(!readAll)}>{readAll ? t('shell.markRead') : t('shell.markUnread')}</button>
        <button onclick={() => bulk((id) => archive(id))}>{t('common.archive')}</button>
        <button class="danger" onclick={eraseAll}>{t('common.deleteEllipsis')}</button>
        <button class="clear" aria-label={t('shell.clearSelection')} title={t('shell.clearSelection')} onclick={() => (chosen = [])}>×</button>
      </div>
    {/if}
  {:else}
    <div class="empty">
      <svg width="28" height="28" viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="4" y="4" width="16" height="6" rx="2" /><rect x="4" y="14" width="16" height="6" rx="2" stroke-dasharray="3 3" /></svg
      >
      {project.exists ? t('shell.agentsShowHere', { name: project.name }) : t('shell.agentsOnceFound')}
    </div>
  {/if}
</aside>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 20;
    width: 210px;
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

  .menu button:hover:not(:disabled) {
    background: var(--menu-hover);
  }

  .menu button:disabled {
    color: var(--text-faint);
  }

  .anchor {
    position: relative;
  }

  .anchor.loose {
    position: static;
  }

  .menu.sort {
    position: absolute;
    top: calc(100% - 4px);
    left: 4px;
    width: max-content;
    min-width: 230px;
  }

  .menu [aria-checked='true'] {
    color: var(--text-strong);
    font-weight: 600;
  }

  .menu [aria-checked='true']::after {
    content: '✓';
    float: right;
    color: var(--accent);
  }

  .scope {
    display: flex;
    gap: 2px;
    margin-bottom: 4px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .menu .scope button {
    flex: 1;
    height: 28px;
    padding: 0 8px;
    border-radius: 3px;
    color: var(--text-2);
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu .scope button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 600;
  }

  .line-sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }

  .fold.own {
    color: var(--accent);
  }

  .pinned {
    flex: none;
    fill: none;
    stroke: var(--project, currentColor);
    stroke-width: 2.2;
    stroke-linejoin: round;
  }


  .menu .danger {
    color: var(--err-text);
  }

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
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 2px 4px 10px;
  }

  .who {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .collapse {
    margin-left: auto;
  }

  .who strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 15px;
  }

  .under {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .under small {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-dim);
  }

  .under small.bad {
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

  .divider {
    flex: none;
    margin: 4px 14px;
    border-top: 1px dashed var(--border-dashed);
  }

  .agent {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 6px;
    text-align: left;
  }

  .agent:global(.dragging),
  .tile:global(.dragging) {
    z-index: 2;
    cursor: grabbing;
    background: var(--surface-2);
  }

  .agent:hover {
    background: var(--surface-1);
  }

  .agent[aria-current='true'] {
    background: var(--surface-1);
    box-shadow: 0 0 0 1px var(--border-strong);
  }

  .agent.picked {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 2px var(--accent-soft);
  }

  .bulk {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    margin: 8px 0 0;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .bulk .selection {
    flex: 1 0 100%;
    padding: 0 2px 2px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
  }

  .bulk button {
    height: 26px;
    padding: 0 8px;
    border-radius: 4px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .bulk button:hover {
    background: var(--surface-4);
    color: var(--text);
  }

  .bulk .danger {
    color: var(--err-text);
  }

  .bulk .clear {
    margin-left: auto;
  }

  .agent.fresh {
    background: color-mix(in srgb, var(--ok) 9%, transparent);
    box-shadow: inset 3px 0 0 var(--ok);
  }

  .agent.fresh .name {
    color: var(--text-strong);
  }

  .agent.fresh .state {
    font-weight: 600;
    color: var(--ok);
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
    flex: none;
    width: 28px;
    height: 1px;
    margin: 2px 0;
    background: var(--border);
  }

  .sep.pins {
    height: 0;
    background: none;
    border-top: 1px dashed var(--border-dashed);
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

  .unread {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
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

  .state,
  .detail {
    padding-left: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .state {
    font: 12px var(--sans);
  }

  .ticket {
    float: right;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: 8px;
    font: 500 11px var(--mono);
    color: var(--ticket-text);
  }

  .ticket .logo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 11px;
    height: 11px;
    border-radius: 2px;
    background: var(--jira);
    color: #fff;
    font: 700 7.5px var(--sans);
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

  .subagents {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    padding: 2px 0 0 18px;
    font: 11.5px var(--sans);
    color: var(--text-dim);
  }

  .subagents .part {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .subagents .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--sub-stopped);
  }

  .subagents .dot.running {
    background: var(--sub-running);
  }

  .subagents .dot.waiting {
    background: var(--sub-waiting);
  }

  .subagents .dot.error {
    background: var(--sub-error);
  }

  .subagents .dot.done {
    background: var(--sub-done);
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

  .reported {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 2px 0 0 18px;
  }

  .reported.above {
    flex-direction: column-reverse;
  }

  .reported .bar {
    flex: none;
  }

  .message {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-2);
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

</style>
