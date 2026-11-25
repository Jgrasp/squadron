<script lang="ts">
  import { untrack } from 'svelte'
  import { ui } from './actions.svelte'
  import { retire } from './agents.svelte'
  import Icon from './Icon.svelte'
  import ProjectAgents from './ProjectAgents.svelte'
  import ProjectGeneral from './ProjectGeneral.svelte'
  import ProjectPage from './ProjectPage.svelte'
  import ProjectReview from './ProjectReview.svelte'
  import ProjectRules from './ProjectRules.svelte'
  import ProjectsRules from './ProjectsRules.svelte'
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
          <ProjectPage>
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
          </ProjectPage>
        </div>
      {/key}
    {:else}
      <header>
        <span class="all"><Icon name="grid" /></span>
        <h3>{t('settings.allProjects')}</h3>
        <span class="sub">{t('settings.allProjectsSub')}</span>
      </header>
      <div class="page">
        <ProjectPage>
          {#if error}<p class="bad" role="alert">{error}</p>{/if}
          <ProjectAgents {act} {pick} />
          {#if list.length}<ProjectsRules {act} />{/if}
          <ProjectRuns {act} />
          <ProjectReview {act} />
          {#if !list.length}<p class="empty">{t('settings.projectsNone')}</p>{/if}
        </ProjectPage>
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

  .empty {
    margin: 0;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
  }
</style>
