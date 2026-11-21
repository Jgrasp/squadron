<script lang="ts">
  import { untrack } from 'svelte'
  import { ui } from './actions.svelte'
  import { retire } from './agents.svelte'
  import Icon from './Icon.svelte'
  import ProjectAgents from './ProjectAgents.svelte'
  import ProjectGeneral from './ProjectGeneral.svelte'
  import ProjectReview from './ProjectReview.svelte'
  import ProjectRules from './ProjectRules.svelte'
  import ProjectRuns from './ProjectRuns.svelte'
  import { home, ordered, orderProjects, projects } from './projects.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let error = $state('')
  let picked = $state(
    untrack(() => {
      const asked = ui.settingsProject
      ui.settingsProject = ''
      return asked === '*' ? '' : asked || projects.current
    }),
  )

  const list = $derived(ordered())
  const project = $derived(picked === '' ? undefined : (list.find((p) => p.path === picked) ?? list[0]))
  const index = $derived(list.findIndex((p) => p === project))
  const pick = (path: string) => (picked = path)
  const own = (path: string) => Object.keys(list.find((p) => p.path === path)?.defaults ?? {}).length

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = errorText(e)
    }
  }

  const movable = (by: number) => list[index + by]?.pinned === list[index].pinned

  function shift(by: number) {
    const paths = list.map((p) => p.path)
    const [path] = paths.splice(index, 1)
    paths.splice(index + by, 0, path)
    act(async () => {
      if (settings.tab_order !== 'manual') await saveSettings({ ...$state.snapshot(settings), tab_order: 'manual' })
      await orderProjects(paths)
    })
  }
</script>

