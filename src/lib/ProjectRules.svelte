<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { ui } from './actions.svelte'
  import Icon from './Icon.svelte'
  import { t } from './i18n/index.svelte'
  import { type Project } from './projects.svelte'
  import RuleList from './RuleList.svelte'
  import { ruleTools } from './rules'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  let open = $state(false)
  let section = $state<HTMLElement>()

  const tools = $derived(ruleTools(project.rules))

  onMount(async () => {
    if (ui.settingsAnchor !== 'rules') return
    ui.settingsAnchor = ''
    open = true
    await tick()
    section?.scrollIntoView({ block: 'start' })
  })
</script>

<section bind:this={section}>
  <h4>{t('settings.projectRules')}</h4>
  <p class="intro">{t('settings.rulesIntro', { name: project.name })}</p>
  <div class="options">
    {#if project.rules.length}
      <div class="option">
        <span class="label">
          <strong>{t('settings.rulesCount', { count: project.rules.length })}</strong>
          <small>{tools}</small>
        </span>
        <button class="btn btn-ghost" aria-expanded={open} onclick={() => (open = !open)}>
          {open ? t('settings.rulesHide') : t('settings.rulesShow')}
          <span class="chevron" class:up={open}><Icon name="chevron" size={12} /></span>
        </button>
      </div>
      {#if open}
        <RuleList {project} {act} />
      {/if}
    {:else}
      <div class="option">
        <span class="label">
          <strong>{t('settings.rulesNone')}</strong>
          <small>{t('settings.rulesEmpty')}</small>
        </span>
      </div>
    {/if}
  </div>
</section>

<style>
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
