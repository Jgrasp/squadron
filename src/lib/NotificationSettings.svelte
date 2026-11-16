<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { agents } from './agents.svelte'
  import { applySetting } from './claudeConfig.svelte'
  import Icon from './Icon.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { testNotification, type NotifyEvent } from './notify.svelte'
  import { saveSettings, settings, type Config } from './settings.svelte'

  const AGENT_ROWS: NotifyEvent[] = ['question', 'permission', 'plan', 'password', 'turn', 'goal', 'error']
  const TURNS = [0, 30, 60, 300]
  const FLAGS: [string, Key, Key][] = [
    ['inputNeededNotifEnabled', 'notify.pushInput', 'notify.pushInputHint'],
    ['agentPushNotifEnabled', 'notify.pushAgent', 'notify.pushAgentHint'],
    ['autoUploadSessions', 'notify.mirror', 'notify.mirrorHint'],
  ]

  let error = $state('')
  let flags = $state<Record<string, boolean> | null>(null)
  let saving = $state<Record<string, boolean>>({})
  let countdown = $state(0)

  invoke<Record<string, boolean>>('claude_flags')
    .then((found) => (flags = found))
    .catch((e) => (error = errorText(e)))

  const off = $derived(settings.notify_when === 'never')
  const connected = $derived(agents.list.filter((a) => a.remote?.state === 'connected').length)
  const on = (event: NotifyEvent) => !settings.notify_off.includes(event)
  const ringing = (event: NotifyEvent) => settings.notify_sound.includes(event)

  async function change(patch: Partial<Config>) {
    error = ''
    try {
      await saveSettings({ ...$state.snapshot(settings), ...patch })
    } catch (e) {
      error = errorText(e)
    }
  }

  const toggle = (list: string[], event: NotifyEvent, keep: boolean) => (keep ? [...list.filter((e) => e !== event), event] : list.filter((e) => e !== event))

  async function flag(key: string, value: boolean) {
    if (!flags) return
    error = ''
    saving[key] = true
    try {
      if (key === 'autoUploadSessions') await invoke('set_claude_flag', { key, value })
      else await applySetting(key, String(value))
      flags[key] = value
    } catch (e) {
      error = errorText(e)
    } finally {
      saving[key] = false
    }
  }

  function test() {
    if (countdown) return
    countdown = 5
    const tick = setInterval(() => {
      countdown -= 1
      if (countdown) return
      clearInterval(tick)
      testNotification().catch((e) => (error = errorText(e)))
    }, 1000)
  }

  const turnLabel = (secs: number) => (!secs ? t('notify.turnAny') : secs < 60 ? t('notify.turnSeconds', { count: secs }) : t('notify.turnMinutes', { count: secs / 60 }))
</script>

<p class="intro">{t('notify.intro')}</p>
<div class="options">
  <label class="option">
    <span class="label"><strong>{t('notify.when')}</strong><small>{t('notify.whenHint')}</small></span>
    <select class="field" value={settings.notify_when} onchange={(e) => change({ notify_when: e.currentTarget.value as Config['notify_when'] })}>
      <option value="background">{t('notify.whenBackground')}</option>
      <option value="always">{t('notify.whenAlways')}</option>
      <option value="never">{t('notify.whenNever')}</option>
    </select>
  </label>
</div>

