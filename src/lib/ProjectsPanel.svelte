<script lang="ts">
  import { cubicOut } from 'svelte/easing'
  import { ui } from './actions.svelte'
  import { failed, ofProject, waiting } from './agents.svelte'
  import { contained, groupOf, parentOf, rollup, rows, tabClosed, within, type Row, type Stat } from './groups'
  import { foldGroup, layout, MIN_PROJECTS, resizeProjects } from './layout.svelte'
  import { addGroup, editGroup, editProject, moveProjects, orderProjects, projects, removeGroup, type Project } from './projects.svelte'
  import { hint, settings } from './settings.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { errorText, t } from './i18n/index.svelte'

  type GroupRow = Extract<Row, { kind: 'group' }>

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
  let slot = $state<{ path: string; after: boolean } | null>(null)

  const searching = $derived(!!query.trim())
  const list = $derived(rows(projects.groups, projects.list, layout.foldedGroups, query))
  const visible = $derived(list.filter((row) => row.kind === 'project'))
  const everyFolded = $derived(projects.groups.length > 0 && projects.groups.every((g) => layout.foldedGroups[g.id]))
  const blocks = $derived.by(() => {
    const out: { head: GroupRow | null; rows: Row[] }[] = []
    for (const row of list) {
      if (row.depth === 0 && row.kind === 'group') out.push({ head: row, rows: [] })
      else if (row.depth === 0) {
        if (out[out.length - 1]?.head !== null) out.push({ head: null, rows: [] })
        out[out.length - 1].rows.push(row)
      } else out[out.length - 1].rows.push(row)
    }
    return out
  })

  const stat = (path: string): Stat => ({ working: ofProject(path).some((a) => a.status === 'working') ? 1 : 0, waiting: waiting(path), failed: failed(path) })
  const inside = (id: string) => rollup(projects.groups, projects.list, id, stat)
  const count = (id: string) => contained(projects.groups, projects.list, id).length
  const glow = (path: string) => (stat(path).working ? `effect-${settings.projects_effect}${settings.projects_effect === 'bar' ? ' stripe' : ''}` : '')

  $effect(() => {
    void query
    cursor = searching ? 0 : -1
  })

  $effect(() => {
    panel.querySelector('input')?.focus()
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
      requestAnimationFrame(() => panel.querySelector('.cursor')?.scrollIntoView({ block: 'nearest' }))
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

  const closed = (project: Project) => tabClosed(project, Math.max(0, ...ofProject(project.path).map((a) => a.since)))

  function hide(paths: string[]) {
    menu = null
    const first = projects.list.find((p) => p.path === paths[0])
    if (first) act(() => Promise.all(paths.map((path) => editProject(path, { tab_closed_at: closed(first) ? 0 : Date.now() }))))
  }

  function context(event: MouseEvent, row: Row | null) {
    event.preventDefault()
    event.stopPropagation()
    menu = { x: Math.min(event.clientX, window.innerWidth - 240), y: Math.min(event.clientY, window.innerHeight - 320), row }
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
    slot = place(event.clientX, event.clientY)
  }

  function place(x: number, y: number) {
    const row = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-path]')
    if (!row || !drag || drag.group || drag.ids.includes(row.dataset.path!)) return null
    const box = row.getBoundingClientRect()
    return { path: row.dataset.path!, after: y > box.top + box.height / 2 }
  }

  function placed(ids: string[], at: { path: string; after: boolean } | null) {
    if (!at) return null
    const all = projects.list.map((p) => p.path)
    const rest = all.filter((p) => !ids.includes(p))
    rest.splice(rest.indexOf(at.path) + (at.after ? 1 : 0), 0, ...all.filter((p) => ids.includes(p)))
    return rest.join('\n') === all.join('\n') ? null : rest
  }

  function up(event: PointerEvent) {
    window.removeEventListener('pointermove', move)
    const started = press?.started
    press = null
    if (!started || !drag) return
    suppress = true
    setTimeout(() => (suppress = false), 0)
    const target = aim(event.clientX, event.clientY)
    const order = placed(drag.ids, drag.group ? null : place(event.clientX, event.clientY))
    const moved = drag
    drag = null
    over = null
    slot = null
    if (target === null) return
    if (target) foldGroup(target, false)
    if (moved.group) {
      const group = projects.groups.find((g) => g.id === moved.ids[0])
      if (group && group.parent !== target) act(() => editGroup(group.id, { parent: target }))
    } else {
      const regroup = moved.ids.some((path) => projects.list.find((p) => p.path === path && groupOf(projects.groups, p) !== target))
      if (!regroup && !order) return
      selected = []
      act(async () => {
        if (regroup) await moveProjects(moved.ids, target)
        if (order) await orderProjects(order)
      })
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
</script>

<svelte:window onpointerdown={outside} />

{#snippet tail(sum: Stat)}
  <span class="tail">
    {#if sum.working}<StatusIcon status="working" />{/if}
    {#if sum.waiting}<span class="badge" role="img" aria-label={t('shell.waitingCount', { count: sum.waiting })}>{sum.waiting}</span>{/if}
    {#if sum.failed}<span class="badge error" role="img" aria-label={t('shell.agentsInError', { count: sum.failed })}>!</span>{/if}
  </span>
{/snippet}

{#snippet chevron(open: boolean)}
  <svg class="chev" class:open width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
{/snippet}

{#snippet renaming(row: GroupRow)}
  <input
    class="name"
    bind:value={draft}
    use:select
    aria-label={t('shell.groupNameOf', { name: row.name })}
    onblur={() => finish(true)}
    onkeydown={(e) => (e.key === 'Enter' ? finish(true) : e.key === 'Escape' && (e.stopPropagation(), finish(false)))}
  />
{/snippet}

{#snippet line(row: Row, depth: number)}
  {#if row.kind === 'group'}
    {#if editing === row.id}
      <div class="row g editing" style:--depth={depth}>
        {@render chevron(!row.folded)}
        {@render renaming(row)}
      </div>
    {:else}
      {@const sum = inside(row.id)}
      <div class="row g" class:target={over === row.id && !slot} class:dim={drag?.group && drag.ids[0] === row.id} class:open={menu?.row?.kind === 'group' && menu.row.id === row.id} data-drop={row.id} style:--depth={depth}>
        <button
          class="fold"
          aria-expanded={!row.folded}
          aria-label={row.folded ? t('shell.groupUnfold', { name: row.name }) : t('shell.groupFold', { name: row.name })}
          onpointerdown={(e) => down(e, row)}
          onclick={() => toggle(row.id)}
          oncontextmenu={(e) => context(e, row)}
        >
          {@render chevron(!row.folded)}
          <span class="nm">{row.name}</span>
          <span class="n">{count(row.id)}</span>
          {@render tail(row.folded || searching ? sum : { working: sum.working, waiting: 0, failed: 0 })}
        </button>
        <button class="plus" aria-label={t('shell.groupNewIn')} title={t('shell.groupNewIn')} onclick={() => create(row.id)}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
        </button>
      </div>
    {/if}
  {:else}
    {@const at = visible.indexOf(row)}
    {@const current = !projects.overview && row.project.path === projects.current}
    <button
      class="row p {glow(row.project.path)}"
      class:on={current && !searching}
      class:selected={selected.includes(row.project.path)}
      class:cursor={at === cursor}
      class:dim={drag && !drag.group && drag.ids.includes(row.project.path)}
      data-drop={row.group}
      data-path={row.project.path}
      style:--project={row.project.color}
      style:--depth={depth}
      aria-current={current ? 'page' : undefined}
      onpointerdown={(e) => down(e, row)}
      onclick={(e) => pick(e, row.project)}
      oncontextmenu={(e) => context(e, row)}
    >
      {#if slot?.path === row.project.path}<i class="slot" class:after={slot.after} aria-hidden="true"></i>{/if}
      <span class="dot" aria-hidden="true"></span>
      <span class="nm">{row.project.name}</span>
      {#if row.project.pinned}
        <svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label={t('shell.pinned')}><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
      {/if}
      {@render tail(stat(row.project.path))}
    </button>
  {/if}
{/snippet}

<aside
  class="panel"
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

  <div class="tree" data-drop="" class:root={over === '' && !slot} role="presentation" oncontextmenu={(e) => context(e, null)}>
    {#each blocks as block (block.head ? `g:${block.head.id}` : 'loose')}
      {#if block.head}
        {@const head = block.head}
        <section class="box" class:target={over === head.id && !slot} class:dim={drag?.group && drag.ids[0] === head.id} data-drop={head.id} aria-label={head.name}>
          {#if editing === head.id}
            <div class="bh editing">
              {@render chevron(!head.folded)}
              {@render renaming(head)}
            </div>
          {:else}
            <div class="bh" class:open={menu?.row?.kind === 'group' && menu.row.id === head.id}>
              <button
                class="fold"
                aria-expanded={!head.folded}
                aria-label={head.folded ? t('shell.groupUnfold', { name: head.name }) : t('shell.groupFold', { name: head.name })}
                onpointerdown={(e) => down(e, head)}
                onclick={() => toggle(head.id)}
                oncontextmenu={(e) => context(e, head)}
              >
                {@render chevron(!head.folded)}
                <span class="nm">{head.name}</span>
                <span class="n">{count(head.id)}</span>
                {@render tail(inside(head.id))}
              </button>
              <button class="plus" aria-label={t('shell.groupNewIn')} title={t('shell.groupNewIn')} onclick={() => create(head.id)}>
                <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
              </button>
            </div>
          {/if}
          {#each block.rows as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
            {@render line(row, row.depth - 1)}
          {/each}
        </section>
      {:else}
        <div class="loose">
          {#if blocks.length > 1}<span class="cap">{t('shell.groupRoot')}</span>{/if}
          {#each block.rows as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
            {@render line(row, 0)}
          {/each}
        </div>
      {/if}
    {:else}
      <p class="none">{t('shell.projectsNoMatch')}</p>
    {/each}
  </div>

  <div class="adds">
    <button class="add" onclick={() => ((ui.projects = false), (ui.adding = ''))}>
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
      {t('shell.addProject')}
    </button>
    <button class="add" onclick={() => create('')}>
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10.5v5M9.5 13h5" /></svg>
      {t('shell.groupNew')}
    </button>
  </div>

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
  {@const row = menu?.row ?? null}
  {@const paths = targets(row)}
  <div class="menu" role="menu" style:left="{menu.x}px" style:top="{menu.y}px">
    {#if row?.kind === 'project'}
      <button role="menuitem" onclick={() => pin(paths)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
        {row.project.pinned ? t('shell.unpinFromTabBar') : t('shell.pinToTabBar')}
      </button>
      <button role="menuitem" onclick={() => hide(paths)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M18 6L6 18M6 6l12 12" /></svg>
        {closed(row.project) ? t('shell.showInTabBar') : t('shell.removeFromTabBar')}
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
      <button role="menuitem" onclick={() => create(row.id)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
        {t('shell.groupNewIn')}
      </button>
      <button role="menuitem" onclick={() => rename(row.id, row.name)}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4" /></svg>
        {t('shell.groupRename')}
      </button>
      <span class="rule" aria-hidden="true"></span>
      <button role="menuitem" class="danger" title={t('shell.groupDeleteHint')} onclick={() => {
          const id = row.id
          menu = null
          act(() => removeGroup(id))
        }}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></svg>
        {t('shell.groupDelete')}
      </button>
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

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
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
    gap: 8px;
    border-radius: 6px;
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

  .box {
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .box.target {
    background: color-mix(in srgb, var(--accent) 6%, var(--surface-1));
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }

  .box.dim {
    opacity: 0.4;
  }

  .bh {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 34px;
    padding-right: 5px;
    border-radius: 4px;
  }

  .bh.editing {
    gap: 8px;
    padding: 0 8px;
  }

  .bh:hover,
  .bh.open {
    background: var(--surface-2);
  }

  .fold {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    color: var(--text);
    text-align: left;
    white-space: nowrap;
  }

  .plus {
    flex: none;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-dim);
    opacity: 0;
  }

  .row.g:not(.editing) {
    gap: 2px;
    padding: 0 5px 0 0;
  }

  .row.g .fold {
    padding-left: calc(8px + var(--depth) * 16px);
    color: inherit;
  }

  .row.g.open {
    background: var(--surface-2);
  }

  .bh:hover .plus,
  .bh.open .plus,
  .row.g:hover .plus,
  .row.g.open .plus,
  .plus:focus-visible {
    opacity: 1;
  }

  .plus:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .bh .nm {
    flex: 0 1 auto;
    font-weight: 700;
  }

  .bh .tail,
  .row.g .tail {
    margin-left: auto;
  }

  .loose {
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 0 4px;
  }

  .cap {
    padding: 6px 8px 2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .row {
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

  .row:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .n {
    font: 500 11.5px var(--mono);
    color: var(--text-faint);
  }

  .row.g {
    font-weight: 600;
    color: var(--text-2);
  }

  .row.g .nm {
    flex: 0 1 auto;
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

  .row.p {
    position: relative;
  }

  .slot {
    position: absolute;
    left: 8px;
    right: 8px;
    top: -1px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
    pointer-events: none;
  }

  .slot.after {
    top: auto;
    bottom: -1px;
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
    flex: none;
    width: 9px;
    height: 9px;
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

  .adds {
    flex: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .add {
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

  .menu .danger,
  .menu .danger svg,
  .menu .danger:hover {
    color: var(--err-text);
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
