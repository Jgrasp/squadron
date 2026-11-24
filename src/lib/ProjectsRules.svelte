<script lang="ts">
  import Icon from './Icon.svelte'
  import { t } from './i18n/index.svelte'
  import { ordered } from './projects.svelte'
  import RuleList from './RuleList.svelte'
  import { ruleTools } from './rules'

  let { act }: { act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  let open = $state<Record<string, boolean>>({})
  const list = $derived(ordered())
</script>

<section>
  <h4>{t('settings.projectRules')}</h4>
  <p class="intro">{t('settings.rulesAllIntro')}</p>
  <div class="options">
    {#each list as project (project.path)}
      <div class="group" style:--project={project.color}>
        <div class="option">
          <span class="square" aria-hidden="true"></span>
          <span class="label">
            <strong>{project.name}</strong>
            <small>{project.rules.length ? ruleTools(project.rules) : t('settings.rulesNone')}</small>
          </span>
          <span class="count">{project.rules.length}</span>
          {#if project.rules.length}
            <button class="btn btn-ghost" aria-expanded={open[project.path]} onclick={() => (open[project.path] = !open[project.path])}>
              {open[project.path] ? t('settings.rulesHide') : t('settings.rulesShow')}
              <span class="chevron" class:up={open[project.path]}><Icon name="chevron" size={12} /></span>
            </button>
          {/if}
        </div>
        {#if open[project.path] && project.rules.length}
          <RuleList {project} {act} />
        {/if}
      </div>
    {/each}
  </div>
</section>

<style>
  .square {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--project);
  }

  .group + .group {
    border-top: 1px solid var(--border);
  }

  .count {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .chevron {
    display: grid;
    color: var(--text-dim);
  }

  .chevron.up {
    transform: rotate(180deg);
  }

  .btn {
    gap: 6px;
  }
</style>
