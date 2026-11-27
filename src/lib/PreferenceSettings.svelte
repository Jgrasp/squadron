<script lang="ts">
  import { chime } from './chime'
  import { ALWAYS } from './groups'
  import { errorText, LANGUAGES, systemLocale, t, type Key } from './i18n/index.svelte'
  import { saveSettings, settings, type ChipsMotion, type Config, type ProjectsEffect, type TabEffect, type TabOrder, type TabRing } from './settings.svelte'
  import AgentChips from './AgentChips.svelte'
  import { ago } from './format'
  import type { Chip } from './newAgentDraft'

  const EFFECTS: [TabEffect, Key, Key, number][] = [
    ['snake', 'settings.effectSnake', 'settings.effectSnakeHint', 2.4],
    ['pulse', 'settings.effectPulse', 'settings.effectPulseHint', 2],
  ]
  const RINGS: [TabRing, Key, Key][] = [
    ['off', 'settings.ringOff', 'settings.ringOffHint'],
    ['shake', 'settings.ringShake', 'settings.ringShakeHint'],
    ['sound', 'settings.ringSound', 'settings.ringSoundHint'],
  ]
  const ORDERS: [TabOrder, Key, Key][] = [
    ['manual', 'settings.orderManual', 'settings.orderManualHint'],
    ['name', 'common.name', 'settings.orderNameHint'],
    ['activity', 'settings.orderActivity', 'settings.orderActivityHint'],
  ]
  const HOURS = [1, 4, 12, 24, ALWAYS, 0]
  const PROJECT_EFFECTS: [ProjectsEffect, Key, Key, number][] = [
    ['snake', 'settings.effectSnake', 'settings.projectsSnakeHint', 2.4],
    ['bar', 'settings.effectBar', 'settings.projectsBarHint', 1.6],
    ['pulse', 'settings.effectPulse', 'settings.projectsPulseHint', 2],
  ]
  const MOTIONS: [ChipsMotion, Key, Key][] = [
    ['off', 'settings.chipsOff', 'settings.chipsOffHint'],
    ['fade', 'settings.chipsFade', 'settings.chipsFadeHint'],
    ['down', 'settings.chipsDown', 'settings.chipsDownHint'],
    ['up', 'settings.chipsUp', 'settings.chipsUpHint'],
  ]
  const EVERY = [2, 3, 5] as const
  const SCOPES: [Config['chips_scope'], Key, Key][] = [
    ['all', 'settings.chipsScopeAll', 'settings.chipsScopeAllHint'],
    ['selected', 'settings.chipsScopeSelected', 'settings.chipsScopeSelectedHint'],
  ]
  const AGO: [Config['ago_style'], Key][] = [
    ['days', 'settings.agoDays'],
    ['weeks', 'settings.agoWeeks'],
    ['date', 'settings.agoDate'],
  ]
  const SAMPLE: Chip[] = [
    { extension: 'jira', key: 'LR-295', title: 'Suspension flow' },
    { extension: 'deploy', key: 'Preprod', title: 'Deployed to preprod', tone: 'ok', logo: false },
    { extension: 'gitlab', key: '#1284', title: 'Suspend button stays disabled' },
    { extension: 'linear', key: 'LIN-77', title: 'Design review' },
    { extension: 'deploy', key: 'Staging', title: 'Deploying to staging', tone: 'info', logo: false },
  ]
  const WIDTHS: [number, Key][] = [
    [640, 'settings.widthNarrow'],
    [740, 'settings.widthNormal'],
    [960, 'settings.widthWide'],
    [0, 'settings.widthFull'],
  ]
  const STEPS: [Config['steps'], Key, Key][] = [
    ['grouped', 'settings.stepsGrouped', 'settings.stepsGroupedHint'],
    ['detailed', 'settings.stepsDetailed', 'settings.stepsDetailedHint'],
  ]
  const THINKING: [Config['thinking'], Key][] = [
    ['hidden', 'settings.thinkingHidden'],
    ['duration', 'settings.thinkingDuration'],
    ['summary', 'settings.thinkingSummary'],
    ['shown', 'settings.thinkingShown'],
  ]
  const KEEP: [number, Key][] = [
    [0, 'settings.archiveNever'],
    [7, 'settings.archiveWeek'],
    [30, 'settings.archiveMonth'],
    [90, 'settings.archiveDays'],
    [365, 'settings.archiveYear'],
  ]

  let error = $state('')
  const custom = $derived(!WIDTHS.some(([px]) => px === settings.conversation_width))
  let dragged = $state(0)
  const width = $derived(dragged || settings.conversation_width || 1100)
  const effect = $derived(EFFECTS.find(([id]) => id === settings.tab_effect))
  const ring = $derived(RINGS.find(([id]) => id === settings.tab_ring))
  const order = $derived(ORDERS.find(([id]) => id === settings.tab_order))
  const projects = $derived(PROJECT_EFFECTS.find(([id]) => id === settings.projects_effect))
  const steps = $derived(STEPS.find(([id]) => id === settings.steps))
  const motion = $derived(MOTIONS.find(([id]) => id === settings.chips_motion))
  const sampleAgo = $derived(settings.ago_style && ago(Date.now() - 50 * 86_400_000, Date.now()))
  const scope = $derived(SCOPES.find(([id]) => id === settings.chips_scope))

  async function change(patch: Partial<Config>) {
    error = ''
    try {
      await saveSettings({ ...$state.snapshot(settings), ...patch })
    } catch (e) {
      error = errorText(e)
    }
  }

  const hours = (count: number) => (count === ALWAYS ? t('settings.tabHoursAlways') : count ? t('settings.tabHoursOption', { count }) : t('settings.tabHoursPinned'))
  const clamp = (px: number) => Math.min(1600, Math.max(520, px || 740))
