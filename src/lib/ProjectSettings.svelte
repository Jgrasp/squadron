<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import { shelve, unshelve } from './agents.svelte'
  import ColorPick from './ColorPick.svelte'
  import { editProject, home, move, ordered, orderProjects, projects } from './projects.svelte'
  import { saveSettings, settings, type TabOrder } from './settings.svelte'
  import { names, runs } from './pipelines.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'

  let error = $state('')
  const list = $derived(ordered())
  const ORDERS: [TabOrder, Key, Key][] = [
    ['manual', 'settings.orderManual', 'settings.orderManualHint'],
    ['name', 'common.name', 'settings.orderNameHint'],
    ['activity', 'settings.orderActivity', 'settings.orderActivityHint'],
  ]
  const movable = (index: number, by: number) => list[index + by]?.pinned === list[index].pinned

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = errorText(e)
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
  <p class="intro">{t('settings.projectsIntro')}</p>
  <button class="btn btn-primary" onclick={() => (ui.adding = '')}>{t('settings.addProject')}</button>
</div>
<span class="label">{t('settings.tabOrder')}</span>
<div class="orders" role="group" aria-label={t('settings.tabOrder')}>
  {#each ORDERS as [id, title, detail] (id)}
    <button aria-pressed={settings.tab_order === id} onclick={() => act(() => saveSettings({ ...$state.snapshot(settings), tab_order: id }))}>
      <strong>{t(title)}</strong>
      <span class="hint">{t(detail)}</span>
    </button>
  {/each}
</div>
<ul class="projects">
  {#each list as project, i (project.path)}
    <li style:--project={project.color}>
      {#if settings.tab_order === 'manual'}
        <span class="order">
          <button aria-label={t('settings.moveUp', { name: project.name })} disabled={!movable(i, -1)} onclick={() => shift(i, -1)}>↑</button>
          <button aria-label={t('settings.moveDown', { name: project.name })} disabled={!movable(i, 1)} onclick={() => shift(i, 1)}>↓</button>
        </span>
      {/if}
      <span class="main">
        <span class="line">
          <span class="square" aria-hidden="true"></span>
          <input
            class="field"
            value={project.name}
            aria-label={t('settings.nameOf', { name: project.name })}
            onchange={(e) => act(() => editProject(project.path, { name: e.currentTarget.value }))}
          />
          <button class="pin" aria-pressed={project.pinned} onclick={() => act(() => editProject(project.path, { pinned: !project.pinned }))}>
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
            {project.pinned ? t('settings.pinned') : t('settings.pin')}
          </button>
        </span>
        <ColorPick value={project.color} label={t('settings.colorOf', { name: project.name })} onpick={(color) => act(() => editProject(project.path, { color }))} />
        <span class="folder">
          <code>{home(project.path)}</code>
          {#if !project.exists}<span class="missing">{t('settings.folderMissing')}</span>{/if}
          <button class="link" onclick={() => relocate(project.path)}>{t('settings.changeFolder')}</button>
          <button class="link quiet" onclick={() => act(() => shelve(project))}>{t('settings.archiveEllipsis')}</button>
        </span>
        <span class="runs" class:off={!settings.runs}>
          <label class="toggle">
            <input type="checkbox" checked={project.runs} disabled={!settings.runs} onchange={(e) => act(() => editProject(project.path, { runs: e.currentTarget.checked }))} />
            {t('runs.projectShow')}
          </label>
          {#if project.runs && settings.runs}
            <input
              class="field repo"
              value={project.repository}
              placeholder={t('runs.repositoryPlaceholder')}
              aria-label={t('runs.repository')}
              title={t('runs.repositoryHint')}
              spellcheck="false"
              onchange={(e) => act(() => editProject(project.path, { repository: e.currentTarget.value }))}
            />
          {/if}
        </span>
        {#if project.runs && settings.runs}
          {#if runs[project.path]?.error}<span class="hint bad">{errorText(runs[project.path].error)}</span>{/if}
          {#if project.runs_hidden.length || names(project).length}
            <span class="names">
              <span class="hint">{t('runs.filterHint')}</span>
              {#each project.runs_hidden as name (name)}
                <button class="name hidden" aria-pressed="false" title={t('runs.unhide', { name })} onclick={() => act(() => editProject(project.path, { runs_hidden: project.runs_hidden.filter((n) => n !== name) }))}>
                  <s>{name}</s>
                </button>
              {/each}
              {#each names(project).filter((n) => !project.runs_hidden.includes(n) && !settings.runs_hidden.includes(n)) as name (name)}
                <button class="name" aria-pressed="true" title={t('runs.hideHere', { name })} onclick={() => act(() => editProject(project.path, { runs_hidden: [...project.runs_hidden, name] }))}>
                  {name}
                </button>
              {/each}
            </span>
          {/if}
        {/if}
      </span>
    </li>
  {/each}
</ul>
{#if projects.archived.length}
  <h3>{t('settings.archivedProjects')}</h3>
  <ul class="projects">
    {#each projects.archived as project (project.path)}
      <li class="archived" style:--project={project.color}>
        <span class="square" aria-hidden="true"></span>
        <span class="main">
          <strong>{project.name}</strong>
          <code>{home(project.path)}</code>
        </span>
        <button class="btn btn-secondary" onclick={() => act(() => unshelve(project.path))}>{t('settings.unarchive')}</button>
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

  .runs {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .runs.off {
    opacity: 0.5;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  .repo {
    flex: 1;
    height: 30px;
    font: 12.5px var(--mono);
  }

  .hint {
    font-size: 12.5px;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .name {
    height: 26px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .name.hidden {
    background: none;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-faint);
  }

  .name:hover {
    background: var(--surface-5);
    color: var(--text-strong);
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
