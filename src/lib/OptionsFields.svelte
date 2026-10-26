<script lang="ts">
  import { models } from './catalogs.svelte'
  import { families, familyOf, modelName, versionsOf } from './commands'
  import type { Options } from './stream'
  import { t, type Key } from './i18n/index.svelte'

  let { options = $bindable() }: { options: Options } = $props()

  const catalog = $derived(models())
  const family = $derived(familyOf(options.model, catalog))
  const versions = $derived(versionsOf(family, catalog))
  const levels = $derived(catalog.find((m) => m.value === options.model)?.supportedEffortLevels)
  const efforts: [Options['effort'], Key][] = [
    ['low', 'settings.effortLow'],
    ['medium', 'settings.effortMedium'],
    ['high', 'settings.effortHigh'],
    ['xhigh', 'settings.effortXhigh'],
    ['max', 'settings.effortMax'],
  ]
  const modes: { value: Options['mode']; title: Key; hint: Key }[] = [
    { value: 'default', title: 'settings.modeDefault', hint: 'settings.modeDefaultHint' },
    { value: 'acceptEdits', title: 'settings.modeAcceptEdits', hint: 'settings.modeAcceptEditsHint' },
    { value: 'plan', title: 'settings.modePlan', hint: 'settings.modePlanHint' },
    { value: 'auto', title: 'settings.modeAuto', hint: 'settings.modeAutoHint' },
  ]
</script>

<div class="pair">
  <div class="group">
    <span id="model" class="label">{t('settings.model')}</span>
    <div class="segments" role="group" aria-labelledby="model">
      {#each families(catalog) as model (model)}
        <button type="button" aria-pressed={family === model} onclick={() => (options.model = model)}>
          {model[0].toUpperCase() + model.slice(1)}
        </button>
      {/each}
    </div>
    {#if versions.length > 1}
      <select class="field version" aria-label={t('settings.version')} bind:value={options.model}>
        {#each versions as version, i (version.value)}
          <option value={version.value}>{i === 0 ? t('settings.latest', { model: modelName(version.value, catalog) }) : version.displayName}</option>
        {/each}
      </select>
    {/if}
  </div>
  <div class="group">
    <span id="effort" class="label">{t('settings.effort')}</span>
    <div class="segments small" role="group" aria-labelledby="effort">
      {#each efforts as [effort, name] (effort)}
        <button
          type="button"
          aria-pressed={options.effort === effort}
          disabled={!!levels && !levels.includes(effort)}
          onclick={() => (options.effort = effort)}
        >
          {t(name)}
        </button>
      {/each}
    </div>
  </div>
</div>

<div class="group">
  <span id="mode" class="label">{t('settings.permissionMode')}</span>
  <div class="modes" role="group" aria-labelledby="mode">
    {#each modes as mode (mode.value)}
      <button type="button" aria-pressed={options.mode === mode.value} onclick={() => (options.mode = mode.value)}>
        <strong>{t(mode.title)}</strong>
        <span>{t(mode.hint)}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .label {
    font-weight: 600;
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .segments {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    flex: 1;
    height: 34px;
    border-radius: 3px;
    color: var(--text-2);
  }

  .segments button:disabled {
    color: var(--text-faint);
    cursor: default;
  }

  .version {
    height: 40px;
  }

  .segments.small button {
    font-size: 12.5px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .modes button {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 11px 14px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    text-align: left;
  }

  .modes strong {
    font-weight: 600;
  }

  .modes span {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .modes button[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
    color: var(--text-strong);
  }

  .modes button[aria-pressed='true'] strong {
    font-weight: 700;
  }
</style>
