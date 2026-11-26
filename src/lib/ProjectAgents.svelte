<script lang="ts">
  import { AUTO, AUTO_PROMPT } from './autoLaunch'
  import { models } from './catalogs.svelte'
  import { families, modelName, versionsOf } from './commands'
  import { advisorChoices } from './format'
  import { t, type Key } from './i18n/index.svelte'
  import { editProject, projects, type Defaults, type Project } from './projects.svelte'
  import Select, { type Option } from './Select.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import type { Options } from './stream'

  let { project, act, pick }: { project?: Project; act: (work: () => Promise<unknown>) => Promise<void>; pick: (path: string) => void } = $props()

  const EFFORTS: [Options['effort'], Key][] = [
    ['auto', 'settings.auto'],
    ['low', 'settings.effortLow'],
    ['medium', 'settings.effortMedium'],
    ['high', 'settings.effortHigh'],
    ['xhigh', 'settings.effortXhigh'],
    ['max', 'settings.effortMax'],
  ]
  const MODES: [Options['mode'], Key, Key][] = [
    ['default', 'settings.modeDefault', 'settings.modeDefaultHint'],
    ['acceptEdits', 'settings.modeAcceptEdits', 'settings.modeAcceptEditsHint'],
    ['plan', 'settings.modePlan', 'settings.modePlanHint'],
    ['auto', 'settings.modeAuto', 'settings.modeAutoHint'],
  ]

  const catalog = $derived(models())
  const base = $derived(settings.defaults)
  const own = $derived<Defaults>(project ? (project.defaults ?? {}) : base)
  const effective = $derived({ model: own.model ?? base.model, effort: own.effort ?? base.effort, mode: own.mode ?? base.mode })
  const overridden = $derived(Object.keys(own).length)

  const capital = (word: string) => word[0].toUpperCase() + word.slice(1)
  const modelLabel = (value: string) => (value === AUTO ? t('settings.auto') : families(catalog).includes(value) ? capital(value) : modelName(value, catalog))
  const effortLabel = (value: string) => t(EFFORTS.find(([id]) => id === value)?.[1] ?? 'settings.effortHigh')
  const modeLabel = (value: string) => t(MODES.find(([id]) => id === value)?.[1] ?? 'settings.modeDefault')
  const inherit = (value: string): Option[] => (project ? [{ value: '', label: t('settings.inheritGlobal', { value }) }] : [])

  const modelOptions = $derived.by(() => {
    const list: Option[] = [...inherit(modelLabel(base.model)), { value: AUTO, label: t('settings.auto'), detail: t('settings.autoHint') }]
    for (const family of families(catalog)) {
      const versions = versionsOf(family, catalog)
      const latest = versions[0] ? modelName(versions[0].value, catalog) : ''
      list.push({ value: family, label: capital(family), detail: latest ? t('settings.latest', { model: latest }) : undefined })
      for (const version of versions) if (version.value !== family && version !== versions[0]) list.push({ value: version.value, label: version.displayName ?? version.value, detail: capital(family) })
    }
    if (own.model && !list.some((o) => o.value === own.model)) list.push({ value: own.model, label: modelLabel(own.model) })
    return list
  })

  const levels = $derived(catalog.find((m) => m.value === effective.model)?.supportedEffortLevels)
  const effortOptions = $derived<Option[]>([
    ...inherit(effortLabel(base.effort)),
    ...EFFORTS.map(([id, name]) => ({ value: id, label: t(name), detail: id === AUTO ? t('settings.autoHint') : undefined, disabled: id !== AUTO && !!levels && !levels.includes(id) && own.effort !== id })),
  ])
  const modeOptions = $derived<Option[]>([...inherit(modeLabel(base.mode)), ...MODES.map(([id, name, hint]) => ({ value: id, label: t(name), detail: t(hint) }))])
  const advisorOptions = $derived<Option[]>(advisorChoices(base.model).map((choice) => ({ value: choice.value, label: choice.label, detail: choice.detail, disabled: 'off' in choice && choice.off && base.advisor !== choice.value })))

  const summary = $derived([
    [modelLabel(effective.model), !!own.model],
    [effortLabel(effective.effort), !!own.effort],
    [modeLabel(effective.mode), !!own.mode],
  ] as const)

  const custom = $derived(
    projects.list
      .filter((p) => Object.keys(p.defaults ?? {}).length)
      .map((p) => ({ project: p, values: [p.defaults.model && modelLabel(p.defaults.model), p.defaults.effort && effortLabel(p.defaults.effort), p.defaults.mode && modeLabel(p.defaults.mode)].filter(Boolean).join(' · ') })),
  )

  function set(field: keyof Defaults, value: string) {
    const next: Record<string, string> = { ...$state.snapshot(own) }
    if (value) next[field] = value
    else delete next[field]
    if (field === 'model' && value === AUTO) next.advisor = AUTO
    act(() => (project ? editProject(project.path, { defaults: next as Defaults }) : saveSettings({ ...$state.snapshot(settings), defaults: next as Options })))
  }

  const changePrompt = (auto_prompt: string) => act(() => saveSettings({ ...$state.snapshot(settings), auto_prompt }))