<div class="frame">
  <div class="list">
    <ul>
      <li>
        <button class="item" aria-current={!project} onclick={() => (picked = '')}>
          <span class="all"><Icon name="grid" /></span>
          <span class="name">
            <strong>{t('settings.allProjects')}</strong>
            <small class="plain">{t('settings.allProjectsHint')}</small>
          </span>
        </button>
      </li>
      <li class="sep" aria-hidden="true"></li>
      {#each list as p (p.path)}
        <li style:--project={p.color}>
          <button class="item" aria-current={p === project} onclick={() => (picked = p.path)}>
            <span class="square" aria-hidden="true"></span>
            <span class="name">
              <strong>{p.name}</strong>
              <small>{home(p.path)}</small>
            </span>
            {#if own(p.path)}<span class="count" title={t('settings.projectOverrides', { count: own(p.path) })}>{own(p.path)}</span>{/if}
          </button>
          {#if p === project}
            <span class="move">
              <button aria-label={t('settings.moveUp', { name: p.name })} disabled={!movable(-1)} onclick={() => shift(-1)}>↑</button>
              <button aria-label={t('settings.moveDown', { name: p.name })} disabled={!movable(1)} onclick={() => shift(1)}>↓</button>
            </span>
          {/if}
        </li>
      {/each}
    </ul>
    <div class="foot">
      <button class="btn btn-primary" onclick={() => (ui.adding = '')}>{t('settings.addProject')}</button>
    </div>
  </div>

  <div class="detail" style:--project={project?.color}>
    {#if project}
      <header>
        <span class="square" aria-hidden="true"></span>
        <h3>{project.name}</h3>
        <code>{home(project.path)}</code>
        {#if !project.exists}<span class="missing">{t('settings.folderMissing')}</span>{/if}
      </header>
      {#key project.path}
        <div class="page">
          {#if error}<p class="bad" role="alert">{error}</p>{/if}
          <ProjectGeneral {project} {act} />
          <ProjectAgents {project} {act} {pick} />
          <ProjectRules {project} {act} />
          <ProjectRuns {project} {act} />
          {#if project.git}<ProjectReview {project} {act} />{/if}
          <section>
            <div class="options">
              <div class="option">
                <span class="label">
                  <strong>{t('settings.removeProject')}</strong>
                  <small>{t('settings.removeProjectHint')}</small>
                </span>
                <button class="btn btn-ghost" onclick={() => act(() => retire(project))}>{t('settings.removeEllipsis')}</button>
              </div>
            </div>
          </section>
        </div>
      {/key}
    {:else}
      <header>
        <span class="all"><Icon name="grid" /></span>
        <h3>{t('settings.allProjects')}</h3>
        <span class="sub">{t('settings.allProjectsSub')}</span>
      </header>
      <div class="page">
        {#if error}<p class="bad" role="alert">{error}</p>{/if}
        <ProjectAgents {act} {pick} />
        {#if !list.length}<p class="empty">{t('settings.projectsNone')}</p>{/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .frame {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
  }

  .list {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .list > ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
  }

  li {
    position: relative;
    display: flex;
    align-items: center;
  }

  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 4px;
    text-align: left;
  }

  .item:hover {
    background: var(--surface-2);
  }

  .item[aria-current='true'] {
    background: var(--surface-4);
    box-shadow: inset 2px 0 0 var(--project);
  }

  .sep {
    height: 1px;
    margin: 6px 4px;
    background: var(--border);
  }

  .all {
    flex: none;
    display: grid;
    place-items: center;
    width: 14px;
    color: var(--text-2);
  }

  .square {
    flex: none;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--project);
  }

  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .name strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .name small.plain {
    font: 12px var(--sans);
  }

  .name small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .count {
    flex: none;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 700;
  }

  .item .count {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .move {
    position: absolute;
    right: 6px;
    display: flex;
    gap: 4px;
  }

  .item:has(+ .move) .count {
    margin-right: 62px;
  }

  .move button {
    width: 26px;
    height: 26px;
    border-radius: 4px;
    background: var(--surface-5);
    color: var(--text-2);
  }

  .move button:disabled {
    opacity: 0.3;
  }

  .foot {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 50%;
    overflow-y: auto;
    padding: 12px;
    border-top: 1px solid var(--border);
  }

  .foot .btn {
    height: 34px;
    flex: none;
  }

  .detail {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 28px 14px;
    border-bottom: 1px solid var(--border);
  }

  header .square {
    width: 14px;
    height: 14px;
  }

  h3 {
    margin: 0;
    font-size: 17px;
  }

  header .sub {
    font-size: 12px;
    color: var(--text-dim);
  }

  header code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-dim);
  }

  .missing,
  .bad {
    color: var(--err-text);
  }

  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 22px 28px 40px;
  }

  .page > :global(*) {
    max-width: 720px;
  }

  .page :global(section) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .page :global(h4) {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .page :global(.intro) {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  .page :global(.options) {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .page :global(.option) {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .page :global(.option + .option) {
    border-top: 1px solid var(--border);
  }

  .page :global(.option.sub) {
    min-height: 44px;
    padding-left: 40px;
  }

  .page :global(.option.column) {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    padding: 12px 14px 14px;
  }

  .page :global(.option.column.sub) {
    padding-left: 40px;
  }

  .page :global(.option .label) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .page :global(.option strong) {
    font-size: 13px;
    font-weight: 600;
  }

  .page :global(.option small) {
    color: var(--text-dim);
    font-size: 12px;
  }

  .page :global(.option .field),
  .page :global(.option .select) {
    flex: none;
    width: 260px;
  }

  .page :global(.option .field) {
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .page :global(.option .btn) {
    flex: none;
    height: 32px;
  }

  .page :global(.switch) {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .page :global(.switch::before) {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .page :global(.switch:checked) {
    background: var(--accent);
  }

  .page :global(.switch:checked::before) {
    transform: translateX(14px);
    background: var(--ink);
  }

  .page :global(.switch:disabled) {
    cursor: default;
    opacity: 0.5;
  }

  .page :global(.switch:focus-visible) {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .empty {
    margin: 0;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
  }
</style>
