<script lang="ts">
  import { cubicOut } from 'svelte/easing'
  import { ui } from './actions.svelte'
  import { agents, calling, clock } from './agents.svelte'
  import { ago } from './format'
  import { descendants, EMPTY, groupOf, inTab, matching, parentOf, sortProjects, total, within, type ProjectSort, type Tally } from './groups'
  import { layout, MIN_PROJECTS, resizeProjects, viewProjects, WIDE_PROJECTS } from './layout.svelte'
  import { addGroup, editGroup, editProject, moveProjects, projects, removeGroup, type Project } from './projects.svelte'
  import { hint, settings } from './settings.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'

  type Target = { kind: 'group'; id: string; name: string } | { kind: 'project'; project: Project }
  type Count = 'working' | 'waiting' | 'failed' | 'done'

  const COUNTS: [Count, Key][] = [
    ['working', 'shell.countWorking'],
    ['waiting', 'shell.countWaiting'],
    ['failed', 'shell.countFailed'],
    ['done', 'shell.countDone'],
  ]
  const SORTS: [ProjectSort, Key, Key][] = [
    ['activity', 'shell.projectsSortActivity', 'shell.projectsSortActivityHint'],
    ['attention', 'shell.projectsSortAttention', 'shell.projectsSortAttentionHint'],
    ['name', 'shell.projectsSortName', 'shell.projectsSortNameHint'],
    ['agents', 'shell.projectsSortAgents', 'shell.projectsSortAgentsHint'],
    ['manual', 'shell.projectsSortManual', 'shell.projectsSortManualHint'],
  ]

  let panel: HTMLElement
  let strip = $state<HTMLElement>()
  let query = $state('')
  let cursor = $state(-1)
  let only = $state<Count | ''>('')
  let sorting = $state(false)
  let fresh = $state(true)
  let sizing = $state(false)
  let ink = $state({ x: 0, w: 0 })
  let selected = $state<string[]>([])
  let anchor = ''
  let editing = $state('')
  let draft = $state('')
  let error = $state('')
  let menu = $state<{ x: number; y: number; target: Target | null } | null>(null)
  let drag = $state<{ x: number; y: number; ids: string[]; group: boolean; label: string; color: string } | null>(null)
  let over = $state<string | null>(null)

  const tallies = $derived.by(() => {
    const map = new Map<string, Tally>()
    for (const agent of agents.list) {
      const one = map.get(agent.project) ?? { ...EMPTY }
      one.agents++
      if (agent.status === 'working') one.working++
      if (agent.status === 'done') one.done++
      if (agent.status === 'error' && !agent.errorSeen) one.failed++
      else if (calling(agent)) one.waiting++
      one.last = Math.max(one.last, agent.since)
      map.set(agent.project, one)
    }
    return map
  })
  const tally = (path: string) => tallies.get(path) ?? EMPTY

  const view = $derived(layout.projectsView)
  const tops = $derived(projects.groups.filter((g) => !parentOf(projects.groups, g)))
  const loose = $derived(projects.list.some((p) => !groupOf(projects.groups, p)))
  const tab = $derived(view.tab === '-' ? (loose ? '-' : '') : tops.some((g) => g.id === view.tab) ? view.tab : '')
  const subs = $derived(tab && tab !== '-' ? descendants(projects.groups, tab) : [])
  const sub = $derived(subs.some((s) => s.id === view.sub) ? view.sub : '')
  const searching = $derived(!!query.trim())
  const sums = $derived(total(projects.list, tally))
  const shown = $derived.by(() => {
    const scope = searching ? matching(projects.groups, projects.list, query) : projects.list.filter((p) => inTab(projects.groups, p, sub || tab))
    return sortProjects(only ? scope.filter((p) => tally(p.path)[only as Count] > 0) : scope, view.sort, tally, view.desc, view.pinnedFirst)
  })
  const tabs = $derived([
    { id: '', name: t('shell.projectsAll') },
    ...tops.map((g) => ({ id: g.id, name: g.name })),
    ...(loose ? [{ id: '-', name: t('shell.groupRoot') }] : []),
  ])
  const sumOf = (id: string) => total(projects.list.filter((p) => inTab(projects.groups, p, id)), tally)
  const groupName = (project: Project) => projects.groups.find((g) => g.id === groupOf(projects.groups, project))?.name ?? ''

  $effect(() => {
    void query
    cursor = query.trim() ? 0 : -1
  })

  $effect(() => {
    void [tab, tabs, layout.projectsWidth, editing]
    const on = strip?.querySelector<HTMLElement>('.tab.on')
    ink = on ? { x: on.offsetLeft, w: on.offsetWidth } : { x: 0, w: 0 }
  })

  $effect(() => {
    panel.querySelector('input')?.focus()
    const timer = setTimeout(() => (fresh = false), 900)
    return () => clearTimeout(timer)
  })

  function appear(_node: HTMLElement) {
    return {
      duration: 380,
      easing: cubicOut,
      css: (k: number, u: number) => `opacity: ${Math.min(1, k * 1.6)}; transform: translateX(${-24 * u}px) scale(${1 - 0.015 * u}); filter: blur(${6 * u}px)`,
    }
  }

  function vanish(_node: HTMLElement) {
    return { duration: 200, easing: cubicOut, css: (k: number, u: number) => `opacity: ${k}; transform: translateX(${-16 * u}px); filter: blur(${4 * u}px)` }
  }

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = errorText(e)
    }
  }

  function open(path: string) {
    projects.overview = projects.board = projects.pulls = false
    projects.current = path
    ui.projects = false
  }

  function pick(event: MouseEvent, project: Project) {
    if (suppress) return (suppress = false)
    if (event.ctrlKey || event.metaKey) {
      selected = selected.includes(project.path) ? selected.filter((p) => p !== project.path) : [...selected, project.path]
      anchor = project.path
      return
    }
    if (event.shiftKey && anchor) {
      const paths = shown.map((p) => p.path)
      const [from, to] = [paths.indexOf(anchor), paths.indexOf(project.path)]
      if (from >= 0 && to >= 0) selected = paths.slice(Math.min(from, to), Math.max(from, to) + 1)
      return
    }
    selected = []
    open(project.path)
  }

  function choose(id: string, inside = false) {
    if (suppress) return (suppress = false)
    viewProjects(inside ? { sub: id } : { tab: id, sub: '' })
  }

  function field(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (!shown.length) return
      cursor = (cursor + (event.key === 'ArrowDown' ? 1 : -1) + shown.length) % shown.length
      requestAnimationFrame(() => panel.querySelector('.cursor')?.scrollIntoView({ block: 'nearest' }))
    } else if (event.key === 'Enter') {
      const project = shown[Math.max(cursor, 0)]
      if (project) open(project.path)
    }
  }

  async function create(parent: string, paths: string[] = []) {
    menu = null
    await act(async () => {
      const id = await addGroup(t('shell.groupNew'), parent)
      viewProjects(parent ? { tab: topOf(parent), sub: id } : { tab: id, sub: '' })
      if (paths.length) {
        selected = []
        await moveProjects(paths, id)
      }
      rename(id, t('shell.groupNew'))
    })
  }

  function topOf(id: string): string {
    const group = projects.groups.find((g) => g.id === id)
    return group && parentOf(projects.groups, group) ? topOf(group.parent) : id
  }

  function rename(id: string, name: string) {
    menu = null
    draft = name
    editing = id
  }

  function finish(save: boolean) {
    const id = editing
    editing = ''
    const name = draft.trim()
    if (save && id && name && name !== projects.groups.find((g) => g.id === id)?.name) act(() => editGroup(id, { name }))
  }

  const select = (node: HTMLInputElement) => (node.focus(), node.select())

  const targets = (target: Target | null) => (target?.kind === 'project' ? (selected.includes(target.project.path) ? selected : [target.project.path]) : [])

  function relocate(paths: string[], group: string) {
    menu = null
    selected = []
    act(() => moveProjects(paths, group))
  }

  function pin(paths: string[]) {
    menu = null
    const first = projects.list.find((p) => p.path === paths[0])
    if (first) act(() => Promise.all(paths.map((path) => editProject(path, { pinned: !first.pinned }))))
  }

  function context(event: MouseEvent, target: Target | null) {
    event.preventDefault()
    event.stopPropagation()
    sorting = false
    menu = { x: Math.min(event.clientX, window.innerWidth - 240), y: Math.min(event.clientY, window.innerHeight - 320), target }
  }

  const flat = $derived.by(() => {
    const out: { id: string; name: string; depth: number }[] = []
    const walk = (parent: string, depth: number) => {
      for (const group of projects.groups.filter((g) => parentOf(projects.groups, g) === parent)) {
        out.push({ id: group.id, name: group.name, depth })
        walk(group.id, depth + 1)
      }
    }
    walk('', 0)
    return out
  })

  let press: { x: number; y: number; target: Target; started: boolean } | null = null
  let suppress = false

  function down(event: PointerEvent, target: Target) {
    if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || editing) return
    press = { x: event.clientX, y: event.clientY, target, started: false }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up, { once: true })
  }

  function begin(target: Target) {
    if (target.kind === 'group') return (drag = { x: 0, y: 0, ids: [target.id], group: true, label: target.name, color: '' })
    const paths = selected.includes(target.project.path) ? selected : [target.project.path]
    if (!selected.includes(target.project.path)) selected = []
    drag = { x: 0, y: 0, ids: paths, group: false, label: target.project.name, color: target.project.color }
  }

  function aim(x: number, y: number) {
    const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop]')?.dataset.drop
    if (target === undefined || !drag) return null
    if (drag.group && target && within(projects.groups, target, drag.ids[0])) return null
    return target
  }

  function move(event: PointerEvent) {
    if (!press) return
    if (!press.started) {
      if (Math.hypot(event.clientX - press.x, event.clientY - press.y) < 6) return
      press.started = true
      begin(press.target)
    }
    if (!drag) return
    drag.x = event.clientX
    drag.y = event.clientY
    over = aim(event.clientX, event.clientY)
  }

  function up(event: PointerEvent) {
    window.removeEventListener('pointermove', move)
    const started = press?.started
    press = null
    if (!started || !drag) return
    suppress = true
    setTimeout(() => (suppress = false), 0)
    const target = aim(event.clientX, event.clientY)
    const moved = drag
    drag = null
    over = null
    if (target === null) return
    if (moved.group) {
      const group = projects.groups.find((g) => g.id === moved.ids[0])
      if (group && group.parent !== target) act(() => editGroup(group.id, { parent: target }))
    } else if (moved.ids.some((path) => projects.list.find((p) => p.path === path && groupOf(projects.groups, p) !== target))) {
      selected = []
      act(() => moveProjects(moved.ids, target))
    }
  }

  function resize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    handle.setPointerCapture(event.pointerId)
    const follow = (e: PointerEvent) => resizeProjects(e.clientX)
    const stop = () => {
      handle.removeEventListener('pointermove', follow)
      handle.removeEventListener('pointerup', stop)
    }
    handle.addEventListener('pointermove', follow)
    handle.addEventListener('pointerup', stop)
  }

  function widen() {
    sizing = true
    resizeProjects(layout.projectsWidth < WIDE_PROJECTS ? Math.min(1000, window.innerWidth - 80) : 380)
    setTimeout(() => (sizing = false), 360)
  }

  function outside(event: PointerEvent) {
    const target = event.target as HTMLElement
    if (menu && !target.closest('.menu')) menu = null
    if (sorting && !target.closest('.sort')) sorting = false
    if (!panel.contains(target) && !target.closest('[data-projects-toggle], .menu')) ui.projects = false
  }
