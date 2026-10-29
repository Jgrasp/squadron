<script lang="ts">
  import { ui } from './actions.svelte'
  import { failed, ofProject, waiting } from './agents.svelte'
  import { groupOf, rollup, rows, within, type Row, type Stat } from './groups'
  import { foldGroup, layout, MIN_PROJECTS, resizeProjects } from './layout.svelte'
  import { addGroup, editGroup, editProject, moveProjects, projects, removeGroup, type Project } from './projects.svelte'
  import { hint, settings } from './settings.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let panel: HTMLElement
  let query = $state('')
  let cursor = $state(-1)
  let selected = $state<string[]>([])
  let anchor = ''
  let editing = $state('')
  let draft = $state('')
  let error = $state('')
  let menu = $state<{ x: number; y: number; row: Row | null } | null>(null)
  let drag = $state<{ x: number; y: number; ids: string[]; group: boolean; label: string; color: string } | null>(null)
  let over = $state<string | null>(null)

  const list = $derived(rows(projects.groups, projects.list, layout.foldedGroups, query))
  const visible = $derived(list.filter((row) => row.kind === 'project'))
  const everyFolded = $derived(projects.groups.length > 0 && projects.groups.every((g) => layout.foldedGroups[g.id]))

  const stat = (path: string): Stat => ({ working: ofProject(path).some((a) => a.status === 'working') ? 1 : 0, waiting: waiting(path), failed: failed(path) })
  const inside = (id: string) => rollup(projects.groups, projects.list, id, stat)

  $effect(() => {
    void query
    cursor = query.trim() ? 0 : -1
  })

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
      const paths = visible.map((row) => (row.kind === 'project' ? row.project.path : ''))
      const [from, to] = [paths.indexOf(anchor), paths.indexOf(project.path)]
      if (from >= 0 && to >= 0) selected = paths.slice(Math.min(from, to), Math.max(from, to) + 1)
      return
    }
    selected = []
    open(project.path)
  }

  function toggle(id: string) {
    if (suppress) return (suppress = false)
    foldGroup(id, !layout.foldedGroups[id])
  }

  function foldAll() {
    for (const group of projects.groups) foldGroup(group.id, !everyFolded)
  }

  function field(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (!visible.length) return
      cursor = (cursor + (event.key === 'ArrowDown' ? 1 : -1) + visible.length) % visible.length
      document.querySelector('.panel .cursor')?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') {
      const row = visible[Math.max(cursor, 0)]
      if (row?.kind === 'project') open(row.project.path)
    }
  }

  async function create(parent: string, paths: string[] = []) {
    menu = null
    await act(async () => {
      const id = await addGroup(t('shell.groupNew'), parent)
      if (parent) foldGroup(parent, false)
      if (paths.length) {
        selected = []
        await moveProjects(paths, id)
      }
      rename(id, t('shell.groupNew'))
    })
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

  const targets = (row: Row | null) => (row?.kind === 'project' ? (selected.includes(row.project.path) ? selected : [row.project.path]) : [])

  function relocate(paths: string[], group: string) {
    menu = null
    selected = []
    if (group) foldGroup(group, false)
    act(() => moveProjects(paths, group))
  }

  function pin(paths: string[]) {
    menu = null
    const first = projects.list.find((p) => p.path === paths[0])
    if (first) act(() => Promise.all(paths.map((path) => editProject(path, { pinned: !first.pinned }))))
  }

  function context(event: MouseEvent, row: Row | null) {
    event.preventDefault()
    event.stopPropagation()
    menu = { x: Math.min(event.clientX, window.innerWidth - 240), y: Math.min(event.clientY, window.innerHeight - 320), row }
  }

  const flat = $derived.by(() => {
    const out: { id: string; name: string; depth: number }[] = []
    const walk = (parent: string, depth: number) => {
      for (const group of projects.groups.filter((g) => (g.parent && projects.groups.some((h) => h.id === g.parent) ? g.parent : '') === parent)) {
        out.push({ id: group.id, name: group.name, depth })
        walk(group.id, depth + 1)
      }
    }
    walk('', 0)
    return out
  })

  let press: { x: number; y: number; row: Row; started: boolean } | null = null
  let suppress = false

  function down(event: PointerEvent, row: Row) {
    if (event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || editing) return
    press = { x: event.clientX, y: event.clientY, row, started: false }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up, { once: true })
  }

  function begin(row: Row) {
    if (row.kind === 'group') return (drag = { x: 0, y: 0, ids: [row.id], group: true, label: row.name, color: '' })
    const paths = selected.includes(row.project.path) ? selected : [row.project.path]
    if (!selected.includes(row.project.path)) selected = []
    drag = { x: 0, y: 0, ids: paths, group: false, label: row.project.name, color: row.project.color }
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
      begin(press.row)
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
    if (target) foldGroup(target, false)
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

  function outside(event: PointerEvent) {
    const target = event.target as HTMLElement
    if (menu && !target.closest('.menu')) menu = null
    if (!panel.contains(target) && !target.closest('[data-projects-toggle], .menu')) ui.projects = false
  }

  $effect(() => {
    panel.querySelector('input')?.focus()
  })
</script>

<svelte:window onpointerdown={outside} />

<aside class="panel" class:dragging={!!drag} bind:this={panel} style:width="{Math.max(MIN_PROJECTS, layout.projectsWidth)}px" aria-label={t('shell.projectsPanel')}>
  <div class="top">
    <span class="title">{t('shell.projectsPanel')}</span>
    <div class="tools">
      <button aria-label={t('shell.groupNew')} title={t('shell.groupNew')} onclick={() => create('')}>
        <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10.5v5M9.5 13h5" /></svg>
      </button>
      {#if projects.groups.length}
        <button aria-label={everyFolded ? t('shell.expandAll') : t('shell.collapseAll')} title={everyFolded ? t('shell.expandAll') : t('shell.collapseAll')} onclick={foldAll}>
          <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
            {#if everyFolded}<path d="M7 9l5-5 5 5M7 15l5 5 5-5" />{:else}<path d="M7 5l5 5 5-5M7 19l5-5 5 5" />{/if}
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg>
    <input bind:value={query} placeholder={t('shell.searchProject')} aria-label={t('shell.searchProject')} spellcheck="false" onkeydown={field} />
    <kbd>{hint('projects')}</kbd>
  </label>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="tree" data-drop="" class:root={over === ''} role="presentation" oncontextmenu={(e) => context(e, null)}>
    {#each list as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
      {#if row.kind === 'group'}
        {@const sum = inside(row.id)}
        {#if editing === row.id}
          <div class="row g editing" style:--depth={row.depth}>
            <svg class="chev" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
            <svg class="fold" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
            <input
              class="name"
              bind:value={draft}
              use:select
              aria-label={t('shell.groupNameOf', { name: row.name })}
              onblur={() => finish(true)}
              onkeydown={(e) => (e.key === 'Enter' ? finish(true) : e.key === 'Escape' && (e.stopPropagation(), finish(false)))}
            />
          </div>
        {:else}
          <button
            class="row g"
            class:target={over === row.id}
            class:dim={drag?.group && drag.ids[0] === row.id}
            data-drop={row.id}
            style:--depth={row.depth}
            aria-expanded={!row.folded}
            aria-label={row.folded ? t('shell.groupUnfold', { name: row.name }) : t('shell.groupFold', { name: row.name })}
            onpointerdown={(e) => down(e, row)}
            onclick={() => toggle(row.id)}
            oncontextmenu={(e) => context(e, row)}
          >
            <svg class="chev" class:open={!row.folded} width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
            <svg class="fold" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
            <span class="nm">{row.name}</span>
            <span class="tail">
              {#if sum.working}<StatusIcon status="working" />{/if}
              {#if sum.waiting}<span class="badge" role="img" aria-label={t('shell.waitingCount', { count: sum.waiting })}>{sum.waiting}</span>{/if}
              {#if sum.failed}<span class="badge error" role="img" aria-label={t('shell.agentsInError', { count: sum.failed })}>!</span>{/if}
            </span>
          </button>
        {/if}
      {:else}
        {@const one = stat(row.project.path)}
        {@const at = visible.indexOf(row)}
        <button
          class="row p {one.working ? `effect-${settings.tab_effect}` : ''}"
          class:on={!projects.overview && row.project.path === projects.current}
          class:selected={selected.includes(row.project.path)}
          class:cursor={at === cursor}
          class:dim={drag && !drag.group && drag.ids.includes(row.project.path)}
          data-drop={row.group}
          style:--project={row.project.color}
          style:--depth={row.depth}
          aria-current={!projects.overview && row.project.path === projects.current ? 'page' : undefined}
          onpointerdown={(e) => down(e, row)}
          onclick={(e) => pick(e, row.project)}
          oncontextmenu={(e) => context(e, row)}
        >
          <span class="dot" aria-hidden="true"></span>
          <span class="nm">{row.project.name}</span>
          {#if row.project.pinned}
            <svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label={t('shell.pinned')}><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
          {/if}
          <span class="tail">
            {#if one.waiting}<span class="badge" role="img" aria-label={t('shell.waitingCount', { count: one.waiting })}>{one.waiting}</span>{/if}
            {#if one.failed}<span class="badge error" role="img" aria-label={t('shell.agentsInError', { count: one.failed })}>!</span>{/if}
          </span>
        </button>
      {/if}
    {:else}
      <p class="none">{t('shell.projectsNoMatch')}</p>
    {/each}
  </div>

  <button class="add" onclick={() => ((ui.projects = false), (ui.adding = ''))}>
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
    {t('shell.addProject')}
  </button>

  <div class="grab" role="separator" aria-orientation="vertical" aria-label={t('shell.projectsResize')} onpointerdown={resize}><i></i></div>
</aside>

{#if drag}
  <div class="ghost" style:left="{drag.x + 14}px" style:top="{drag.y + 10}px" style:--project={drag.color || 'var(--accent)'}>
    <span class="card back"></span>
    <span class="card">
      {#if drag.group}
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
      {:else}
        <span class="dot"></span>
      {/if}
      <span class="nm">{drag.label}</span>
      {#if drag.ids.length > 1}<span class="count" title={t('shell.projectsDragged', { count: drag.ids.length })}>{drag.ids.length}</span>{/if}
    </span>
  </div>
{/if}

{#if menu}
  {@const row = menu.row}
  {@const paths = targets(row)}
  <div class="menu" role="menu" style:left="{menu.x}px" style:top="{menu.y}px">
    {#if row?.kind === 'project'}
      <button role="menuitem" onclick={() => pin(paths)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
        {row.project.pinned ? t('shell.unpinFromTabBar') : t('shell.pinToTabBar')}
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
    {:else if row}
      <button role="menuitem" onclick={() => create(row.id)}>{t('shell.groupNewIn')}</button>
      <button role="menuitem" onclick={() => rename(row.id, row.name)}>{t('shell.groupRename')}</button>
      <span class="rule" aria-hidden="true"></span>
      <button role="menuitem" title={t('shell.groupDeleteHint')} onclick={() => ((menu = null), act(() => removeGroup(row.id)))}>{t('shell.groupDelete')}</button>
    {:else}
      <button role="menuitem" onclick={() => create('')}>{t('shell.groupNew')}</button>
    {/if}
  </div>
{/if}

<style>
  .panel {
    position: fixed;
    z-index: 9;
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
    user-select: none;
  }

  .panel.dragging {
    cursor: grabbing;
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

  .error {
    margin: 0;
    padding: 0 8px;
    font-size: 12.5px;
    color: var(--err-text);
  }

  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    border-radius: 4px;
  }

  .tree.root {
    box-shadow: inset 0 0 0 1.5px var(--accent);
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .none {
    margin: 16px 8px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .row {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px 0 calc(8px + var(--depth) * 16px);
    border-radius: 4px;
    color: var(--text-muted);
    text-align: left;
    white-space: nowrap;
  }

  .row::before {
    content: '';
    position: absolute;
    left: 8px;
    top: 0;
    bottom: 0;
    width: calc(var(--depth) * 16px);
    background: repeating-linear-gradient(to right, transparent 0 6px, var(--border-strong) 6px 7px, transparent 7px 16px);
    pointer-events: none;
  }

  .row:hover {
    background: color-mix(in srgb, var(--text) 6%, transparent);
    color: var(--text);
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row.g {
    font-weight: 600;
    color: var(--text-2);
  }

  .chev {
    width: 12px;
    height: 12px;
    color: var(--text-faint);
    transition: transform 0.12s;
  }

  .chev.open {
    transform: rotate(90deg);
  }

  .fold {
    color: var(--text-dim);
  }

  .row.target {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--accent);
    color: var(--text-strong);
  }

  .row.p.on {
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 70%, transparent);
    color: var(--text-strong);
    font-weight: 600;
  }

  .row.p.selected {
    background: color-mix(in srgb, var(--accent-soft) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-soft) 45%, transparent);
    color: var(--text-strong);
  }

  .row.p.cursor {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .row.dim {
    opacity: 0.4;
  }

  .editing {
    cursor: text;
  }

  .name {
    flex: 1;
    min-width: 0;
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

  .dot {
    width: 9px;
    height: 9px;
    margin-right: 1.5px;
    border-radius: 2px;
    background: var(--project, var(--text-faint));
  }

  .pinned {
    stroke: var(--project, currentColor);
    opacity: 0.9;
  }

  .tail {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .badge {
    height: 20px;
    min-width: 20px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 700;
  }

  .badge.error {
    padding: 0 5px;
    background: var(--err);
    color: var(--err-ink);
    font-weight: 800;
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
    transition: opacity 0.12s;
  }

  .grab:hover i,
  .panel:hover .grab i {
    opacity: 0.7;
  }

  .ghost {
    position: fixed;
    z-index: 60;
    width: 196px;
    height: 34px;
    pointer-events: none;
  }

  .card {
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

  .card.back {
    transform: translate(6px, 6px);
    opacity: 0.65;
    box-shadow: 0 0 0 1px var(--border-strong);
  }

  .card .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
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

  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--text-2);
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
    color: var(--text-strong);
  }

  .menu svg {
    color: var(--text-dim);
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
