<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { ui } from './actions.svelte'
  import Icon from './Icon.svelte'
  import { t } from './i18n/index.svelte'
  import { removeRule, type Project } from './projects.svelte'
  import { ruleLabel } from './rules'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  let open = $state(false)
  let section = $state<HTMLElement>()
  let whole = $state<string | null>(null)

  const toolOf = (rule: string) => (rule.startsWith('Bash(') ? 'Bash' : rule.split('(')[0])
  const tools = $derived(
    [...project.rules.reduce((m, r) => m.set(toolOf(r), (m.get(toolOf(r)) ?? 0) + 1), new Map<string, number>())]
      .sort((a, b) => b[1] - a[1])
      .map(([tool, count]) => `${tool} ${count}`)
      .join(' · '),
  )
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
        <ul>
          {#each project.rules as rule (rule)}
            <li>
              <button class="rule" class:whole={whole === rule} aria-expanded={whole === rule} onclick={() => (whole = whole === rule ? null : rule)}>
                <span class="tool">{toolOf(rule)}</span>
                <code>{ruleLabel(rule)}</code>
              </button>
              <button class="remove" aria-label={t('common.remove')} title={t('common.remove')} onclick={() => act(() => removeRule(project.path, rule))}>×</button>
            </li>
          {/each}
        </ul>
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

  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
    border-top: 1px solid var(--border);
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-right: 8px;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li:hover {
    background: var(--surface-2);
  }

  .rule {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
    padding: 0 0 0 14px;
    text-align: left;
  }

  .tool {
    flex: none;
    width: 38px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 12px/1.5 var(--mono);
    color: var(--text-2);
  }

  .whole code {
    padding: 8px 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .remove {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 16px;
  }

  .remove:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }
</style>