</script>

<h3>{t('settings.tabBar')}</h3>
<div class="options">
  <div class="option">
    <span class="label"><strong>{t('settings.workingTabEffect')}</strong><small>{effect && t(effect[2], { seconds: effect[3] })}</small></span>
    <span class="sample effect-{settings.tab_effect}"><span class="dot" aria-hidden="true"></span>Orbit</span>
    <div class="segments" role="group" aria-label={t('settings.workingTabEffect')}>
      {#each EFFECTS as [id, title] (id)}
        <button aria-pressed={settings.tab_effect === id} onclick={() => change({ tab_effect: id })}>{t(title)}</button>
      {/each}
    </div>
  </div>
  <div class="option">
    <span class="label"><strong>{t('settings.waitingTab')}</strong><small>{ring && t(ring[2], { seconds: 2.4 })}</small></span>
    <span class="sample" class:ring={settings.tab_ring !== 'off'}><span class="dot" aria-hidden="true"></span>Orbit<span class="badge">1</span></span>
    <div class="segments" role="group" aria-label={t('settings.waitingTab')}>
      {#each RINGS as [id, title] (id)}
        <button aria-pressed={settings.tab_ring === id} onclick={() => (change({ tab_ring: id }), id === 'sound' && chime())}>{t(title)}</button>
      {/each}
    </div>
  </div>
  <label class="option">
    <span class="label"><strong>{t('settings.tabOrder')}</strong><small>{order && t(order[2])}</small></span>
    <select class="field" value={settings.tab_order} onchange={(e) => change({ tab_order: e.currentTarget.value as TabOrder })}>
      {#each ORDERS as [id, title] (id)}<option value={id}>{t(title)}</option>{/each}
    </select>
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.tabHours')}</strong><small>{t('settings.tabHoursHint')}</small></span>
    <select class="field" value={settings.tab_hours} onchange={(e) => change({ tab_hours: Number(e.currentTarget.value) })}>
      {#each HOURS as count (count)}<option value={count}>{hours(count)}</option>{/each}
    </select>
  </label>
</div>

<h3>{t('settings.projectsPanel')}</h3>
<div class="options">
  <div class="option">
    <span class="label"><strong>{t('settings.projectsEffect')}</strong><small>{projects && t(projects[2], { seconds: projects[3] })}</small></span>
    <span class="sample card stripe effect-{settings.projects_effect}">Orbit</span>
    <div class="segments" role="group" aria-label={t('settings.projectsEffect')}>
      {#each PROJECT_EFFECTS as [id, title] (id)}
        <button aria-pressed={settings.projects_effect === id} onclick={() => change({ projects_effect: id })}>{t(title)}</button>
      {/each}
    </div>
  </div>
</div>

<h3>{t('settings.agentList')}</h3>
<div class="options">
  <div class="option">
    <span class="label"><strong>{t('settings.chipsMotion')}</strong><small>{motion && t(motion[2])}</small></span>
    <span class="sample card chips"><AgentChips chips={SAMPLE} turns onmore={() => {}} /></span>
    <div class="segments" role="group" aria-label={t('settings.chipsMotion')}>
      {#each MOTIONS as [id, title] (id)}
        <button aria-pressed={settings.chips_motion === id} onclick={() => change({ chips_motion: id })}>{t(title)}</button>
      {/each}
    </div>
  </div>
  {#if settings.chips_motion !== 'off'}
    <div class="option sub">
      <span class="label"><strong>{t('settings.chipsEvery')}</strong></span>
      <div class="segments" role="group" aria-label={t('settings.chipsEvery')}>
        {#each EVERY as seconds (seconds)}
          <button aria-pressed={settings.chips_every === seconds} onclick={() => change({ chips_every: seconds })}>{t('settings.chipsSeconds', { seconds })}</button>
        {/each}
      </div>
    </div>
    <div class="option sub">
      <span class="label"><strong>{t('settings.chipsScope')}</strong><small>{scope && t(scope[2])}</small></span>
      <div class="segments" role="group" aria-label={t('settings.chipsScope')}>
        {#each SCOPES as [id, title] (id)}
          <button aria-pressed={settings.chips_scope === id} onclick={() => change({ chips_scope: id })}>{t(title)}</button>
        {/each}
      </div>
    </div>
  {/if}
  <div class="option">
    <span class="label"><strong>{t('settings.agoStyle')}</strong><small>{t('settings.agoStyleHint')}</small></span>
    <div class="segments" role="group" aria-label={t('settings.agoStyle')}>
      {#each AGO as [id, title] (id)}
        <button aria-pressed={settings.ago_style === id} onclick={() => change({ ago_style: id })}>{t(title)}</button>
      {/each}
    </div>
  </div>
  <div class="option sub">
    <div class="sample-list">
      <div class="sample-agent"><span class="ok" aria-hidden="true">✓</span><strong>LR-293 {t('settings.agoSampleName')}</strong><small>{t('shell.doneAgo', { ago: sampleAgo })}</small></div>
    </div>
  </div>
</div>

<h3>{t('settings.conversation')}</h3>
<div class="options">
  <div class="option">
    <span class="label"><strong>{t('settings.conversationWidth')}</strong><small>{t('settings.conversationWidthHint')}</small></span>
    <div class="segments" role="group" aria-label={t('settings.conversationWidth')}>
      {#each WIDTHS as [px, title] (px)}
        <button aria-pressed={!custom && settings.conversation_width === px} onclick={() => change({ conversation_width: px })}>{t(title)}</button>
      {/each}
      <button aria-pressed={custom} onclick={() => custom || change({ conversation_width: 1100 })}>{t('settings.widthCustom')}</button>
    </div>
  </div>
  {#if custom}
    <div class="option sub">
      <span class="label"><strong>{t('settings.widthCustomLabel')}</strong></span>
      <input class="slider" type="range" min="520" max="1600" step="20" aria-label={t('settings.widthCustomLabel')} value={width} oninput={(e) => (dragged = +e.currentTarget.value)} onchange={(e) => change({ conversation_width: +e.currentTarget.value }).then(() => (dragged = 0))} />
      <span class="field px"><input type="number" min="520" max="1600" step="20" aria-label={t('settings.widthCustomLabel')} value={width} onchange={(e) => change({ conversation_width: clamp(+e.currentTarget.value) })} />px</span>
    </div>
  {/if}
  <div class="option">
    <span class="label"><strong>{t('settings.agentSteps')}</strong><small>{steps && t(steps[2])}</small></span>
    <div class="segments" role="group" aria-label={t('settings.agentSteps')}>
      {#each STEPS as [id, title] (id)}
        <button aria-pressed={settings.steps === id} onclick={() => change({ steps: id })}>{t(title)}</button>
      {/each}
    </div>
  </div>
  <label class="option">
    <span class="label"><strong>{t('settings.thinking')}</strong><small>{t('settings.thinkingHint')}</small></span>
    <select class="field" value={settings.thinking} onchange={(e) => change({ thinking: e.currentTarget.value as Config['thinking'] })}>
      {#each THINKING as [id, title] (id)}<option value={id}>{t(title)}</option>{/each}
    </select>
  </label>
</div>

<h3>{t('settings.groupSquadron')}</h3>
<div class="options">
  <label class="option">
    <span class="label"><strong>{t('settings.usage')}</strong><small>{t('settings.usageHint')}</small></span>
    <input class="switch" type="checkbox" role="switch" checked={settings.usage} onchange={(e) => change({ usage: e.currentTarget.checked })} />
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.artifactsExpire')}</strong><small>{t('settings.artifactsExpireHint')}</small></span>
    <select class="field" value={settings.artifacts_after_days} onchange={(e) => change({ artifacts_after_days: Number(e.currentTarget.value) })}>
      {#each KEEP as [days, text] (days)}<option value={days}>{t(text, { count: days })}</option>{/each}
    </select>
  </label>
  <label class="option">
    <span class="label"><strong>{t('settings.language')}</strong></span>
    <select class="field" value={settings.language} onchange={(e) => change({ language: e.currentTarget.value as Config['language'] })}>
      <option value="system">{t('settings.languageSystem', { language: LANGUAGES.find(([id]) => id === systemLocale())?.[1] ?? '' })}</option>
      {#each LANGUAGES as [id, name] (id)}<option value={id}>{name}</option>{/each}
    </select>
  </label>
</div>

{#if error}<p class="bad" role="alert">{error}</p>{/if}

<style>
  h3 {
    margin: 6px 0 -8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  h3:first-child {
    margin-top: 0;
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

  .option > .field {
    flex: none;
    width: 260px;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .segments {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-2);
    font-size: 12px;
    white-space: nowrap;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .sample {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 14px 0 12px;
    border-radius: 4px;
    background: var(--bg-tabs);
    color: var(--text-strong);
    font-weight: 600;
  }

  .sample.card {
    width: 150px;
    height: 36px;
    padding-left: 16px;
    border-radius: 6px;
    background-color: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .sample.card::after {
    border-radius: inherit;
  }

  .option.sub:has(.sample-list) {
    min-height: 0;
    padding: 14px;
  }

  .sample-list {
    width: 304px;
    max-width: 100%;
    box-sizing: border-box;
    padding: 12px;
    border-radius: 6px;
    background: var(--bg-sidebar);
  }

  .sample-agent {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 8px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .sample-agent .ok {
    color: var(--ok-text);
  }

  .sample-agent small {
    grid-column: 2;
  }

  .sample.chips {
    justify-content: flex-end;
    padding-right: 12px;
    background-color: var(--bg-sidebar);
  }

  .sample .dot {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--accent);
  }

  .sample .badge {
    height: 20px;
    min-width: 20px;
    padding: 0 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11.5px;
    font-weight: 700;
  }

  .slider {
    flex: 1;
    accent-color: var(--accent);
  }

  .px {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    padding: 0 10px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .px input {
    width: 56px;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    outline: none;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
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
</style>
