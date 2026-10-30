<script lang="ts">
  import { applySetting, claudeConfig, labelOf, loadSettings, TERMINAL } from './claudeConfig.svelte'
  import { errorText, t } from './i18n/index.svelte'

  loadSettings()
  $effect(() => () => (claudeConfig.query = ''))

  let saving = $state<Record<string, boolean>>({})
  let replies = $state<Record<string, { text: string; bad: boolean }>>({})
  let typed = $state<Record<string, string>>({})

  const matching = $derived(
    (claudeConfig.settings ?? []).filter((s) => `${s.key} ${labelOf(s.key)}`.toLowerCase().includes(claudeConfig.query.trim().toLowerCase())),
  )
  const groups = $derived([
    { name: t('settings.claudeConfigAgents'), terminal: false, settings: matching.filter((s) => !TERMINAL.has(s.key)) },
    { name: t('settings.claudeConfigTerminal'), terminal: true, settings: matching.filter((s) => TERMINAL.has(s.key)) },
  ])

  async function set(key: string, value: string) {
    if (!value.trim()) return
    saving[key] = true
    try {
      replies[key] = { text: await applySetting(key, value.trim()), bad: false }
    } catch (e) {
      replies[key] = { text: errorText(e), bad: true }
    } finally {
      saving[key] = false
    }
  }
</script>

<section class="config">
  <h2>{t('settings.claudeConfig')}</h2>
  <p class="intro">{t('settings.claudeConfigIntro')}</p>
  <input class="field" type="search" placeholder={t('settings.claudeConfigSearch')} aria-label={t('settings.claudeConfigSearch')} bind:value={claudeConfig.query} />
  {#if claudeConfig.loading}
    <p class="intro">{t('settings.claudeConfigLoading')}</p>
  {:else if claudeConfig.failed}
    <p class="bad" role="alert">{errorText(claudeConfig.failed)}</p>
  {:else if claudeConfig.settings && !matching.length}
    <p class="intro">{t('settings.claudeConfigNone', { query: claudeConfig.query.trim() })}</p>
  {/if}
  {#each groups as group (group.name)}
    {#if group.settings.length}
      <div class="group" class:terminal={group.terminal}>
        <h3>{group.name}</h3>
        {#if group.terminal}<p class="intro">{t('settings.claudeConfigTerminalHint')}</p>{/if}
        {#each group.settings as setting (setting.key)}
          {@const chosen = claudeConfig.chosen[setting.key]}
          <div class="setting">
            <div class="name">
              <code>{setting.key}</code>
              {#if labelOf(setting.key)}<span>{labelOf(setting.key)}</span>{/if}
              {#if saving[setting.key]}
                <small>{t('settings.claudeConfigSaving')}</small>
              {:else if replies[setting.key]}
                <small class:bad={replies[setting.key].bad}>{replies[setting.key].text}</small>
              {/if}
            </div>
            {#if !setting.values.length}
              <form class="free" onsubmit={(e) => (e.preventDefault(), set(setting.key, typed[setting.key] ?? ''))}>
                <input class="field" aria-label={setting.key} placeholder={chosen} bind:value={typed[setting.key]} disabled={saving[setting.key]} />
                <button class="btn btn-secondary" disabled={saving[setting.key] || !typed[setting.key]?.trim()}>{t('settings.claudeConfigApply')}</button>
              </form>
            {:else if setting.values.length <= 4}
              <div class="choices" role="group" aria-label={setting.key}>
                {#each setting.values as value (value)}
                  <button aria-pressed={chosen === value} disabled={saving[setting.key]} onclick={() => set(setting.key, value)}>{value}</button>
                {/each}
              </div>
            {:else}
              <select class="field" aria-label={setting.key} value={chosen ?? ''} disabled={saving[setting.key]} onchange={(e) => set(setting.key, e.currentTarget.value)}>
                <option value="" disabled>{t('settings.claudeConfigPick')}</option>
                {#each setting.values as value (value)}<option {value}>{value}</option>{/each}
              </select>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/each}
</section>

<style>
  .config {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h2 {
    margin: 12px 0 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  h3 {
    margin: 14px 0 4px;
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

  .terminal .setting {
    opacity: 0.55;
  }

  .setting {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name code {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .name span {
    font-size: 13px;
  }

  .name small {
    font-size: 12px;
    color: var(--ok);
  }

  .name small.bad {
    color: var(--err-text);
  }

  .choices {
    flex: none;
    display: flex;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .choices button {
    height: 30px;
    padding: 0 12px;
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  .choices button[aria-pressed='true'] {
    border-radius: 4px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
    color: var(--text-strong);
  }

  select.field {
    flex: none;
    width: 220px;
    height: 32px;
    font: 12px var(--mono);
  }

  .free {
    flex: none;
    display: flex;
    gap: 8px;
  }

  .free .field {
    width: 180px;
    height: 32px;
  }

  .free .btn {
    height: 32px;
  }
</style>
