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

<p class="intro">{t('settings.agentsIntro', { name: project.name })}</p>

<div class="fields">
  <div class="group">
    <span class="label">{t('settings.model')}</span>
    <Select label={t('settings.model')} value={own.model ?? ''} options={modelOptions} marked={!!own.model} onpick={(value) => set('model', value)} />
  </div>
  <div class="group">
    <span class="label">{t('settings.effort')}</span>
    <Select label={t('settings.effort')} value={own.effort ?? ''} options={effortOptions} marked={!!own.effort} onpick={(value) => set('effort', value)} />
  </div>
  <div class="group">
    <span class="label">{t('settings.permissionMode')}</span>
    <Select label={t('settings.permissionMode')} value={own.mode ?? ''} options={modeOptions} marked={!!own.mode} onpick={(value) => set('mode', value)} />
  </div>
</div>

<p class="result">
  <span>{t('settings.agentsStart', { name: project.name })}</span>
  {#each summary as [text, marked], i (i)}
    {#if i}<span aria-hidden="true">·</span>{/if}
    <b class:marked>{text}</b>
  {/each}
</p>

<div class="actions">
  <button class="btn btn-secondary" disabled={!overridden} onclick={() => act(() => editProject(project.path, { defaults: {} }))}>{t('settings.agentsReset')}</button>
  <button class="btn btn-secondary" onclick={() => (ui.settings = 'defaults')}>{t('settings.agentsGlobal')}</button>
</div>

<style>
  .intro {
    margin: 0;
    max-width: 620px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .fields {
    align-self: stretch;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
    max-width: 760px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .label {
    font-weight: 600;
  }

  .result {
    align-self: stretch;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    max-width: 760px;
    margin: 0;
    padding: 12px 14px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-dim);
    font-size: 12.5px;
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

  .actions .btn {
    height: 34px;
  }
</style>
