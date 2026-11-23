<script lang="ts">
  import { editProject, projects, type Project } from './projects.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { lookUpRuns, names, runs } from './pipelines.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import Select from './Select.svelte'

  let { project, act }: { project?: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  const LINGERS = [1, 2, 5, 10, 15, 30, 60]
  const lingers = $derived(LINGERS.map((minutes) => ({ value: String(minutes), label: t('runs.lingerMinutes', { count: minutes }) })))
  let hide = $state('')
  const seen = $derived([...new Set(projects.list.flatMap(names))].filter((n) => !settings.runs_hidden.includes(n)).sort())
  const change = (changes: Partial<typeof settings>) => act(() => saveSettings({ ...$state.snapshot(settings), ...changes }))

  function hideName(event: Event) {
    event.preventDefault()
    const name = hide.trim()
    if (name && !settings.runs_hidden.includes(name)) change({ runs_hidden: [...settings.runs_hidden, name] })
    hide = ''
  }
</script>

<section>
  <h4>{t('settings.projectRuns')}</h4>
  {#if !project}
    <p class="intro">{t('runs.settingsIntro')}</p>
    <div class="options">
      <label class="option">
        <span class="label"><strong>{t('runs.projectShow')}</strong><small>{t('runs.showHint')}</small></span>
        <input type="checkbox" class="switch" checked={settings.runs} onchange={(e) => change({ runs: e.currentTarget.checked })} />
      </label>
      {#if settings.runs}
        <div class="option sub">
          <span class="label"><strong>{t('runs.linger')}</strong></span>
          <Select label={t('runs.linger')} value={String(settings.runs_linger)} options={lingers} onpick={(value) => change({ runs_linger: Number(value) })} />
        </div>
        <div class="option column sub">
          <span class="label">
            <strong>{t('runs.hiddenEverywhere')}</strong>
            {#if !settings.runs_hidden.length}<small>{t('runs.hiddenNone')}</small>{/if}
          </span>
          {#if settings.runs_hidden.length}
            <span class="names">
              {#each settings.runs_hidden as name (name)}
                <button class="name" title={t('runs.unhide', { name })} aria-label={t('runs.unhide', { name })} onclick={() => change({ runs_hidden: settings.runs_hidden.filter((n) => n !== name) })}>
                  {name}<span aria-hidden="true">×</span>
                </button>
              {/each}
            </span>
          {/if}
          <form class="add" onsubmit={hideName}>
            <input class="field" list="runs-seen" placeholder={t('runs.hideName')} aria-label={t('runs.hideName')} bind:value={hide} />
            <datalist id="runs-seen">{#each seen as name (name)}<option value={name}></option>{/each}</datalist>
            <button class="btn btn-ghost" disabled={!hide.trim()}>{t('common.add')}</button>
          </form>
        </div>
      {/if}
    </div>
  {:else}
    <div class="options" class:off={!settings.runs}>
      <label class="option">
        <span class="label"><strong>{t('runs.projectShow')}</strong></span>
        <input type="checkbox" class="switch" checked={project.runs} disabled={!settings.runs} onchange={(e) => act(() => editProject(project.path, { runs: e.currentTarget.checked }))} />
      </label>
      {#if project.runs && settings.runs}
        {#if settings.notify_runs !== 'never' && settings.notify_when !== 'never'}
          <label class="option sub">
            <span class="label"><strong>{t('runs.projectNotify')}</strong></span>
            <input type="checkbox" class="switch" checked={project.runs_notify} onchange={(e) => act(() => editProject(project.path, { runs_notify: e.currentTarget.checked }))} />
          </label>
        {/if}
        <div class="option sub">
          <label class="label" for="project-repository">
            <strong>{t('runs.repository')}</strong>
            {#if runs[project.path]?.error}<small class="bad">{errorText(runs[project.path].error)}</small>{:else}<small>{t('runs.repositoryHint')}</small>{/if}
          </label>
          <input
            id="project-repository"
            class="field repo"
            value={project.repository}
            placeholder={t('runs.repositoryShort')}
            spellcheck="false"
            onchange={(e) => act(() => editProject(project.path, { repository: e.currentTarget.value }).then(() => lookUpRuns(projects.list.find((p) => p.path === project.path)!, true)))}
          />
        </div>
        {#if project.runs_hidden.length || names(project).length}
          <div class="option column sub">
            <span class="label">
              <strong>{t('runs.workflows')}</strong>
              <small>{t('runs.filterHint')}</small>
            </span>
            <span class="names">
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
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .off {
    opacity: 0.5;
  }

  .repo {
    font-family: var(--mono);
  }

  .bad {
    color: var(--err-text);
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

  .names .name span {
    margin-left: 8px;
  }

  .name:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }
</style>
