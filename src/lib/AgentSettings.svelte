<script lang="ts">
  import { tokens } from './format'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { COLD_TOKENS, saveSettings, settings, type Config } from './settings.svelte'

  const AGES: [number, Key][] = [
    [0, 'settings.archiveNever'],
    [1, 'settings.archiveDay'],
    [3, 'settings.archiveDays'],
    [7, 'settings.archiveWeek'],
    [14, 'settings.archiveTwoWeeks'],
    [30, 'settings.archiveMonth'],
  ]

  let error = $state('')

  async function change(patch: Partial<Config>) {
    error = ''
    try {
      await saveSettings({ ...$state.snapshot(settings), ...patch })
    } catch (e) {
      error = errorText(e)
    }
  }
</script>

<div class="options">
  <label class="option">
    <span class="label"><strong>{t('settings.archiveIdle')}</strong><small>{t('settings.archiveIdleHint')}</small></span>
    <select class="field" value={settings.archive_after_days} onchange={(e) => change({ archive_after_days: Number(e.currentTarget.value) })}>
      {#each AGES as [days, text] (days)}<option value={days}>{t(text, { count: days })}</option>{/each}
    </select>
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.coldTokens')}</strong><small>{t('settings.coldTokensHint')}</small></span>
    <select class="field" value={settings.cold_tokens} onchange={(e) => change({ cold_tokens: Number(e.currentTarget.value) })}>
      {#each COLD_TOKENS as size (size)}<option value={size}>{size ? t('settings.coldFrom', { tokens: tokens(size) }) : t('settings.coldNever')}</option>{/each}
    </select>
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.autoContinue')}</strong><small>{t('settings.autoContinueHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.auto_continue} onchange={(e) => change({ auto_continue: e.currentTarget.checked })} />
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.pilot')}</strong><small>{t('settings.pilotHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.pilot} onchange={(e) => change({ pilot: e.currentTarget.checked })} />
  </label>
  <label class="option sub" class:dim={!settings.pilot}>
    <span class="label"><strong>{t('settings.pilotAll')}</strong><small>{t('settings.pilotAllHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.pilot_all} disabled={!settings.pilot} onchange={(e) => change({ pilot_all: e.currentTarget.checked })} />
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.browser')} <em class="alpha">{t('settings.alpha')}</em></strong><small>{t('settings.browserHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.browser} onchange={(e) => change({ browser: e.currentTarget.checked })} />
  </label>
</div>
<p class="note">{t('settings.pilotApplies')}</p>

{#if error}<p class="bad" role="alert">{error}</p>{/if}

<style>
  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .option + .option {
    border-top: 1px solid var(--border);
  }

  .option.sub {
    min-height: 44px;
    padding-left: 40px;
  }

  .option.dim {
    opacity: 0.5;
  }

  .option .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    font-weight: 400;
  }

  .option strong {
    font-size: 13px;
    font-weight: 600;
  }

  .option small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .option .field {
    flex: none;
    width: 260px;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .alpha {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
    font-size: 10.5px;
    font-style: normal;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }

  .switch:disabled {
    cursor: default;
  }

  .switch:focus-visible {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .note {
    margin: -6px 0 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
