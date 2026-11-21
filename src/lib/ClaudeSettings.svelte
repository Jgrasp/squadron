<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import { account } from './catalogs.svelte'
  import { applySetting, claudeConfig, labelOf, loadSettings, loadValues } from './claudeConfig.svelte'
  import type { Setting } from './commands'
  import Select from './Select.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'

  let { path = $bindable() }: { path: string } = $props()

  loadSettings()
  loadValues()

  const SECTIONS: [Key, string[]][] = [
    ['settings.claudeModelSection', ['outputStyle', 'language', 'thinking', 'switchModelsOnFlag']],
    ['settings.claudeContextSection', ['autoCompact', 'checkpoints', 'recap', 'promptSuggestionEnabled']],
    ['settings.claudeWorkflowsSection', ['workflows', 'workflowSizeGuideline', 'workflowKeywordTriggerEnabled', 'useAutoModeDuringPlan']],
    ['settings.claudeSessionsSection', ['remoteControl', 'worktreeBaseRef']],
  ]

  let version = $state('')
  let unanswered = $state('')
  let saving = $state<Record<string, boolean>>({})
  let errors = $state<Record<string, string>>({})

  const sections = $derived(
    SECTIONS.map(([title, keys]) => ({ title, settings: keys.flatMap((key) => claudeConfig.settings?.find((s) => s.key === key) ?? []) })).filter((s) => s.settings.length),
  )
  const signedIn = $derived([account.value?.email, account.value?.organization, account.value?.subscriptionType].filter(Boolean).join(' · '))

  async function check() {
    version = ''
    unanswered = ''
    try {
      version = await invoke<string>('claude_version', { path })
    } catch (e) {
      unanswered = errorText(e) || t('settings.claudeNoAnswer')
    }
  }
  check()

  async function browse() {
    const chosen = await open({ directory: false })
    if (typeof chosen === 'string') path = chosen
  }

  async function set(key: string, value: string) {
    value = value.trim()
    const before = claudeConfig.chosen[key]
    if (!value || value === before) return
    claudeConfig.chosen[key] = value
    saving[key] = true
    errors[key] = ''
    try {
      await applySetting(key, value)
    } catch (e) {
      errors[key] = errorText(e)
      if (before === undefined) delete claudeConfig.chosen[key]
      else claudeConfig.chosen[key] = before
    } finally {
      saving[key] = false
      await loadValues()
    }
  }

  const boolean = (setting: Setting) => setting.values.length === 2 && setting.values.includes('true') && setting.values.includes('false')
  const options = (setting: Setting) => [
    ...(claudeConfig.chosen[setting.key] === undefined ? [{ value: '', label: t('settings.claudeConfigPick'), disabled: true }] : []),
    ...setting.values.map((value) => ({ value, label: value })),
  ]
</script>

<section class="claude">
  <h3>{t('settings.executable')}</h3>
  <div class="options">
    <div class="option">
      <span class="label"><strong>{t('settings.claudeProgram')}</strong><small>{t('settings.claudeIntro')}</small></span>
      <input class="field mono" aria-label={t('settings.executable')} bind:value={path} spellcheck="false" />
      <button class="btn btn-ghost" onclick={browse}>{t('common.browse')}</button>
    </div>
    <div class="option sub">
      <span class="label">
        {#if unanswered}
          <small class="bad" role="alert">{unanswered}</small>
        {:else}
          <small>{#if version}<span class="good">{version}</span>{/if}{#if version && signedIn} · {/if}{#if signedIn}{t('settings.signedInAs', { account: signedIn })}{/if}</small>
        {/if}
      </span>
      <button class="btn btn-ghost" onclick={check}>{t('settings.check')}</button>
    </div>
  </div>

  {#if claudeConfig.loading}
    <p class="intro">{t('settings.claudeConfigLoading')}</p>
  {:else if claudeConfig.failed}
    <p class="bad" role="alert">{errorText(claudeConfig.failed)}</p>
  {/if}
  {#each sections as section (section.title)}
    <h3>{t(section.title)}</h3>
    <div class="options">
      {#each section.settings as setting (setting.key)}
        {@const chosen = claudeConfig.chosen[setting.key]}
        <div class="option">
          <span class="label">
            <strong>{labelOf(setting.key) || setting.key}</strong>
            {#if errors[setting.key]}<small class="bad" role="alert">{errors[setting.key]}</small>{:else}<small><code>{setting.key}</code></small>{/if}
          </span>
          {#if !setting.values.length}
            <input
              class="field"
              aria-label={labelOf(setting.key) || setting.key}
              placeholder={t('settings.claudeLanguageDefault')}
              value={chosen ?? ''}
              disabled={saving[setting.key]}
              onchange={(e) => set(setting.key, e.currentTarget.value)}
            />
          {:else if boolean(setting) && chosen !== undefined}
            <input
              type="checkbox"
              class="switch"
              aria-label={labelOf(setting.key) || setting.key}
              checked={chosen === 'true'}
              disabled={saving[setting.key]}
              onchange={(e) => set(setting.key, String(e.currentTarget.checked))}
            />
          {:else}
            <Select label={labelOf(setting.key) || setting.key} value={chosen ?? ''} options={options(setting)} onpick={(value) => set(setting.key, value)} />
          {/if}
        </div>
      {/each}
    </div>
  {/each}

  <div class="elsewhere">
    <div>
      <span>{t('settings.claudeElsewhereDefaults')}</span>
      <button class="btn btn-ghost" onclick={() => ((ui.settingsProject = '*'), (ui.settings = 'projects'))}>{t('settings.claudeElsewhereDefaultsGo')}</button>
    </div>
    <div>
      <span>{t('settings.claudeElsewherePhone')}</span>
      <button class="btn btn-ghost" onclick={() => (ui.settings = 'notifications')}>{t('settings.tabNotifications')}</button>
    </div>
  </div>
</section>

<style>
  .claude {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 6px 0 -2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

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

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .option strong {
    font-size: 13px;
    font-weight: 600;
  }

  .option small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .option small.bad {
    color: var(--err-text);
  }

  .option code {
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .good {
    font: 12px var(--mono);
    color: var(--ok-text);
  }

  .option .field,
  .option :global(.select) {
    flex: none;
    width: 260px;
  }

  .option .field {
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .option .field.mono {
    font-family: var(--mono);
  }

  .option .btn,
  .elsewhere .btn {
    flex: none;
    height: 32px;
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
    opacity: 0.5;
  }

  .switch:focus-visible {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .elsewhere {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .elsewhere div {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .elsewhere span {
    flex: 1;
  }
</style>
