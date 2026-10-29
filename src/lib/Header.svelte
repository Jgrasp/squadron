<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { flip } from 'svelte/animate'
  import { calling, clock, ofProject, shelve, unread, waiting, waitingAgents } from './agents.svelte'
  import { needsYou } from './pulls.svelte'
  import { inBar } from './groups'
    import { getVersion } from '@tauri-apps/api/app'
  import { checkNow } from './update.svelte'
  import { chime } from './chime'
  import { ui, type SettingsTab } from './actions.svelte'
  import { detailOf, labelOf, shown, sortable } from './layout-slots.svelte'
  import { hint, saveSettings, settings } from './settings.svelte'
  import { editProject, ordered, orderProjects, projects } from './projects.svelte'
  import { t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'

  let { onadd }: { onadd: () => void } = $props()

  const failed = (path: string) => ofProject(path).filter((a) => a.status === 'error' && !a.errorSeen).length

  let heard = new Set<string>()
  $effect(() => {
    const asks = waitingAgents().map((a) => `${a.id}:${a.asks[0]?.id}`)
    if (settings.tab_ring === 'sound' && asks.some((ask) => !heard.has(ask))) chime()
    heard = new Set(asks)
  })


  const appWindow = getCurrentWindow()
  let headerWidth = $state(0)
  let measures: HTMLElement[] = $state([])
  let menuOpen = $state(false)
  let moreOpen = $state(false)
  let pullsOpen = $state(false)
  let search = $state('')
  let context = $state<{ path: string; x: number; y: number } | null>(null)
  const target = $derived(projects.list.find((p) => p.path === context?.path))

  const fit = $derived.by(() => {
    const available = headerWidth - 2 * 200 - 34 - 64 - 130
    let used = 0
    let count = 0
    for (const el of measures.slice(0, projects.list.length)) {
      if (!el) break
      used += el.offsetWidth + 4
      if (used > available) break
      count++
    }
    return Math.max(count, 1)
  })

  const minute = $derived(Math.floor(clock.now / 60_000) * 60_000)
  const activity = (path: string) => {
    const list = ofProject(path)
    return { busy: list.some((a) => a.status === 'working' || calling(a)), last: Math.max(0, ...list.map((a) => a.since)) }
  }
  const tabs = $derived(ordered().filter((p) => inBar(p, projects.current, activity(p.path), minute, settings.tab_hours)))

  const pullProjects = $derived(projects.list.map((project) => ({ project, count: needsYou(project.path) })).filter((entry) => entry.count > 0))
  const pullTotal = $derived(pullProjects.reduce((n, entry) => n + entry.count, 0))

  function openPulls(path: string) {
    projects.overview = projects.board = false
    projects.current = path
    projects.pulls = true
    pullsOpen = false
  }

  function pullsButton() {
    if (pullProjects.length === 1) openPulls(pullProjects[0].project.path)
    else pullsOpen = !pullsOpen
  }
  const visible = $derived.by(() => {
    const shown = tabs.slice(0, Math.max(fit, tabs.filter((p) => p.pinned).length))
    const active = projects.list.find((p) => p.path === projects.current)
    if (active && !shown.includes(active)) shown[shown.length - 1] = active
    return shown
  })

  const hidden = $derived(tabs.filter((p) => !visible.includes(p)))
  const found = $derived(
    hidden.filter((p) => p.name.toLowerCase().includes(search.trim().toLowerCase())),
  )

  let dropped = $state(false)

  function drop(path: string, index: number) {
    const order = visible.map((p) => p.path).filter((p) => p !== path)
    order.splice(index, 0, path)
    dropped = true
    requestAnimationFrame(() => (dropped = false))
    if (settings.tab_order !== 'manual') saveSettings({ ...$state.snapshot(settings), tab_order: 'manual' })
    orderProjects([...order, ...ordered().map((p) => p.path).filter((p) => !order.includes(p))])
  }

  function choose(path: string) {
    projects.overview = projects.pulls = projects.board = false
    projects.current = path
    moreOpen = false
    search = ''
  }

  function closeMenus(event: MouseEvent) {
    if (!(event.target as HTMLElement).closest('[data-menu]')) {
      menuOpen = false
      moreOpen = false
      pullsOpen = false
      context = null
    }
  }

  let version = $state('')
  getVersion()
    .then((v) => (version = v))
    .catch(() => {})
</script>

<svelte:window
  onclick={closeMenus}
  onkeydown={(e) => e.key === 'Escape' && ((menuOpen = false), (moreOpen = false), (pullsOpen = false), (context = null))}
/>

{#snippet finished(count: number)}
  <span class="badge done" role="img" aria-label={t('shell.agentsUnread', { count })} title={t('shell.agentsUnread', { count })}
    ><svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12l5 5L19 7" /></svg>{count}</span
  >
{/snippet}

{#snippet pin()}
  <svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label={t('shell.pinned')}><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
{/snippet}

<header data-tauri-drag-region bind:clientWidth={headerWidth} class:build-dev={import.meta.env.DEV}>
  <div class="start" data-tauri-drag-region>
    <div class="anchor" data-menu>
      <button
        class="brand"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <span class="triangle" aria-hidden="true"></span>
        <span class="wordmark">Squadron</span>
        {#if import.meta.env.DEV}<span class="dev">dev</span>{/if}
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
          ><path d="M6 9l6 6 6-6" /></svg
        >
      </button>
      {#if menuOpen}
        <div class="menu" role="menu" aria-label={t('shell.squadronMenu')}>
          <div class="about">
            <span class="triangle big" aria-hidden="true"></span>
            <span><strong>Squadron</strong><small>{t('shell.version', { version, platform: navigator.platform.startsWith('Mac') ? 'macOS' : 'Linux' })}</small></span>
          </div>
          {#each shown('menu.squadron') as id (id)}
            <button
              role="menuitem"
              class="item"
              onclick={() => ((menuOpen = false), id === 'updates' ? checkNow() : (ui.settings = id === 'settings' ? 'projects' : (id as SettingsTab)))}
            >
              <span><strong>{labelOf('menu.squadron', id)}</strong><small>{detailOf('menu.squadron', id)}</small></span>
            </button>
          {/each}
          <span class="rule" aria-hidden="true"></span>
          <button role="menuitem" onclick={() => appWindow.close()}>
            <span>{t('shell.quit')}</span><kbd>{hint('quit')}</kbd>
          </button>
        </div>
      {/if}
    </div>
    <button
      class="panelbtn"
      class:on={ui.projects}
      data-projects-toggle
      aria-pressed={ui.projects}
      aria-label={t('shell.projectsPanel')}
      title={t('shell.projectsPanelTitle', { key: hint('projects') })}
      onclick={() => (ui.projects = !ui.projects)}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16" /></svg>
    </button>
  </div>

  <nav aria-label={t('shell.projects')} use:sortable={{ enabled: visible.length > 1, onplace: drop }}>
    <button
      class="tab overview"
      aria-current={projects.overview ? 'page' : undefined}
      title={t('shell.overviewTitle', { key: hint('jump') })}
      onclick={() => (projects.overview = true)}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></svg
      >
      {t('shell.overview')}
      {#if waitingAgents().length}<span class="badge outline">{waitingAgents().length}</span>{/if}
    </button>
    <span class="divider" aria-hidden="true"></span>
    {#each visible as project (project.path)}
      <button
        animate:flip={{ duration: dropped ? 0 : 260 }}
        data-item={project.path}
        class="tab {ofProject(project.path).some((a) => a.status === 'working') ? `effect-${settings.tab_effect}` : ''}"
        aria-current={!projects.overview && project.path === projects.current ? 'page' : undefined}
        class:ring={settings.tab_ring !== 'off' && waiting(project.path) > 0}
        style:--project={project.color}
        onclick={() => choose(project.path)}
        oncontextmenu={(e) => (e.preventDefault(), (context = { path: project.path, x: e.clientX, y: e.clientY }))}
      >
        <span class="dot" aria-hidden="true"></span>
        {project.name}
        {#if project.pinned}{@render pin()}{/if}
        {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}{#if unread(project.path)}{@render finished(unread(project.path))}{/if}{#if needsYou(project.path)}<span
            class="badge pulls"
            role="img"
            aria-label={t('shell.pullsWaitingForYou', { count: needsYou(project.path) })}
            title={t('shell.pullsWaitingForYou', { count: needsYou(project.path) })}
            ><svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"
              ><circle cx="6" cy="6" r="2" /><circle cx="6" cy="18" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8M18 16V9a3 3 0 0 0-3-3h-4" /></svg
            >{needsYou(project.path)}</span
          >{/if}{#if failed(project.path)}<span
            class="badge error"
            role="img"
            aria-label={t('shell.agentsInError', { count: failed(project.path) })}
            title={t('shell.agentsInError', { count: failed(project.path) })}>!</span
          >{/if}
        {#if !project.exists}
          <svg class="warn" width="14" height="14" viewBox="0 0 24 24" role="img" aria-label={t('shell.folderNotFound')}
            ><path d="M12 3l9 16H3zM12 10v4M12 17h.01" /></svg
          >
        {/if}
      </button>
    {/each}
    {#if hidden.length}
      <div class="anchor" data-menu>
        <button
          class="tab more"
          class:ring={settings.tab_ring !== 'off' && hidden.some((p) => waiting(p.path))}
          aria-haspopup="menu"
          aria-expanded={moreOpen}
          onclick={() => (moreOpen = !moreOpen)}
        >
          +{hidden.length}
          {#if hidden.some((p) => waiting(p.path))}
            <span class="badge">{hidden.reduce((n, p) => n + waiting(p.path), 0)}</span>
          {/if}
          {#if hidden.some((p) => unread(p.path))}{@render finished(hidden.reduce((n, p) => n + unread(p.path), 0))}{/if}
          {#if hidden.some((p) => failed(p.path))}<span class="badge error" role="img" aria-label={t('shell.someAgentsInError')}>!</span>{/if}
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
            ><path d="M6 9l6 6 6-6" /></svg
          >
        </button>
        {#if moreOpen}
          <div class="menu right" role="menu" aria-label={t('shell.otherProjects')}>
            <label class="search">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
                ><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg
              >
              <input bind:value={search} placeholder={t('shell.searchProject')} aria-label={t('shell.searchProject')} />
            </label>
            {#each found as project (project.path)}
              <span class="entry">
                <button role="menuitem" style:--project={project.color} onclick={() => choose(project.path)}>
                  <span class="dot" aria-hidden="true"></span><span class="grow">{project.name}</span>
                  {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}{#if unread(project.path)}{@render finished(unread(project.path))}{/if}{#if failed(project.path)}<span
                      class="badge error"
                      role="img"
                      aria-label={t('shell.agentsInError', { count: failed(project.path) })}
                      title={t('shell.agentsInError', { count: failed(project.path) })}>!</span
                    >{/if}
                </button>
                <button class="pin" aria-label={t('shell.pinNamed', { name: project.name })} title={t('shell.pinToTabBar')} onclick={() => editProject(project.path, { pinned: true })}>
                  <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
                </button>
              </span>
            {/each}
            <span class="rule" aria-hidden="true"></span>
            <button role="menuitem" onclick={() => ((moreOpen = false), (ui.settings = 'projects'))}>{t('shell.reorderTabs')}</button>
          </div>
        {/if}
      </div>
    {/if}
    <button class="add" aria-label={t('shell.addProject')} title={t('shell.addProject')} onclick={onadd}>+</button>
  </nav>

  <div class="end" data-tauri-drag-region>
    {#if pullTotal}
      <div class="anchor" data-menu>
        <button
          class="prbtn"
          class:open={pullsOpen}
          aria-haspopup={pullProjects.length > 1 ? 'menu' : undefined}
          aria-expanded={pullProjects.length > 1 ? pullsOpen : undefined}
          title={t('shell.pullsForYou', { count: pullTotal })}
          aria-label={t('shell.pullsForYou', { count: pullTotal })}
          onclick={pullsButton}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
            ><circle cx="6" cy="6" r="2" /><circle cx="6" cy="18" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8M18 16V9a3 3 0 0 0-3-3h-4M13 3l-3 3 3 3" /></svg
          >
          <span class="badge">{pullTotal}</span>
        </button>
        {#if pullsOpen}
          <div class="menu right" role="menu" aria-label={t('shell.pullsToReview')}>
            <span class="section">{t('shell.pullsToReview')}</span>
            {#each pullProjects as { project, count } (project.path)}
              <button role="menuitem" style:--project={project.color} title={t('shell.pullsOfProject', { name: project.name })} onclick={() => openPulls(project.path)}>
                <span class="dot" aria-hidden="true"></span><span class="grow">{project.name}</span>
                <span class="count">{t('shell.pullCount', { count })}</span>
                <svg class="go" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
    <button class="win" aria-label={t('shell.minimize')} onclick={() => appWindow.minimize()}>–</button>
    <button class="win" aria-label={t('shell.maximize')} onclick={() => appWindow.toggleMaximize()}>□</button>
    <button class="win" aria-label={t('common.close')} onclick={() => appWindow.close()}>×</button>
  </div>

  {#if context && target}
    <div class="menu context" role="menu" aria-label={t('shell.actionsOf', { name: target.name })} data-menu style:left="{context.x}px" style:top="{context.y}px">
      <button role="menuitem" onclick={() => (editProject(target.path, { pinned: !target.pinned }), (context = null))}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
        {target.pinned ? t('shell.unpinFromTabBar') : t('shell.pinToTabBar')}
      </button>
      <button role="menuitem" onclick={() => ((ui.settingsProject = target.path), (context = null), (ui.settings = 'projects'))}>{t('shell.projectSettings')}</button>
      <button role="menuitem" onclick={() => (shelve(target), (context = null))}>{t('shell.archiveProjectEllipsis')}</button>
    </div>
  {/if}

  <div class="measure" aria-hidden="true">
    {#each tabs as project, i (project.path)}
      <span class="tab" bind:this={measures[i]}><span class="dot"></span>{project.name}{#if project.pinned}{@render pin()}{/if}</span>
    {/each}
  </div>
</header>

<style>
  header {
    position: relative;
    height: 54px;
    flex: none;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    padding: 0 12px 0 18px;
    background: var(--bg-app);
    border-bottom: 1px solid var(--border);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .start {
    justify-self: start;
    align-self: stretch;
    display: flex;
    align-items: center;
  }

  .anchor {
    position: relative;
  }

  .panelbtn {
    width: 30px;
    height: 30px;
    margin-left: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .panelbtn:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .panelbtn.on {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .prbtn {
    height: 30px;
    padding: 0 6px 0 9px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .prbtn:hover,
  .prbtn.open {
    background: var(--wait-bg);
    color: var(--wait-text);
  }

  .count {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .menu .go {
    color: var(--text-faint);
  }

  .menu button:hover .go {
    color: var(--text-strong);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    margin-left: -8px;
    padding: 0 8px;
    border-radius: 4px;
  }

  .brand[aria-expanded='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .brand svg {
    color: var(--text-dim);
  }

  .triangle {
    width: 14px;
    height: 14px;
    background: var(--accent);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  .triangle.big {
    width: 18px;
    height: 18px;
  }

  .wordmark {
    font-weight: 700;
    font-size: 12px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .menu {
    position: absolute;
    top: 42px;
    left: -8px;
    z-index: 10;
    width: 280px;
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

  .pinned {
    flex: none;
    fill: none;
    stroke: var(--project, currentColor);
    stroke-width: 2.2;
    stroke-linejoin: round;
    opacity: 0.9;
  }

  .menu.context {
    position: fixed;
    width: 220px;
  }

  .menu.context svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .menu.right {
    left: auto;
    right: 0;
    width: 260px;
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
  }

  .menu kbd {
    margin-left: auto;
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .section {
    padding: 8px 10px 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .menu .item {
    height: auto;
    padding: 9px 10px;
  }

  .item span {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .item strong {
    font-weight: 600;
  }

  .item small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .rule {
    height: 1px;
    margin: 6px 4px;
    background: var(--border-strong);
  }

  .about {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 10px 12px;
  }

  .about span:last-child {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .about small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .grow {
    flex: 1;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 10px;
    margin-bottom: 4px;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-dim);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
  }

  nav {
    display: flex;
    gap: 4px;
    padding: 4px;
    border-radius: 6px;
    background: var(--bg-tabs);
  }

  .build-dev {
    --dev: #5db5e7;
  }

  .build-dev .triangle {
    background: var(--dev);
  }

  .dev {
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--dev);
    color: #0d1b24;
    font: 700 10.5px/16px var(--mono);
    text-transform: uppercase;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 14px 0 12px;
    border-radius: 4px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .tab:global(.dragging) {
    z-index: 2;
    cursor: grabbing;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab[style] {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 55%, transparent);
  }

  .tab.ring:not([aria-current='page']) {
    background: color-mix(in srgb, var(--project, var(--accent)) 10%, transparent);
    box-shadow:
      inset 0 0 0 1.5px var(--project, var(--accent)),
      0 0 14px color-mix(in srgb, var(--project, var(--accent)) 40%, transparent);
    color: var(--text);
  }

  .tab[aria-current='page'] {
    background: color-mix(in srgb, var(--project, var(--accent)) 16%, transparent);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--project, var(--accent)) 75%, transparent),
      0 0 18px color-mix(in srgb, var(--project, var(--accent)) 28%, transparent);
    color: var(--text-strong);
    font-weight: 600;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project, var(--text-faint));
  }

  .divider {
    width: 1px;
    margin: 6px 2px;
    background: var(--border-strong);
  }

  .overview {
    padding: 0 8px 0 10px;
  }

  .badge.outline {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .tab:has(.badge) {
    padding-right: 6px;
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

  .entry {
    display: flex;
    align-items: center;
  }

  .entry > button[role='menuitem'] {
    flex: 1;
    min-width: 0;
  }

  .pin {
    flex: none;
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .pin:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .pin svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .badge.pulls {
    gap: 4px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .badge.pulls svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
  }

  .badge.done {
    gap: 3px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--ok);
    color: var(--ok);
  }

  .badge.done svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .badge.error {
    padding: 0 5px;
    background: var(--err);
    color: var(--err-ink);
    font-weight: 800;
  }

  .menu .badge {
    margin-right: 0;
  }

  .warn {
    color: var(--err);
  }

  .more {
    padding: 0 8px 0 10px;
    gap: 6px;
    background: var(--surface-5);
    color: var(--text-strong);
    font-weight: 600;
  }

  .add {
    width: 30px;
    height: 30px;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 16px;
  }

  .add:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .end {
    justify-self: end;
    align-self: stretch;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .win {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    background: var(--surface-2);
    color: #c7bcb4;
  }

  .win:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .measure {
    position: absolute;
    visibility: hidden;
    pointer-events: none;
    display: flex;
  }
</style>