</script>

<svelte:window onpointerdown={outside} />

{#snippet dots(sum: Tally)}
  {#if sum.working}<i class="mark working"></i>{/if}
  {#if sum.waiting}<i class="mark waiting"></i>{/if}
  {#if sum.failed}<i class="mark failed"></i>{/if}
{/snippet}

{#snippet renaming(id: string, name: string)}
  <input
    class="name"
    bind:value={draft}
    use:select
    aria-label={t('shell.groupNameOf', { name })}
    onblur={() => finish(true)}
    onkeydown={(e) => (e.key === 'Enter' ? finish(true) : e.key === 'Escape' && (e.stopPropagation(), finish(false)))}
  />
{/snippet}

<aside
  class="panel"
  class:fresh
  class:sizing
  class:dragging={!!drag}
  bind:this={panel}
  style:width="{Math.max(MIN_PROJECTS, layout.projectsWidth)}px"
  aria-label={t('shell.projectsPanel')}
  in:appear|global
  out:vanish|global
>
  <div class="top">
    <span class="title">{t('shell.projectsPanel')}</span>
    <div class="tools">
      <button aria-label={t('shell.groupNew')} title={t('shell.groupNew')} onclick={() => create('')}>
        <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10.5v5M9.5 13h5" /></svg>
      </button>
    </div>
  </div>

  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg>
    <input bind:value={query} placeholder={t('shell.searchProject')} aria-label={t('shell.searchProject')} spellcheck="false" onkeydown={field} />
    <kbd>{hint('projects')}</kbd>
  </label>

  <div class="counts" role="group" aria-label={t('shell.projectsCounts')}>
    {#each COUNTS as [id, key] (id)}
      <button class="count {id}" class:zero={!sums[id]} aria-pressed={only === id} title={t('shell.projectsOnly')} onclick={() => (only = only === id ? '' : id)}>
        <b>{sums[id]}</b>
        <small>{t(key)}</small>
      </button>
    {/each}
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="bar">
    <div class="tabs" class:searching bind:this={strip} role="tablist" onwheel={(e) => (e.currentTarget.scrollLeft += e.deltaY)} style:--ink-x="{ink.x}px" style:--ink-w="{ink.w}px">
      {#each tabs as entry (entry.id)}
        {@const sum = sumOf(entry.id)}
        {@const group = entry.id && entry.id !== '-'}
        {#if editing && editing === entry.id}
          <span class="tab on editing">{@render renaming(entry.id, entry.name)}</span>
        {:else}
          <button
            class="tab"
            class:on={tab === entry.id}
            class:target={over === (entry.id === '-' ? '' : entry.id) && !!drag}
            class:dim={drag?.group && drag.ids[0] === entry.id}
            role="tab"
            aria-selected={tab === entry.id}
            data-drop={entry.id === '-' ? '' : entry.id}
            onpointerdown={group ? (e) => down(e, { kind: 'group', id: entry.id, name: entry.name }) : undefined}
            onclick={() => choose(entry.id)}
            oncontextmenu={(e) => context(e, group ? { kind: 'group', id: entry.id, name: entry.name } : null)}
          >
            {entry.name}<em>{projects.list.filter((p) => inTab(projects.groups, p, entry.id)).length}</em>
            {#if entry.id}{@render dots(sum)}{/if}
          </button>
        {/if}
      {/each}
      <span class="ink" aria-hidden="true"></span>
    </div>
    <div class="sort">
      <button class="sorter" class:open={sorting} aria-haspopup="menu" aria-expanded={sorting} onclick={() => ((sorting = !sorting), (menu = null))}>
        <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4v16M3.5 16.5L7 20l3.5-3.5M17 20V4M13.5 7.5L17 4l3.5 3.5" /></svg>
        {t(SORTS.find(([id]) => id === view.sort)![1])}
      </button>
      {#if sorting}
        <div class="menu sorts" role="menu">
          <span class="section">{t('shell.projectsSortBy')}</span>
          {#each SORTS as [id, label, detail] (id)}
            <button role="menuitemradio" aria-checked={view.sort === id} onclick={() => ((sorting = false), viewProjects({ sort: id }))}>
              <svg class="check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4 4 10-10" /></svg>
              {t(label)}<small>{t(detail)}</small>
            </button>
          {/each}
          <span class="rule" aria-hidden="true"></span>
          <button role="menuitemcheckbox" aria-checked={view.desc} disabled={view.sort === 'manual'} onclick={() => viewProjects({ desc: !view.desc })}>
            <svg class="check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4 4 10-10" /></svg>
            {t('shell.projectsSortReverse')}
          </button>
          <button role="menuitemcheckbox" aria-checked={view.pinnedFirst} onclick={() => viewProjects({ pinnedFirst: !view.pinnedFirst })}>
            <svg class="check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4 4 10-10" /></svg>
            {t('shell.projectsPinnedFirst')}
          </button>
        </div>
      {/if}
    </div>
  </div>

  {#if subs.length && !searching}
    <div class="subs">
      <button class="chip" class:on={!sub} class:target={over === tab && !!drag} data-drop={tab} onclick={() => choose('', true)}>
        {t('shell.projectsAllOf', { name: tabs.find((e) => e.id === tab)?.name ?? '' })}<em>{projects.list.filter((p) => inTab(projects.groups, p, tab)).length}</em>
      </button>
      {#each subs as entry (entry.id)}
        {@const name = entry.path[entry.path.length - 1]}
        {#if editing === entry.id}
          <span class="chip on editing">{@render renaming(entry.id, name)}</span>
        {:else}
          <button
            class="chip"
            class:on={sub === entry.id}
            class:target={over === entry.id && !!drag}
            class:dim={drag?.group && drag.ids[0] === entry.id}
            data-drop={entry.id}
            onpointerdown={(e) => down(e, { kind: 'group', id: entry.id, name })}
            onclick={() => choose(entry.id, true)}
            oncontextmenu={(e) => context(e, { kind: 'group', id: entry.id, name })}
          >
            {entry.path.join(' › ')}<em>{projects.list.filter((p) => inTab(projects.groups, p, entry.id)).length}</em>
            {@render dots(sumOf(entry.id))}
          </button>
        {/if}
      {/each}
    </div>
  {/if}

  <div class="list" role="presentation" oncontextmenu={(e) => context(e, null)}>
    <div class="cards">
      {#each shown as project, i (project.path)}
        {@const one = tally(project.path)}
        {@const current = !projects.overview && project.path === projects.current}
        <button
          class="card stripe {one.working ? `effect-${settings.projects_effect}` : ''}"
          class:on={current}
          class:idle={!one.agents}
          class:selected={selected.includes(project.path)}
          class:cursor={i === cursor}
          class:dim={drag && !drag.group && drag.ids.includes(project.path)}
          style:--project={project.color}
          style:--i={Math.min(i, 14)}
          aria-current={current ? 'page' : undefined}
          onpointerdown={(e) => down(e, { kind: 'project', project })}
          onclick={(e) => pick(e, project)}
          oncontextmenu={(e) => context(e, { kind: 'project', project })}
        >
          <span class="head">
            <span class="nm">{project.name}</span>
            {#if project.pinned}
              <svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label={t('shell.pinned')}><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
            {/if}
            {#if one.working}
              <span class="run"><StatusIcon status="working" />{t('shell.workingCount', { count: one.working })}</span>
            {:else if one.last}
              <span class="when">{ago(one.last, clock.now)}</span>
            {/if}
          </span>
          <span class="tags">
            {#if one.agents}
              <span class="tag">
                <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="8" width="14" height="11" rx="3" /><path d="M12 4v4M9.5 13h.01M14.5 13h.01" /></svg>
                {t('shell.agentCount', { count: one.agents })}
              </span>
              {#if one.waiting}<span class="tag waiting">{t('shell.questionCount', { count: one.waiting })}</span>{/if}
              {#if one.failed}<span class="tag failed">{t('shell.errorCount', { count: one.failed })}</span>{/if}
              {#if one.done}
                <span class="tag done">
                  <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 12.5l4 4 8-9" /></svg>
                  {t('shell.doneCount', { count: one.done })}
                </span>
              {/if}
            {:else}
              <span class="empty">{t('shell.noAgents')}</span>
            {/if}
            {#if (searching || !tab) && groupName(project)}<span class="group">{groupName(project)}</span>{/if}
          </span>
        </button>
      {:else}
        <p class="none">{t('shell.projectsNoMatch')}</p>
      {/each}
    </div>
  </div>

  <button class="add" onclick={() => ((ui.projects = false), (ui.adding = ''))}>
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
    {t('shell.addProject')}
  </button>

  <div class="grab" role="separator" aria-orientation="vertical" aria-label={t('shell.projectsResize')} onpointerdown={resize} ondblclick={widen}><i></i></div>
</aside>

{#if drag}
  <div class="ghost" style:left="{drag.x + 14}px" style:top="{drag.y + 10}px" style:--project={drag.color || 'var(--accent)'}>
    <span class="ghost-card back"></span>
    <span class="ghost-card">
      {#if drag.group}
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
      {:else}
        <span class="dot"></span>
      {/if}
      <span class="nm">{drag.label}</span>
      {#if drag.ids.length > 1}<span class="count-badge" title={t('shell.projectsDragged', { count: drag.ids.length })}>{drag.ids.length}</span>{/if}
    </span>
  </div>
{/if}

{#if menu}
  {@const target = menu.target}
  {@const paths = targets(target)}
  <div class="menu" role="menu" style:left="{menu.x}px" style:top="{menu.y}px">
    {#if target?.kind === 'project'}
      <button role="menuitem" onclick={() => pin(paths)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
        {target.project.pinned ? t('shell.unpinFromTabBar') : t('shell.pinToTabBar')}
      </button>
      <span class="rule" aria-hidden="true"></span>
      <span class="section">{t('shell.groupMoveTo')}</span>
      {#if paths.some((p) => projects.list.find((x) => x.path === p && groupOf(projects.groups, x)))}
        <button role="menuitem" onclick={() => relocate(paths, '')}>{t('shell.groupRoot')}</button>
      {/if}
      {#each flat.filter((g) => !paths.every((p) => projects.list.find((x) => x.path === p)?.group === g.id)) as group (group.id)}
        <button role="menuitem" style:padding-left="{10 + group.depth * 14}px" onclick={() => relocate(paths, group.id)}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
          {group.name}
        </button>
      {/each}
      <button role="menuitem" onclick={() => create('', paths)}>{t('shell.groupNew')}</button>
      {#if paths.length === 1}
        <span class="rule" aria-hidden="true"></span>
        <button role="menuitem" onclick={() => ((ui.settingsProject = paths[0]), (menu = null), (ui.projects = false), (ui.settings = 'projects'))}>{t('shell.projectSettings')}</button>
      {/if}
    {:else if target}
      <button role="menuitem" onclick={() => create(target.id)}>{t('shell.groupNewIn')}</button>
      <button role="menuitem" onclick={() => rename(target.id, target.name)}>{t('shell.groupRename')}</button>
      <span class="rule" aria-hidden="true"></span>
      <button role="menuitem" title={t('shell.groupDeleteHint')} onclick={() => ((menu = null), act(() => removeGroup(target.id)))}>{t('shell.groupDelete')}</button>
    {:else}
      <button role="menuitem" onclick={() => create('')}>{t('shell.groupNew')}</button>
    {/if}
  </div>
{/if}

<style>
  .panel {
    position: fixed;
    z-index: 40;
    top: 54px;
    left: 0;
    bottom: 0;
    max-width: 100vw;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 8px 8px;
    background: var(--bg-rail);
    box-shadow:
      0 0 0 1px var(--border-strong),
      16px 0 48px rgba(0, 0, 0, 0.6);
    transform-origin: 0 0;
    user-select: none;
  }

  .panel.sizing {
    transition: width 0.34s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .panel.dragging {
    cursor: grabbing;
  }

  .fresh .top,
  .fresh .search,
  .fresh .counts,
  .fresh .bar,
  .fresh .subs,
  .fresh .card {
    animation: rise 0.35s ease-out backwards;
    animation-delay: calc(110ms + var(--i, 0) * 22ms);
  }

  .fresh .search {
    --i: 1;
  }

  .fresh .counts {
    --i: 2;
  }

  .fresh .bar,
  .fresh .subs {
    --i: 3;
  }

  .fresh .card {
    animation-delay: calc(190ms + var(--i) * 22ms);
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(-5px);
    }
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px 0 8px;
  }

  .title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tools {
    display: flex;
    gap: 2px;
  }

  .tools button {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .tools button:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-dim);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    user-select: text;
  }

  kbd {
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 500 11px var(--mono);
    color: var(--text-dim);
  }

  .counts {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 4px;
  }

  .count {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    min-width: 0;
    padding: 7px 9px;
    border-radius: 5px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    --tone: var(--ok);
  }

  .count.waiting {
    --tone: var(--accent);
  }

  .count.failed {
    --tone: var(--err);
  }

  .count.done {
    --tone: var(--done);
  }

  .count:hover {
    background: var(--surface-2);
  }

  .count[aria-pressed='true'] {
    background: color-mix(in srgb, var(--tone) 9%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--tone) 60%, transparent);
  }

  .count b {
    font: 500 18px var(--mono);
    color: var(--tone);
  }

  .count.zero b {
    color: var(--text-faint);
  }

  .count small {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .error {
    margin: 0;
    padding: 0 8px;
    font-size: 12.5px;
    color: var(--err-text);
  }

  .bar {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }

  .tabs {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
    mask: linear-gradient(90deg, black calc(100% - 24px), transparent);
  }

  .tabs.searching {
    opacity: 0.45;
  }

  .tab {
    flex: none;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 11px;
    border-radius: 4px 4px 0 0;
    color: var(--text-dim);
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab.on {
    color: var(--text-strong);
  }

  .tab em,
  .chip em {
    font: normal 500 10.5px var(--mono);
    color: var(--text-faint);
  }

  .ink {
    position: absolute;
    left: 0;
    bottom: 0;
    width: var(--ink-w);
    height: 2px;
    border-radius: 2px;
    background: var(--accent);
    transform: translateX(var(--ink-x));
    transition:
      transform 0.25s cubic-bezier(0.22, 1, 0.36, 1),
      width 0.25s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .fresh .ink {
    transition: none;
  }

  .tab.target,
  .chip.target {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--accent);
    color: var(--text-strong);
  }

  .tab.dim,
  .chip.dim,
  .card.dim {
    opacity: 0.4;
  }

  .mark {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }

  .mark.waiting {
    background: var(--accent);
  }

  .mark.failed {
    background: var(--err);
  }

  .sort {
    position: relative;
    flex: none;
    margin-bottom: 4px;
  }

  .sorter {
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border-radius: 4px;
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .sorter svg {
    color: var(--text-dim);
  }

  .sorter:hover,
  .sorter.open {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .subs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .chip {
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border-radius: 13px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .chip:hover {
    color: var(--text);
  }

  .chip.on {
    background: var(--surface-4);
    box-shadow: inset 0 0 0 1px var(--border-dashed);
    color: var(--text-strong);
  }

  .editing {
    padding: 0 4px;
  }

  .name {
    width: 140px;
    height: 24px;
    padding: 0 8px;
    border: none;
    border-radius: 3px;
    outline: none;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 2px var(--border-focus);
    color: var(--text);
    font: inherit;
    font-weight: 600;
    user-select: text;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 2px;
  }

  .cards {
    columns: 280px;
    column-gap: 6px;
  }

  .none {
    margin: 16px 8px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .card {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin-bottom: 6px;
    padding: 9px 10px 9px 14px;
    border-radius: 6px;
    background-color: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
    break-inside: avoid;
  }

  .card::after {
    border-radius: inherit;
  }

  .card:hover {
    background-color: var(--surface-2);
  }

  .card.on {
    background-color: color-mix(in srgb, var(--project) 10%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 55%, transparent);
  }

  .card.selected {
    background-color: color-mix(in srgb, var(--accent-soft) 10%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-soft) 45%, transparent);
  }

  .card.cursor {
    background-color: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text);
  }

  .card.idle .nm {
    font-weight: 500;
    color: var(--text-muted);
  }

  .pinned {
    stroke: var(--project);
  }

  .run {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-2);
    white-space: nowrap;
  }

  .when {
    font: 11px var(--mono);
    color: var(--text-faint);
    white-space: nowrap;
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
  }

  .tag {
    height: 20px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 7px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-muted);
    font-size: 11.5px;
    font-weight: 500;
    white-space: nowrap;
  }

  .tag.waiting {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .tag.failed {
    background: var(--err);
    color: var(--err-ink);
    font-weight: 700;
  }

  .tag.done {
    background: color-mix(in srgb, var(--done) 14%, transparent);
    color: var(--done);
  }

  .empty {
    font-size: 12px;
    color: var(--text-faint);
  }

  .group {
    margin-left: auto;
    height: 20px;
    display: inline-flex;
    align-items: center;
    padding: 0 7px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 11px var(--mono);
    color: var(--text-dim);
    white-space: nowrap;
  }

  .add {
    flex: none;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-dashed);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .add:hover {
    background: var(--surface-3);
  }

  .grab {
    position: absolute;
    top: 0;
    right: -6px;
    bottom: 0;
    width: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: col-resize;
    touch-action: none;
  }

  .grab i {
    width: 6px;
    height: 56px;
    border-radius: 3px;
    background: var(--text-dim);
    box-shadow:
      0 0 0 3px var(--bg-rail),
      0 0 0 4px var(--border-strong);
    opacity: 0;
    transition:
      opacity 0.12s,
      height 0.2s,
      background 0.2s;
  }

  .panel:hover .grab i {
    opacity: 0.7;
  }

  .grab:hover i {
    height: 80px;
    background: var(--accent);
    opacity: 1;
  }

  .ghost {
    position: fixed;
    z-index: 60;
    width: 196px;
    height: 34px;
    pointer-events: none;
  }

  .ghost-card {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px 0 12px;
    border-radius: 4px;
    background: var(--surface-5);
    box-shadow:
      0 0 0 1px color-mix(in srgb, var(--accent) 60%, transparent),
      0 12px 30px rgba(0, 0, 0, 0.55);
    color: var(--text-strong);
    font-weight: 600;
  }

  .ghost-card.back {
    transform: translate(6px, 6px);
    opacity: 0.65;
    box-shadow: 0 0 0 1px var(--border-strong);
  }

  .ghost-card .nm {
    color: inherit;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project);
  }

  .count-badge {
    min-width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 700;
  }

  .menu {
    position: fixed;
    z-index: 70;
    width: 232px;
    max-height: 60vh;
    overflow-y: auto;
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

  .menu.sorts {
    position: absolute;
    top: 30px;
    right: 0;
    width: 280px;
    max-height: none;
    transform-origin: top right;
    animation: pop 0.14s ease-out;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.96) translateY(-4px);
    }
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--text-2);
    text-align: left;
  }

  .menu button:hover:not(:disabled) {
    background: var(--menu-hover);
    color: var(--text-strong);
  }

  .menu button:disabled {
    opacity: 0.45;
  }

  .menu svg {
    color: var(--text-dim);
  }

  .menu .check {
    color: var(--accent);
    visibility: hidden;
  }

  .menu [aria-checked='true'] .check {
    visibility: visible;
  }

  .menu small {
    margin-left: auto;
    font: 11px var(--mono);
    color: var(--text-faint);
    white-space: nowrap;
  }

  .section {
    padding: 6px 10px 2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .rule {
    height: 1px;
    margin: 4px -6px;
    background: var(--border-strong);
  }
</style>
