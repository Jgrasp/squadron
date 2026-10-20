<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { waiting } from './agents.svelte'
  import { projects } from './projects.svelte'

  let { onadd }: { onadd: () => void } = $props()

  const appWindow = getCurrentWindow()
  let headerWidth = $state(0)
  let measures: HTMLElement[] = $state([])
  let menuOpen = $state(false)
  let moreOpen = $state(false)
  let search = $state('')

  const fit = $derived.by(() => {
    const available = headerWidth - 2 * 170 - 34 - 64
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

  const visible = $derived.by(() => {
    const shown = projects.list.slice(0, fit)
    const active = projects.list.find((p) => p.path === projects.current)
    if (active && !shown.includes(active)) shown[shown.length - 1] = active
    return shown
  })

  const hidden = $derived(projects.list.filter((p) => !visible.includes(p)))
  const found = $derived(
    hidden.filter((p) => p.name.toLowerCase().includes(search.trim().toLowerCase())),
  )

  function choose(path: string) {
    projects.current = path
    moreOpen = false
    search = ''
  }

  function closeMenus(event: MouseEvent) {
    if (!(event.target as HTMLElement).closest('[data-menu]')) {
      menuOpen = false
      moreOpen = false
    }
  }
</script>

<svelte:window
  onclick={closeMenus}
  onkeydown={(e) => e.key === 'Escape' && ((menuOpen = false), (moreOpen = false))}
/>

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
            <span><strong>Squadron</strong><small>Version 0.1.0 · Linux</small></span>
          </div>
          <button role="menuitem" onclick={() => appWindow.close()}>
            <span>Quit</span><kbd>Ctrl Q</kbd>
          </button>
        </div>
      {/if}
    </div>
  </div>

  <nav aria-label="Projects">
    {#each visible as project (project.path)}
      <button
        class="tab"
        aria-current={project.path === projects.current ? 'page' : undefined}
        onclick={() => (projects.current = project.path)}
      >
        <span class="dot" aria-hidden="true"></span>
        {project.name}
        {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}
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
              <button role="menuitem" onclick={() => choose(project.path)}>
                <span class="dot" aria-hidden="true"></span><span class="grow">{project.name}</span>
                {#if waiting(project.path)}<span class="badge">{waiting(project.path)}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
    <button class="add" aria-label="Add a project" title="Add a project" onclick={onadd}>+</button>
  </nav>

  <div class="end" data-tauri-drag-region>
    <button class="win" aria-label="Minimize" onclick={() => appWindow.minimize()}>–</button>
    <button class="win" aria-label="Maximize" onclick={() => appWindow.toggleMaximize()}>□</button>
    <button class="win" aria-label="Close" onclick={() => appWindow.close()}>×</button>
  </div>

  <div class="measure" aria-hidden="true">
    {#each projects.list as project, i (project.path)}
      <span class="tab" bind:this={measures[i]}><span class="dot"></span>{project.name}</span>
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

  .tab[aria-current='page'] {
    background: rgba(224, 138, 91, 0.16);
    box-shadow:
      inset 0 0 0 1px rgba(224, 138, 91, 0.75),
      0 0 18px rgba(224, 138, 91, 0.28);
    color: var(--text-strong);
    font-weight: 600;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--text-faint);
  }

  .tab[aria-current='page'] .dot {
    background: var(--accent);
  }

  .badge {
    height: 20px;
    min-width: 20px;
    padding: 0 6px;
    margin-right: -8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 700;
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