<h3>{t('notify.agents')}</h3>
<div class="options" class:off>
  {#each AGENT_ROWS as event (event)}
    <div class="option">
      <label class="label" for="notify-{event}"><strong>{t(`notify.${event}Label` as Key)}</strong><small>{t(`notify.${event}Hint` as Key)}</small></label>
      <button
        class="sound"
        aria-pressed={ringing(event)}
        title={t('notify.chime')}
        aria-label={t('notify.chime')}
        disabled={off || !on(event)}
        onclick={() => change({ notify_sound: toggle(settings.notify_sound, event, !ringing(event)) })}
      >
        <Icon name="sound" size={15} />
      </button>
      <input
        id="notify-{event}"
        class="switch"
        type="checkbox"
        role="switch"
        checked={on(event)}
        disabled={off}
        onchange={(e) => change({ notify_off: toggle(settings.notify_off, event, !e.currentTarget.checked) })}
      />
    </div>
    {#if event === 'turn'}
      <label class="option sub" class:dim={!on('turn')}>
        <span class="label"><strong>{t('notify.turnMin')}</strong><small>{t('notify.turnMinHint')}</small></span>
        <select class="field" value={settings.notify_turn_secs} disabled={off || !on('turn')} onchange={(e) => change({ notify_turn_secs: Number(e.currentTarget.value) })}>
          {#each TURNS as secs (secs)}<option value={secs}>{turnLabel(secs)}</option>{/each}
        </select>
      </label>
    {/if}
  {/each}
</div>

<h3>{t('notify.codeHosting')}</h3>
<div class="options" class:off>
  <label class="option">
    <span class="label"><strong>{t('notify.runsLabel')}</strong><small>{t('notify.runsHint')}</small></span>
    <select class="field" value={settings.notify_runs} disabled={off} onchange={(e) => change({ notify_runs: e.currentTarget.value as Config['notify_runs'] })}>
      <option value="all">{t('notify.runsAll')}</option>
      <option value="failures">{t('notify.runsFailures')}</option>
      <option value="never">{t('notify.runsNever')}</option>
    </select>
  </label>
  <label class="option">
    <span class="label"><strong>{t('notify.pullsLabel')}</strong><small>{t('notify.pullsHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={on('pulls')} disabled={off} onchange={(e) => change({ notify_off: toggle(settings.notify_off, 'pulls', !e.currentTarget.checked) })} />
  </label>
</div>

<h3>{t('notify.squadron')}</h3>
<div class="options" class:off>
  {#each ['updates', 'extensions'] as const as event (event)}
    <label class="option">
      <span class="label"><strong>{t(`notify.${event}Label`)}</strong><small>{t(`notify.${event}Hint`)}</small></span>
      <input class="switch" type="checkbox" role="switch" checked={on(event)} disabled={off} onchange={(e) => change({ notify_off: toggle(settings.notify_off, event, !e.currentTarget.checked) })} />
    </label>
  {/each}
</div>

<div class="bar">
  {#if error}<p class="bad" role="alert">{error}</p>{:else}<p class="hint">{t('notify.testHint')}</p>{/if}
  <button class="btn btn-ghost" disabled={countdown > 0} onclick={test}>{countdown ? t('notify.testIn', { count: countdown }) : t('notify.test')}</button>
</div>

<h2>
  {t('notify.remote')}
  {#if connected}<span class="pill"><span class="dot" aria-hidden="true"></span>{t('notify.remoteConnected', { count: connected })}</span>{/if}
</h2>
<p class="intro">{t('notify.remoteIntro')}</p>
<div class="options">
  <label class="option">
    <span class="label"><strong>{t('notify.remoteNew')}</strong><small>{t('notify.remoteNewHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.remote_new_agents} onchange={(e) => change({ remote_new_agents: e.currentTarget.checked })} />
  </label>
  {#each FLAGS as [key, label, hint] (key)}
    <label class="option">
      <span class="label"><strong>{t(label)}</strong><small>{t(hint)}</small></span>
      <input class="switch" type="checkbox" role="switch" checked={flags?.[key] ?? false} disabled={!flags || saving[key]} onchange={(e) => flag(key, e.currentTarget.checked)} />
    </label>
  {/each}
  <label class="option">
    <span class="label"><strong>{t('notify.remoteDesktop')}</strong><small>{t('notify.remoteDesktopHint')}</small></span>
    <select class="field" value={settings.notify_remote} disabled={off} onchange={(e) => change({ notify_remote: e.currentTarget.value as Config['notify_remote'] })}>
      <option value="keep">{t('notify.remoteKeep')}</option>
      <option value="skip">{t('notify.remoteSkip')}</option>
      <option value="background">{t('notify.remoteBackground')}</option>
    </select>
  </label>
</div>

<style>
  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 12px 0 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  h3 {
    margin: 6px 0 -8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .pill {
    height: 20px;
    padding: 0 8px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 500;
  }

  .pill .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--remote-dot);
  }

  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .options.off {
    opacity: 0.5;
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
    width: 260px;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .sound {
    flex: none;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-faint);
  }

  .sound[aria-pressed='true'] {
    background: var(--surface-3);
    color: var(--text-2);
  }

  .sound:hover:not(:disabled) {
    color: var(--text);
  }

  .sound:disabled {
    opacity: 0.4;
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

  .bar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
  }

  .bar .btn {
    height: 32px;
  }

  .hint {
    flex: 1;
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  .bad {
    flex: 1;
    margin: 0;
    color: var(--err-text);
  }
</style>
