<script lang="ts">
  import { editProject, projects, type Project } from './projects.svelte'
  import { settings } from './settings.svelte'
  import { lookUpRuns, names, runs } from './pipelines.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()
</script>

<section>
  <h4>{t('settings.projectRuns')}</h4>
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

  .name:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }
</style>
