<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { flip } from 'svelte/animate'
  import { ofProject, shelve, todayCost, waiting, waitingAgents } from './agents.svelte'
  import { needsYou } from './pulls.svelte'
  import { getVersion } from '@tauri-apps/api/app'
  import { checkNow } from './update.svelte'
  import { cost } from './format'
  import { ui, type SettingsTab } from './actions.svelte'
  import { detailOf, labelOf, shown } from './layout-slots.svelte'
  import { hint, settings } from './settings.svelte'
  import { editProject, ordered, projects } from './projects.svelte'

  let { onadd }: { onadd: () => void } = $props()

  const failed = (path: string) => ofProject(path).filter((a) => a.status === 'error').length


  const appWindow = getCurrentWindow()
  let headerWidth = $state(0)
  let measures: HTMLElement[] = $state([])
  let menuOpen = $state(false)
  let moreOpen = $state(false)
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

  const tabs = $derived(ordered())
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

  function choose(path: string) {
    projects.overview = false
    projects.current = path
    moreOpen = false
    search = ''
  }

  function closeMenus(event: MouseEvent) {
    if (!(event.target as HTMLElement).closest('[data-menu]')) {
      menuOpen = false
      moreOpen = false
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
  onkeydown={(e) => e.key === 'Escape' && ((menuOpen = false), (moreOpen = false), (context = null))}
/>

{#snippet pin()}
  <svg class="pinned" width="11" height="11" viewBox="0 0 24 24" role="img" aria-label="Pinned"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
{/snippet}

<header data-tauri-drag-region bind:clientWidth={headerWidth}>
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
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
          ><path d="M6 9l6 6 6-6" /></svg
        >
      </button>
      {#if menuOpen}
        <div class="menu" role="menu" aria-label="Squadron menu">
          <div class="about">
            <span class="triangle big" aria-hidden="true"></span>
            <span><strong>Squadron</strong><small>Version {version} · Linux</small></span>
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
            <span>Quit</span><kbd>{hint('quit')}</kbd>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <nav aria-label="Projects">
    <button
      class="tab overview"
      aria-current={projects.overview ? 'page' : undefined}
      title="Overview ({hint('jump')} jumps to the next agent waiting for you)"
      onclick={() => (projects.overview = true)}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="3" y="3" width="7" height="7" rx="1" /><rect x="14" y="3" width="7" height="7" rx="1" /><rect x="3" y="14" width="7" height="7" rx="1" /><rect x="14" y="14" width="7" height="7" rx="1" /></svg
      >
      Overview
      {#if waitingAgents().length}<span class="badge outline">{waitingAgents().length}</span>{/if}
    </button>
    <span class="divider" aria-hidden="true"></span>
    {#each visible as project (project.path)}
      <button
        animate:flip={{ duration: 260 }}
        class="tab {ofProject(project.path).some((a) => a.status === 'working') ? `effect-${settings.tab_effect}` : ''}"
        aria-current={!projects.overview && project.path === projects.current ? 'page' : undefined}
        style:--project={project.color}
        onclick={() => ((projects.overview = false), (projects.current = project.path))}
        oncontextmenu={(e) => (e.preventDefault(), (context = { path: project.path, x: e.clientX, y: e.clientY }))}
      >
        <span class="dot" aria-hidden="true"></span>
        {project.name}
        {#if project.pinned}{@render pin()}{/if}
        {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}{#if needsYou(project.path)}<span
            class="badge pulls"
            role="img"
            aria-label="{needsYou(project.path)} pull request{needsYou(project.path) > 1 ? 's' : ''} waiting for you"
            title="{needsYou(project.path)} pull request{needsYou(project.path) > 1 ? 's' : ''} waiting for you"
            ><svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"
              ><circle cx="6" cy="6" r="2" /><circle cx="6" cy="18" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8M18 16V9a3 3 0 0 0-3-3h-4" /></svg
            >{needsYou(project.path)}</span
          >{/if}{#if failed(project.path)}<span
            class="badge error"
            role="img"
            aria-label="{failed(project.path)} agent{failed(project.path) > 1 ? 's' : ''} in error"
            title="{failed(project.path)} agent{failed(project.path) > 1 ? 's' : ''} in error">!</span
          >{/if}
        {#if !project.exists}
          <svg class="warn" width="14" height="14" viewBox="0 0 24 24" role="img" aria-label="Folder not found"
            ><path d="M12 3l9 16H3zM12 10v4M12 17h.01" /></svg
          >
        {/if}
      </button>
    {/each}
    {#if hidden.length}
      <div class="anchor" data-menu>
        <button
          class="tab more"
          aria-haspopup="menu"
          aria-expanded={moreOpen}
          onclick={() => (moreOpen = !moreOpen)}
        >
          +{hidden.length}
          {#if hidden.some((p) => waiting(p.path))}
            <span class="badge">{hidden.reduce((n, p) => n + waiting(p.path), 0)}</span>
          {/if}
          {#if hidden.some((p) => failed(p.path))}<span class="badge error" role="img" aria-label="Agents in error">!</span>{/if}
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
            ><path d="M6 9l6 6 6-6" /></svg
          >
        </button>
        {#if moreOpen}
          <div class="menu right" role="menu" aria-label="Other projects">
            <label class="search">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
                ><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg
              >
              <input bind:value={search} placeholder="Search a project" aria-label="Search a project" />
            </label>
            {#each found as project (project.path)}
              <span class="entry">
                <button role="menuitem" style:--project={project.color} onclick={() => choose(project.path)}>
                  <span class="dot" aria-hidden="true"></span><span class="grow">{project.name}</span>
                  {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}{#if failed(project.path)}<span
                      class="badge error"
                      role="img"
                      aria-label="{failed(project.path)} agent{failed(project.path) > 1 ? 's' : ''} in error"
                      title="{failed(project.path)} agent{failed(project.path) > 1 ? 's' : ''} in error">!</span
                    >{/if}
                </button>
                <button class="pin" aria-label="Pin {project.name} to the tab bar" title="Pin to the tab bar" onclick={() => editProject(project.path, { pinned: true })}>
                  <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
                </button>
              </span>
            {/each}
            <span class="rule" aria-hidden="true"></span>
            <button role="menuitem" onclick={() => ((moreOpen = false), (ui.settings = 'projects'))}>Reorder and pin tabs…</button>
          </div>
        {/if}
      </div>
    {/if}
    <button class="add" aria-label="Add a project" title="Add a project" onclick={onadd}>+</button>
  </nav>

  <div class="end" data-tauri-drag-region>
    <span class="today" title="Cost of today, all agents">Today <span>{cost(todayCost())}</span></span>
    <button class="win" aria-label="Minimize" onclick={() => appWindow.minimize()}>–</button>
    <button class="win" aria-label="Maximize" onclick={() => appWindow.toggleMaximize()}>□</button>
    <button class="win" aria-label="Close" onclick={() => appWindow.close()}>×</button>
  </div>

  {#if context && target}
    <div class="menu context" role="menu" aria-label="{target.name} actions" data-menu style:left="{context.x}px" style:top="{context.y}px">
      <button role="menuitem" onclick={() => (editProject(target.path, { pinned: !target.pinned }), (context = null))}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
        {target.pinned ? 'Unpin from the tab bar' : 'Pin to the tab bar'}
      </button>
      <button role="menuitem" onclick={() => ((context = null), (ui.settings = 'projects'))}>Project settings…</button>
      <button role="menuitem" onclick={() => (shelve(target), (context = null))}>Archive project…</button>
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

  .tab:hover {
    color: var(--text);
  }

  .tab[style] {
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 32%, transparent);
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

  .today {
    margin-right: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .today span {
    font-family: var(--mono);
    color: var(--text-3);
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
