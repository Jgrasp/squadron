<script lang="ts">
  import type { Snippet } from 'svelte'
  import { models } from './catalogs.svelte'
  import { advisorChoices, advisorIdle, advisorLabel, advising, effortLabel, modelLabel } from './format'
  import { t, type Key } from './i18n/index.svelte'
  import OptionRow from './OptionRow.svelte'
  import type { Options } from './stream'

  let { options = $bindable(), before, picker }: { options: Options; before?: Snippet; picker?: Snippet } = $props()

  const EFFORTS: Options['effort'][] = ['low', 'medium', 'high', 'xhigh', 'max']
  const MODES: Options['mode'][] = ['default', 'acceptEdits', 'plan', 'auto']
  const TITLES: Record<Options['mode'], Key> = {
    default: 'settings.modeDefault',
    acceptEdits: 'settings.modeAcceptEdits',
    plan: 'settings.modePlan',
    auto: 'settings.modeAuto',
  }
  const HINTS: Record<Options['mode'], Key> = {
    default: 'settings.modeDefaultHint',
    acceptEdits: 'settings.modeAcceptEditsHint',
    plan: 'settings.modePlanHint',
    auto: 'settings.modeAutoHint',
  }

  const catalog = $derived(models().filter((m) => m.value !== 'default'))
  const modelChoices = $derived(
    catalog.length ? catalog.map((m) => ({ value: m.value, label: modelLabel(m.value), detail: m.description })) : ['opus', 'sonnet', 'haiku'].map((value) => ({ value, label: modelLabel(value) })),
  )
  const levels = $derived(catalog.find((m) => m.value === options.model)?.supportedEffortLevels)
  const effortChoices = $derived(EFFORTS.map((value) => ({ value, label: effortLabel(value), off: !!levels && !levels.includes(value) })))
  const noEffort = $derived(levels?.length === 0)
  let openKey = $state<'model' | 'effort' | null>(null)
  let expanded = $state(false)

  function pickModel(value: string) {
    options.model = value
    const supported = catalog.find((m) => m.value === value)?.supportedEffortLevels
    if (supported?.length && !supported.includes(options.effort)) options.effort = supported[supported.length - 1] as Options['effort']
  }

  const modeChoices = $derived(MODES.map((value) => ({ value, label: t(TITLES[value]), detail: t(HINTS[value]), tone: `tone-${value}` })))
</script>

{#snippet field(key: 'model' | 'effort', label: string, value: string, choices: { value: string; label: string; detail?: string; off?: boolean }[], selected: string, onpick: (v: string) => void)}
  <div class="pick" class:open={openKey === key}>
    <button type="button" class="field-trigger" aria-haspopup="listbox" aria-expanded={openKey === key} onclick={() => (openKey = openKey === key ? null : key)}>
      <dt>{label}</dt>
      <dd>{value}<svg class="chev" width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 15l6-6 6 6" /></svg></dd>
    </button>
    {#if openKey === key}
      <div class="option-menu" role="listbox" aria-label={label}>
        {#each choices as choice (choice.value)}
          <button type="button" role="option" aria-selected={choice.value === selected} disabled={choice.off} onclick={() => ((openKey = null), onpick(choice.value))}>
            <strong>{choice.label}</strong>
            {#if choice.detail}<span>{choice.detail}</span>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<svelte:window onpointerdown={(e) => openKey && !(e.target as Element).closest('.pick') && (openKey = null)} />

<div class="summary">
  <dl>
    {@render field('model', t('settings.model'), modelLabel(options.model), modelChoices, options.model, pickModel)}
    {@render field('effort', t('settings.effort'), noEffort ? '—' : effortLabel(options.effort), effortChoices, options.effort, (v) => (options.effort = v as Options['effort']))}
    {#if advising(options)}
      <div class="advisor" class:idle={advisorIdle(options)}><dt>{t('settings.advisor')}</dt><dd>{modelLabel(options.advisor!)}</dd></div>
    {/if}
  </dl>
  <button type="button" class="btn btn-secondary launch" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h9M17 7h3M4 17h3M11 17h9" /><circle cx="15" cy="7" r="2" /><circle cx="9" cy="17" r="2" /></svg>
    {t('board.launchSettings')}
  </button>
</div>

{#if expanded}
  <div class="settings">
    {@render before?.()}
    <div class="rows">
      <OptionRow label={t('settings.model')} value={modelLabel(options.model)} choices={modelChoices} selected={options.model} onpick={pickModel}>
        {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="14" rx="3" /><path d="M9 1v4M15 1v4M9 19v4M15 19v4M1 9h4M1 15h4M19 9h4M19 15h4" /></svg>{/snippet}
      </OptionRow>
      <OptionRow label={t('settings.effort')} value={effortLabel(options.effort)} choices={effortChoices} selected={options.effort} onpick={(v) => (options.effort = v as Options['effort'])}>
        {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M5 20v-4M10 20v-8M15 20v-12M20 20V4" /></svg>{/snippet}
      </OptionRow>
      <OptionRow
        label={t('settings.advisor')}
        value={advisorLabel(options.advisor)}
        tone={advising(options) ? 'tone-advisor' : ''}
        choices={advisorChoices(options.model)}
        selected={options.advisor ?? ''}
        onpick={(v) => (options.advisor = v || undefined)}
      >
        {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M12 3l2.4 5.6L20 11l-5.6 2.4L12 19l-2.4-5.6L4 11l5.6-2.4z" /></svg>{/snippet}
      </OptionRow>
      <OptionRow label={t('settings.permissionMode')} value={t(TITLES[options.mode])} tone="tone-{options.mode}" choices={modeChoices} selected={options.mode} onpick={(v) => (options.mode = v as Options['mode'])}>
        {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z" /></svg>{/snippet}
      </OptionRow>
      {@render picker?.()}
    </div>
  </div>
{/if}

<style>
  .summary {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  dl {
    display: flex;
    gap: 4px;
    margin: 0;
  }

  .pick {
    position: relative;
  }

  .field-trigger {
    display: block;
    min-width: 78px;
    padding: 4px 8px;
    border-radius: 5px;
    text-align: left;
  }

  .field-trigger:hover,
  .pick.open .field-trigger {
    background: var(--surface-3);
  }

  dd {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .chev {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    color: var(--text-dim);
    opacity: 0;
  }

  .field-trigger:hover .chev,
  .pick.open .chev {
    opacity: 1;
  }

  .pick .option-menu {
    left: 0;
    right: auto;
  }

  .advisor {
    padding: 4px 8px;
  }

  dt {
    font-size: 10px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  dd {
    margin: 1px 0 0;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .advisor dt {
    color: var(--advisor);
  }

  .advisor dd {
    color: var(--advisor-text);
  }

  .advisor.idle dd {
    color: var(--text-faint);
    text-decoration: line-through;
  }

  .launch {
    height: 32px;
    margin-left: auto;
    gap: 7px;
  }

  .launch svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .launch[aria-expanded='true'] {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .rows {
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: 6px;
    background: var(--surface-2);
  }
</style>
