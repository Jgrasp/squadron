<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import { shelve, unshelve } from './agents.svelte'
  import ColorPick from './ColorPick.svelte'
  import { editProject, home, move, ordered, orderProjects, projects } from './projects.svelte'
  import { saveSettings, settings, type TabOrder } from './settings.svelte'


  let error = $state('')
  const list = $derived(ordered())
  const ORDERS: [TabOrder, string, string][] = [
    ['manual', 'By hand', 'The order you set by dragging the tabs, or with the arrows below'],
    ['name', 'Name', 'Alphabetical order'],
    ['activity', 'Last activity', 'The project whose agents moved last comes first'],
  ]
  const movable = (index: number, by: number) => list[index + by]?.pinned === list[index].pinned

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = String(e)
    }
  }

  function shift(index: number, by: number) {
    const paths = list.map((p) => p.path)
    const [path] = paths.splice(index, 1)
    paths.splice(index + by, 0, path)
    act(() => orderProjects(paths))
  }

  async function relocate(path: string) {
    const chosen = await open({ directory: true, defaultPath: path })
    if (typeof chosen === 'string') act(() => move(path, chosen))
  }
</script>

<div class="top">
  <p class="intro">Pinned projects always keep their tab, first in the bar. The others follow this order and move to the + menu when the bar is full.</p>
  <button class="btn btn-primary" onclick={() => (ui.adding = '')}>+ Add a project</button>
</div>
<span class="label">Tab order</span>
<div class="orders" role="group" aria-label="Tab order">
  {#each ORDERS as [id, title, detail] (id)}
    <button aria-pressed={settings.tab_order === id} onclick={() => act(() => saveSettings({ ...$state.snapshot(settings), tab_order: id }))}>
      <strong>{title}</strong>
      <span class="hint">{detail}</span>
    </button>
  {/each}
</div>
<ul class="projects">
  {#each list as project, i (project.path)}
    <li style:--project={project.color}>
      {#if settings.tab_order === 'manual'}
        <span class="order">
          <button aria-label="Move {project.name} up" disabled={!movable(i, -1)} onclick={() => shift(i, -1)}>↑</button>
          <button aria-label="Move {project.name} down" disabled={!movable(i, 1)} onclick={() => shift(i, 1)}>↓</button>
        </span>
      {/if}
      <span class="main">
        <span class="line">
          <span class="square" aria-hidden="true"></span>
          <input
            class="field"
            value={project.name}
            aria-label="Name of {project.name}"
            onchange={(e) => act(() => editProject(project.path, { name: e.currentTarget.value }))}
          />
          <button class="pin" aria-pressed={project.pinned} onclick={() => act(() => editProject(project.path, { pinned: !project.pinned }))}>
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
            {project.pinned ? 'Pinned' : 'Pin'}
          </button>
        </span>
        <ColorPick value={project.color} label="Color of {project.name}" onpick={(color) => act(() => editProject(project.path, { color }))} />
        <span class="folder">
          <code>{home(project.path)}</code>
          {#if !project.exists}<span class="missing">Folder not found</span>{/if}
          <button class="link" onclick={() => relocate(project.path)}>Change the folder…</button>
          <button class="link quiet" onclick={() => act(() => shelve(project))}>Archive…</button>
        </span>
      </span>
    </li>
  {/each}
</ul>
{#if projects.archived.length}
  <h3>Archived projects</h3>
  <ul class="projects">
    {#each projects.archived as project (project.path)}
      <li class="archived" style:--project={project.color}>
        <span class="square" aria-hidden="true"></span>
        <span class="main">
          <strong>{project.name}</strong>
          <code>{home(project.path)}</code>
        </span>
        <button class="btn btn-secondary" onclick={() => act(() => unshelve(project.path))}>Unarchive</button>
      </li>
    {/each}
  </ul>
{/if}
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<style>
  .top {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 14px;
  }

  .top .btn {
    flex: none;
    height: 34px;
    padding: 0 14px;
  }

  .intro {
    flex: 1;
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .label {
    display: block;
    margin-bottom: 8px;
    font-weight: 600;
  }

  .orders {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
    margin-bottom: 18px;
  }

  .orders button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 14px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    text-align: left;
  }

  .orders button[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
  }

  .orders strong {
    font-weight: 600;
  }

  .orders .hint {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .projects {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    gap: 12px;
    padding: 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .order {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .order button {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .order button:disabled {
    opacity: 0.3;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .square {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--project);
  }

  .field {
    flex: 1;
    height: 34px;
  }

  .pin {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-muted);
  }

  .pin svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .pin[aria-pressed='true'] {
    background: var(--surface-4);
    box-shadow: inset 0 0 0 1px var(--project);
    color: var(--text-strong);
    font-weight: 600;
  }

  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .missing {
    color: var(--err-text);
  }

  .link {
    color: var(--accent-link);
    text-decoration: underline;
  }

  .bad {
    color: var(--err-text);
  }

  .quiet {
    margin-left: auto;
    color: var(--text-dim);
  }

  h3 {
    margin: 22px 0 10px;
    font-size: 13px;
  }

  .archived {
    align-items: center;
  }

  .archived .main {
    gap: 2px;
  }

  .archived code {
    font-size: 12px;
    color: var(--text-dim);
  }

  .archived .btn {
    height: 30px;
    padding: 0 12px;
  }
</style>
