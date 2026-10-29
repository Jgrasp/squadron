<script lang="ts">
  import { editProject, projects, type Project } from './projects.svelte'
  import { settings } from './settings.svelte'
  import { lookUpRuns, names, runs } from './pipelines.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()
</script>

<div class="group" class:off={!settings.runs}>
  <label class="toggle">
    <input type="checkbox" checked={project.runs} disabled={!settings.runs} onchange={(e) => act(() => editProject(project.path, { runs: e.currentTarget.checked }))} />
    {t('runs.projectShow')}
  </label>
  {#if project.runs && settings.runs && settings.runs_notify}
    <label class="toggle">
      <input type="checkbox" checked={project.runs_notify} onchange={(e) => act(() => editProject(project.path, { runs_notify: e.currentTarget.checked }))} />
      {t('runs.projectNotify')}
    </label>
  {/if}
</div>

{#if project.runs && settings.runs}
  <div class="group">
    <label class="label" for="project-repository">{t('runs.repository')}</label>
    <input
      id="project-repository"
      class="field repo"
      value={project.repository}
      placeholder={t('runs.repositoryPlaceholder')}
      title={t('runs.repositoryHint')}
      spellcheck="false"
      onchange={(e) => act(() => editProject(project.path, { repository: e.currentTarget.value }).then(() => lookUpRuns(projects.list.find((p) => p.path === project.path)!, true)))}
    />
    {#if runs[project.path]?.error}<span class="hint bad">{errorText(runs[project.path].error)}</span>{/if}
  </div>
  {#if project.runs_hidden.length || names(project).length}
    <div class="group">
      <span class="hint">{t('runs.filterHint')}</span>
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

<style>
  .group {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    max-width: 560px;
  }

  .group.off {
    opacity: 0.5;
  }

  .label {
    font-weight: 600;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  .repo {
    align-self: stretch;
    height: 36px;
    font: 12.5px var(--mono);
  }

  .hint {
    font-size: 12.5px;
    color: var(--text-dim);
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