</script>

<section>
  <h4>{t('settings.projectNewAgents')}</h4>
  <p class="intro">{project ? t('settings.agentsIntro', { name: project.name }) : t('settings.allProjectsIntro')}</p>
  <div class="options">
    <div class="option">
      <span class="label"><strong>{t('settings.model')}</strong></span>
      <Select label={t('settings.model')} value={own.model ?? ''} options={modelOptions} marked={!!project && !!own.model} onpick={(value) => set('model', value)} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('settings.effort')}</strong></span>
      <Select label={t('settings.effort')} value={own.effort ?? ''} options={effortOptions} marked={!!project && !!own.effort} onpick={(value) => set('effort', value)} />
    </div>
    {#if !project}
      <div class="option">
        <span class="label"><strong>{t('settings.advisor')}</strong><small>{t('settings.advisorHint')}</small></span>
        <Select label={t('settings.advisor')} value={own.advisor ?? ''} options={advisorOptions} onpick={(value) => set('advisor', value)} />
      </div>
    {/if}
    <div class="option">
      <span class="label"><strong>{t('settings.permissionMode')}</strong></span>
      <Select label={t('settings.permissionMode')} value={own.mode ?? ''} options={modeOptions} marked={!!project && !!own.mode} onpick={(value) => set('mode', value)} />
    </div>
    {#if project}
      <div class="option column result">
        <p>
          {t('settings.agentsStart', { name: project.name })}
          {#each summary as [text, marked], i (i)}
            {#if i}<span aria-hidden="true">·</span>{/if}
            <b class:marked>{text}</b>
          {/each}
        </p>
        <span class="actions">
          <button class="btn btn-ghost" disabled={!overridden} onclick={() => act(() => editProject(project.path, { defaults: {} }))}>{t('settings.agentsReset')}</button>
          <button class="btn btn-ghost" onclick={() => pick('')}>{t('settings.agentsGlobal')}</button>
        </span>
      </div>
    {:else}
      <div class="option column">
        <span class="label">
          <strong>{t('settings.autoPrompt')}</strong>
          <small>{t('settings.autoPromptIntro')}</small>
        </span>
        <textarea
          class="field prompt"
          rows="5"
          aria-label={t('settings.autoPrompt')}
          value={settings.auto_prompt || AUTO_PROMPT}
          onchange={(e) => changePrompt(e.currentTarget.value.trim() === AUTO_PROMPT ? '' : e.currentTarget.value.trim())}
        ></textarea>
        {#if settings.auto_prompt}
          <span><button class="btn btn-ghost" onclick={() => changePrompt('')}>{t('settings.autoPromptReset')}</button></span>
        {/if}
      </div>
      <div class="option column result">
        <p>{custom.length ? t('settings.allProjectsOwn') : t('settings.allProjectsNone')}</p>
        {#if custom.length}
          <span class="actions">
            {#each custom as { project: p, values } (p.path)}
              <button class="btn btn-ghost own" style:--project={p.color} onclick={() => pick(p.path)}>
                <span class="square" aria-hidden="true"></span>{p.name} · <b>{values}</b>
              </button>
            {/each}
          </span>
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .result p {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .result b {
    color: var(--text-2);
    font-weight: 600;
  }

  .result b.marked {
    color: var(--accent-soft);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .own {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .own .square {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    background: var(--project);
  }

  .own b {
    color: var(--accent-soft);
    font-weight: 600;
  }
</style>
