<script lang="ts">
  import { untrack } from 'svelte'
  import { ui } from './actions.svelte'
  import { unshelve } from './agents.svelte'
  import ProjectAgents from './ProjectAgents.svelte'
  import ProjectGeneral from './ProjectGeneral.svelte'
  import ProjectReview from './ProjectReview.svelte'
  import ProjectRuns from './ProjectRuns.svelte'
  import ProjectTickets from './ProjectTickets.svelte'
  import { home, ordered, orderProjects, projects } from './projects.svelte'
  import Select, { type Option } from './Select.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { tickets } from './tickets.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'

  type Section = 'general' | 'agents' | 'runs' | 'review' | 'tickets'

  const SECTIONS: [Section, Key][] = [
    ['general', 'settings.projectGeneral'],
    ['agents', 'settings.projectAgents'],
    ['runs', 'settings.projectRuns'],
    ['review', 'settings.projectReview'],
    ['tickets', 'tickets.projectTickets'],
  ]
  const HOURS = [1, 4, 12, 24, 0]
  const STORE = 'squadron.project-section'

  let error = $state('')
  let archivedOpen = $state(false)
  let section = $state<Section>(SECTIONS.find(([id]) => id === localStorage.getItem(STORE))?.[0] ?? 'general')
  let picked = $state(
    untrack(() => {
      const asked = ui.settingsProject
      ui.settingsProject = ''
      return asked || projects.current
    }),
  )

  const list = $derived(ordered())
  const project = $derived(list.find((p) => p.path === picked) ?? list[0])
  const index = $derived(list.findIndex((p) => p === project))
  const sections = $derived(SECTIONS.filter(([id]) => id !== 'review' || project?.git))
  const shown = $derived(sections.some(([id]) => id === section) ? section : 'general')
  const own = (path: string) => Object.keys(list.find((p) => p.path === path)?.defaults ?? {}).length

  const orders = $derived<Option[]>([
    { value: 'manual', label: t('settings.orderManual'), detail: t('settings.orderManualHint') },
    { value: 'name', label: t('common.name'), detail: t('settings.orderNameHint') },
    { value: 'activity', label: t('settings.orderActivity'), detail: t('settings.orderActivityHint') },
  ])
  const delays = $derived<Option[]>(HOURS.map((hours) => ({ value: String(hours), label: hours ? t('settings.tabHoursOption', { count: hours }) : t('settings.tabHoursPinned') })))

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = errorText(e)
    }
  }

  function open(next: Section) {
    section = next
    localStorage.setItem(STORE, next)
  }

  const movable = (by: number) => list[index + by]?.pinned === list[index].pinned

  function shift(by: number) {
    const paths = list.map((p) => p.path)
    const [path] = paths.splice(index, 1)
    paths.splice(index + by, 0, path)
    act(() => orderProjects(paths))
  }
</script>

<div class="frame">
  <div class="list">
    <div class="tabbar">
      <div class="pair">
        <span class="label">{t('settings.tabOrder')}</span>
        <Select label={t('settings.tabOrder')} value={settings.tab_order} options={orders} onpick={(order) => act(() => saveSettings({ ...$state.snapshot(settings), tab_order: order as typeof settings.tab_order }))} />
      </div>
      <div class="pair">
        <span class="label">{t('settings.tabHours')}</span>
        <Select label={t('settings.tabHours')} value={String(settings.tab_hours)} options={delays} onpick={(hours) => act(() => saveSettings({ ...$state.snapshot(settings), tab_hours: Number(hours) }))} />
      </div>
    </div>
    <ul>
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
          {#if p === project && settings.tab_order === 'manual'}
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
      {#if projects.archived.length}
        <button class="btn btn-secondary" aria-expanded={archivedOpen} onclick={() => (archivedOpen = !archivedOpen)}>{t('settings.archivedProjects')} ({projects.archived.length})</button>
        {#if archivedOpen}
          <ul class="archived">
            {#each projects.archived as p (p.path)}
              <li style:--project={p.color}>
                <span class="square" aria-hidden="true"></span>
                <span class="name">
                  <strong>{p.name}</strong>
                  <small>{home(p.path)}</small>
                </span>
                <button class="btn btn-secondary" onclick={() => act(() => unshelve(p.path))}>{t('settings.unarchive')}</button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
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
      <div class="tabs" role="tablist" aria-label={t('settings.projectSections')}>
        {#each sections as [id, name] (id)}
          <button role="tab" aria-selected={shown === id} onclick={() => open(id)}>
            {t(name)}
            {#if id === 'agents' && own(project.path)}<span class="count">{own(project.path)}</span>{/if}
          </button>
        {/each}
      </div>
      <div class="pane" role="tabpanel">
        {#if shown === 'general'}
          <ProjectGeneral {project} {act} />
        {:else if shown === 'agents'}
          <ProjectAgents {project} {act} />
        {:else if shown === 'runs'}
          <ProjectRuns {project} {act} />
        {:else if shown === 'review'}
          <ProjectReview {project} {act} />
        {:else}
          <ProjectTickets {project} />
          {#if tickets.connections && !tickets.connections.length}
            <p class="empty">{t('settings.projectTicketsNone')}</p>
            <button class="btn btn-secondary" onclick={() => (ui.settings = 'tickets')}>{t('settings.projectTicketsConnect')}</button>
          {/if}
        {/if}
        {#if error}<p class="bad" role="alert">{error}</p>{/if}
      </div>
    {:else}
      <div class="pane">
        <p class="empty">{t('settings.projectsNone')}</p>
        <button class="btn btn-primary" onclick={() => (ui.adding = '')}>{t('settings.addProject')}</button>
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

  .tabbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 8px;
    padding: 14px 12px;
    border-bottom: 1px solid var(--border);
  }

  .pair {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-size: 12px;
    color: var(--text-dim);
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

  .archived {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .archived li {
    gap: 10px;
    padding: 6px 4px 6px 8px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .archived .btn {
    height: 28px;
    padding: 0 10px;
    font-size: 12.5px;
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
    padding: 18px 28px 0;
  }

  header .square {
    width: 14px;
    height: 14px;
  }

  h3 {
    margin: 0;
    font-size: 17px;
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

  .tabs {
    display: flex;
    gap: 2px;
    padding: 12px 28px 0;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    margin-bottom: -1px;
    padding: 0 14px;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
  }

  .tabs button:hover {
    color: var(--text);
  }

  .tabs button[aria-selected='true'] {
    border-bottom-color: var(--accent);
    color: var(--text-strong);
    font-weight: 600;
  }

  .pane {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 18px;
    padding: 22px 28px 28px;
  }

  .pane > :global(*) {
    max-width: 100%;
  }

  .pane > :global(textarea),
  .pane > :global(.tickets) {
    width: 100%;
  }

  .empty {
    margin: 0;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
  }
</style>
