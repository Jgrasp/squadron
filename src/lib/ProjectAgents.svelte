<script lang="ts">
  import { ui } from './actions.svelte'
  import { models } from './catalogs.svelte'
  import { families, modelName, versionsOf } from './commands'
  import { t, type Key } from './i18n/index.svelte'
  import { editProject, type Defaults, type Project } from './projects.svelte'
  import Select, { type Option } from './Select.svelte'
  import { settings } from './settings.svelte'
  import type { Options } from './stream'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  const EFFORTS: [Options['effort'], Key][] = [
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
  const own = $derived<Defaults>(project.defaults ?? {})
  const base = $derived(settings.defaults)
  const effective = $derived({ model: own.model ?? base.model, effort: own.effort ?? base.effort, mode: own.mode ?? base.mode })
  const overridden = $derived(Object.keys(own).length)

  const capital = (word: string) => word[0].toUpperCase() + word.slice(1)
  const modelLabel = (value: string) => (families(catalog).includes(value) ? capital(value) : modelName(value, catalog))
  const effortLabel = (value: string) => t(EFFORTS.find(([id]) => id === value)?.[1] ?? 'settings.effortHigh')
  const modeLabel = (value: string) => t(MODES.find(([id]) => id === value)?.[1] ?? 'settings.modeDefault')
  const inherit = (value: string): Option => ({ value: '', label: t('settings.inheritGlobal', { value }) })

  const modelOptions = $derived.by(() => {
    const list: Option[] = [inherit(modelLabel(base.model))]
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
    inherit(effortLabel(base.effort)),
    ...EFFORTS.map(([id, name]) => ({ value: id, label: t(name), disabled: !!levels && !levels.includes(id) && own.effort !== id })),
  ])
  const modeOptions = $derived<Option[]>([inherit(modeLabel(base.mode)), ...MODES.map(([id, name, hint]) => ({ value: id, label: t(name), detail: t(hint) }))])

  const summary = $derived([
    [modelLabel(effective.model), !!own.model],
    [effortLabel(effective.effort), !!own.effort],
    [modeLabel(effective.mode), !!own.mode],
  ] as const)

  function set(field: keyof Defaults, value: string) {
    const next: Record<string, string> = { ...$state.snapshot(own) }
    if (value) next[field] = value
    else delete next[field]
    act(() => editProject(project.path, { defaults: next as Defaults }))
  }
</script>

<section>
  <h4>{t('settings.projectNewAgents')}</h4>
  <p class="intro">{t('settings.agentsIntro', { name: project.name })}</p>
  <div class="options">
    <div class="option">
      <span class="label"><strong>{t('settings.model')}</strong></span>
      <Select label={t('settings.model')} value={own.model ?? ''} options={modelOptions} marked={!!own.model} onpick={(value) => set('model', value)} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('settings.effort')}</strong></span>
      <Select label={t('settings.effort')} value={own.effort ?? ''} options={effortOptions} marked={!!own.effort} onpick={(value) => set('effort', value)} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('settings.permissionMode')}</strong></span>
      <Select label={t('settings.permissionMode')} value={own.mode ?? ''} options={modeOptions} marked={!!own.mode} onpick={(value) => set('mode', value)} />
    </div>
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
        <button class="btn btn-ghost" onclick={() => (ui.settings = 'defaults')}>{t('settings.agentsGlobal')}</button>
      </span>
    </div>
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
    gap: 8px;
  }
</style>
