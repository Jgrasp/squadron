<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import Dialog from './Dialog.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import { home, projects, removeRule } from './projects.svelte'
  import { ruleLabel } from './rules'
  import AboutSquadron from './AboutSquadron.svelte'
  import CodeHosting from './CodeHosting.svelte'
  import { ui, type SettingsTab } from './actions.svelte'
  import ArchivedAgents from './ArchivedAgents.svelte'
  import { account } from './catalogs.svelte'
  import ProjectSettings from './ProjectSettings.svelte'
  import { reset } from './layout-slots.svelte'
  import { detect } from './OpenWith.svelte'
  import SlotPreview from './SlotPreview.svelte'
  import { SLOTS, type Slot } from './slots'
  import { saveSettings, settings, type App, type TabEffect, type TabRing } from './settings.svelte'
  import { chime } from './chime'
  import { tokens } from './format'
  import { errorText, LANGUAGES, list, systemLocale, t, type Key } from './i18n/index.svelte'
  import Trans from './Trans.svelte'
  import { ACTIONS, binding, combo, label, usable, type Action } from './shortcuts'

  type Tab = SettingsTab

  let { onclose }: { onclose: () => void } = $props()

  const tab = $derived(ui.settings ?? 'defaults')

  const groups: [Key, [Tab, Key][]][] = [
    [
      'settings.projects',
      [
        ['projects', 'settings.projects'],
        ['archived', 'settings.tabArchived'],
      ],
    ],
    [
      'settings.agents',
      [
        ['defaults', 'settings.tabDefaults'],
        ['rules', 'settings.tabRules'],
      ],
    ],
    [
      'settings.groupTools',
      [
        ['claude', 'settings.tabClaude'],
        ['editors', 'settings.tabEditors'],
        ['hosting', 'settings.tabHosting'],
      ],
    ],
    [
      'settings.groupInterface',
      [
        ['preferences', 'settings.tabPreferences'],
        ['shortcuts', 'settings.tabShortcuts'],
        ['layout', 'settings.tabLayout'],
      ],
    ],
    ['settings.groupSquadron', [['about', 'settings.tabAbout']]],
  ]
  const current = $derived.by(() => {
    const key = groups.flatMap(([, items]) => items).find(([id]) => id === tab)?.[1]
    return key ? t(key) : ''
  })
  const effects: [TabEffect, Key, Key, number][] = [
    ['snake', 'settings.effectSnake', 'settings.effectSnakeHint', 2.4],
    ['pulse', 'settings.effectPulse', 'settings.effectPulseHint', 2],
  ]

  let defaults = $state({ ...settings.defaults })
  let claude = $state(settings.claude)
  let effect = $state<TabEffect>(settings.tab_effect)
  let ring = $state<TabRing>(settings.tab_ring)
  const RINGS: [TabRing, Key, Key][] = [
    ['off', 'settings.ringOff', 'settings.ringOffHint'],
    ['shake', 'settings.ringShake', 'settings.ringShakeHint'],
    ['sound', 'settings.ringSound', 'settings.ringSoundHint'],
  ]
  let pilot = $state(settings.pilot)
  let pilotAll = $state(settings.pilot_all)
  let archiveAfter = $state(settings.archive_after_days)
  let compactWindow = $state(settings.compact_window)
  const WINDOWS = [0, 600_000, 400_000, 200_000]
  let steps = $state(settings.steps)
  let thinking = $state(settings.thinking)
  let language = $state(settings.language)
  const STEPS: [typeof steps, Key, Key][] = [
    ['grouped', 'settings.stepsGrouped', 'settings.stepsGroupedHint'],
    ['detailed', 'settings.stepsDetailed', 'settings.stepsDetailedHint'],
  ]
  const THINKING: [typeof thinking, Key][] = [
    ['hidden', 'settings.thinkingHidden'],
    ['duration', 'settings.thinkingDuration'],
    ['summary', 'settings.thinkingSummary'],
    ['shown', 'settings.thinkingShown'],
  ]
  const AGES: [number, Key][] = [
    [0, 'settings.archiveNever'],
    [1, 'settings.archiveDay'],
    [3, 'settings.archiveDays'],
    [7, 'settings.archiveWeek'],
    [14, 'settings.archiveTwoWeeks'],
    [30, 'settings.archiveMonth'],
  ]
  let keys = $state<Record<string, string>>({ ...settings.keys })
  let editors = $state<App[]>(settings.editors.map((e) => ({ ...e })))
  let found = $state<App[]>([])
  detect().then((list) => (found = list))

  function addFound() {
    for (const app of found) if (!editors.some((e) => e.name === app.name)) editors.push({ ...app })
  }
  let recording = $state<Action | null>(null)
  let conflict = $state('')

  function record(event: KeyboardEvent) {
    if (!recording) return
    event.preventDefault()
    event.stopPropagation()
    const value = combo(event)
    if (!value) return
    const other = ACTIONS.find(([id]) => id !== recording && binding(keys, id) === value)
    if (other) conflict = t('settings.conflictUsed', { keys: label(value), action: t(other[1]) })
    else if (!usable(value)) conflict = t('settings.conflictTyping', { keys: label(value) })
    else {
      if (value === ACTIONS.find(([id]) => id === recording)![2]) delete keys[recording]
      else keys[recording] = value
      recording = null
      conflict = ''
    }
  }
  let version = $state('')
  let error = $state('')
  let saved = $state(false)

  const withRules = $derived(projects.list.filter((p) => p.rules.length))

  async function save() {
    error = ''
    try {
      await saveSettings({ claude, defaults: $state.snapshot(defaults), tab_effect: effect, tab_ring: ring, tab_order: settings.tab_order, keys: $state.snapshot(keys), editors: $state.snapshot(editors), pilot, pilot_all: pilotAll, archive_after_days: archiveAfter, compact_window: compactWindow, steps, thinking, language })
      saved = true
      setTimeout(() => (saved = false), 1500)
    } catch (e) {
      error = errorText(e)
    }
  }

  async function check() {
    version = ''
    error = ''
    try {
      version = await invoke<string>('claude_version', { path: claude })
    } catch (e) {
      error = errorText(e) || t('settings.claudeNoAnswer')
    }
  }

  async function browse() {
    const chosen = await open({ directory: false })
    if (typeof chosen === 'string') claude = chosen
  }
</script>

<svelte:window onkeydowncapture={record} />

<Dialog title={t('settings.title')} {onclose} full>
  <div class="frame">
  <div class="side" role="tablist" aria-label={t('settings.sections')} aria-orientation="vertical">
    {#each groups as [title, items] (title)}
      <span class="group">{t(title)}</span>
      {#each items as [id, label] (id)}
        <button role="tab" aria-selected={tab === id} onclick={() => (ui.settings = id)}>{t(label)}</button>
      {/each}
    {/each}
  </div>

  <div class="content">
  <h2 class="heading">{current}</h2>
  <div class="body">
    {#if tab === 'projects'}
      <ProjectSettings />
    {:else if tab === 'archived'}
      <ArchivedAgents />
    {:else if tab === 'about'}
      <AboutSquadron />
    {:else if tab === 'hosting'}
      <CodeHosting />
    {:else if tab === 'defaults'}
      <p class="intro">{t('settings.defaultsIntro')}</p>
      <OptionsFields bind:options={defaults} />
    {:else if tab === 'rules'}
      <p class="intro">{t('settings.rulesIntro')}</p>
      {#each withRules as project (project.path)}
        <section>
          <h2>{project.name} <span>{home(project.path)}</span></h2>
          <ul>
            {#each project.rules as rule (rule)}
              <li>
                <code>{ruleLabel(rule)}</code>
                <span class="tool">{rule.startsWith('Bash(') ? 'Bash' : rule.split('(')[0]}</span>
                <button class="remove" onclick={() => removeRule(project.path, rule)}>{t('common.remove')}</button>
              </li>
            {/each}
          </ul>
        </section>
      {:else}
        <p class="empty">{t('settings.rulesEmpty')}</p>
      {/each}
    {:else if tab === 'editors'}
      <p class="intro">
        <Trans k="settings.editorsIntro">{#snippet project()}<code>{'{project}'}</code>{/snippet}{#snippet file()}<code>{'{file}'}</code>{/snippet}</Trans>
      </p>
      {#each editors as app, i (i)}
        <div class="app">
          <input class="field" bind:value={app.name} placeholder={t('common.name')} aria-label={t('common.name')} />
          <input class="field mono" bind:value={app.command} placeholder={'zed {project} {file}'} aria-label={t('settings.editorCommand')} spellcheck="false" />
          <button class="remove" onclick={() => editors.splice(i, 1)}>{t('common.remove')}</button>
        </div>
      {:else}
        <p class="empty">
          {found.length ? t('settings.editorsNoneFound', { apps: list(found.map((a) => a.name)) }) : t('settings.editorsNone')}
        </p>
      {/each}
      <div class="row">
        <button class="btn btn-secondary" onclick={() => editors.push({ name: '', command: '' })}>{t('settings.addApp')}</button>
        <button class="btn btn-secondary" disabled={!found.some((f) => !editors.some((e) => e.name === f.name))} onclick={addFound}>
          {t('settings.addFoundApps')}
        </button>
      </div>
    {:else if tab === 'layout'}
      <div class="intro row-intro">
        <p class="intro">{t('settings.layoutIntro')}</p>
        <button class="btn btn-secondary" onclick={() => ((ui.settings = null), (ui.customizing = true))}>{t('settings.customizeInPlace')}</button>
      </div>
      {#each Object.entries(SLOTS) as [slot, def] (slot)}
        <section>
          <h2>{t(def.title)} <span class="hint-text">{t(def.hint)}</span></h2>
          <SlotPreview slot={slot as Slot} />
          <button class="link" onclick={() => reset(slot as Slot)}>{t('settings.backToDefault')}</button>
        </section>
      {/each}
    {:else if tab === 'shortcuts'}
      <p class="intro">{t('settings.shortcutsIntro')}</p>
      <ul class="keys">
        {#each ACTIONS as [id, name] (id)}
          <li>
            <span class="what">{t(name)}</span>
            {#if recording === id}
              <span class="listening" role="status">{t('settings.pressKeys')}</span>
              <button class="remove" onclick={() => ((recording = null), (conflict = ''))}>{t('common.cancel')}</button>
            {:else}
              <button class="combo" class:custom={!!keys[id]} title={t('settings.change')} onclick={() => ((recording = id), (conflict = ''))}>
                <kbd>{label(binding(keys, id))}</kbd>
              </button>
              {#if keys[id]}
                <button class="remove" onclick={() => delete keys[id]}>{t('common.default')}</button>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
      {#if conflict}<p class="bad" role="alert">{conflict}</p>{/if}
    {:else if tab === 'preferences'}
      <p class="intro">{t('settings.tabEffectIntro')}</p>
      <div class="effects" role="group" aria-label={t('settings.workingTabEffect')}>
        {#each effects as [id, title, hint, seconds] (id)}
          <button aria-pressed={effect === id} onclick={() => (effect = id)}>
            <span class="sample effect-{id}">
              <span class="dot" aria-hidden="true"></span>Orbit
            </span>
            <strong>{t(title)}</strong>
            <span class="hint">{t(hint, { seconds })}</span>
          </button>
        {/each}
      </div>
      <span class="label">{t('settings.waitingTab')}</span>
      <div class="effects three" role="group" aria-label={t('settings.waitingTab')}>
        {#each RINGS as [id, title, hint] (id)}
          <button aria-pressed={ring === id} onclick={() => ((ring = id), id === 'sound' && chime())}>
            <span class="sample" class:ring={id !== 'off'}>
              <span class="dot" aria-hidden="true"></span>Orbit<span class="badge">1</span>
            </span>
            <strong>{t(title)}</strong>
            <span class="hint">{t(hint, { seconds: 2.4 })}</span>
          </button>
        {/each}
      </div>
      <h2>{t('settings.conversation')}</h2>
      <span class="label">{t('settings.agentSteps')}</span>
      <div class="effects" role="group" aria-label={t('settings.agentSteps')}>
        {#each STEPS as [id, title, detail] (id)}
          <button aria-pressed={steps === id} onclick={() => (steps = id)}>
            <strong>{t(title)}</strong>
            <span class="hint">{t(detail)}</span>
          </button>
        {/each}
      </div>
      <span class="label">{t('settings.thinking')}</span>
      <div class="effects" role="group" aria-label={t('settings.thinking')}>
        {#each THINKING as [id, title] (id)}
          <button aria-pressed={thinking === id} onclick={() => (thinking = id)}><strong>{t(title)}</strong></button>
        {/each}
      </div>
      <h2>{t('settings.agentsAndSquadron')}</h2>
      <label for="archive-after">{t('settings.archiveIdle')}</label>
      <select id="archive-after" class="field" bind:value={archiveAfter}>
        {#each AGES as [days, text] (days)}<option value={days}>{t(text, { count: days })}</option>{/each}
      </select>
      <p class="intro">{t('settings.archiveIdleHint')}</p>
      <label for="compact-window">{t('settings.compactWindow')}</label>
      <select id="compact-window" class="field" bind:value={compactWindow}>
        {#each WINDOWS as size (size)}<option value={size}>{size ? t('settings.compactAround', { tokens: tokens(size) }) : t('settings.compactDefault')}</option>{/each}
      </select>
      <p class="intro">{t('settings.compactWindowHint')}</p>
      <label class="check">
        <input type="checkbox" bind:checked={pilot} />
        <span>
          <strong>{t('settings.pilot')}</strong>
          <small>{t('settings.pilotHint')}</small>
        </span>
      </label>
      <label class="check" class:disabled={!pilot}>
        <input type="checkbox" bind:checked={pilotAll} disabled={!pilot} />
        <span>
          <strong>{t('settings.pilotAll')}</strong>
          <small>{t('settings.pilotAllHint')}</small>
        </span>
      </label>
      <p class="intro">{t('settings.pilotApplies')}</p>
      <h2 id="language-heading">{t('settings.language')}</h2>
      <select id="language" class="field" aria-labelledby="language-heading" bind:value={language}>
        <option value="system">{t('settings.languageSystem', { language: LANGUAGES.find(([id]) => id === systemLocale())?.[1] ?? '' })}</option>
        {#each LANGUAGES as [id, name] (id)}<option value={id}>{name}</option>{/each}
      </select>
    {:else}
      <p class="intro">{t('settings.claudeIntro')}</p>
      <label for="claude-path">{t('settings.executable')}</label>
      <div class="row">
        <input id="claude-path" class="field mono" bind:value={claude} spellcheck="false" />
        <button class="btn btn-secondary" onclick={browse}>{t('common.browse')}</button>
        <button class="btn btn-secondary" onclick={check}>{t('settings.check')}</button>
      </div>
      {#if version}<p class="good">{version}</p>{/if}
      {#if account.value}
        <p class="intro">
          {t('settings.signedInAs', { account: [account.value.email, account.value.organization, account.value.subscriptionType].filter(Boolean).join(' · ') })}
        </p>
      {/if}
    {/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
  </div>

  {#if !['rules', 'layout', 'projects', 'archived', 'about', 'hosting'].includes(tab)}
    <footer>
      {#if saved}<span class="good">{t('common.saved')}</span>{/if}
      <button class="btn btn-secondary" onclick={onclose}>{t('common.close')}</button>
      <button class="btn btn-primary" onclick={save}>{t('common.save')}</button>
    </footer>
  {/if}
  </div>
  </div>
</Dialog>

<style>
  .frame {
    flex: 1;
    min-height: 0;
    display: flex;
    border-top: 1px solid var(--border);
    margin-top: 14px;
  }

  .side {
    flex: none;
    width: 230px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 12px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .group {
    padding: 14px 10px 6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .group:first-child {
    padding-top: 2px;
  }

  .side button {
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
    color: var(--text-muted);
  }

  .side button:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .side button[aria-selected='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .heading {
    margin: 0;
    padding: 20px 32px 4px;
    font-size: 16px;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
    padding: 14px 32px 24px;
  }

  .body > :global(*) {
    max-width: 860px;
  }

  .intro,
  .empty {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  h2 span {
    margin-left: 6px;
    font: 400 12px var(--mono);
    color: var(--text-dim);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  li code {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 12.5px;
  }

  .tool {
    font-size: 12px;
    color: var(--text-dim);
  }

  .remove {
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .remove:hover {
    background: #3a2420;
    color: #f8d5cc;
  }

  label {
    font-weight: 600;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row .btn {
    height: 42px;
  }

  .mono {
    font: 13px var(--mono);
  }

  .good {
    margin: 0;
    font: 12.5px var(--mono);
    color: var(--done-text);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  .app {
    display: grid;
    grid-template-columns: 150px 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .app .field {
    height: 36px;
  }

  .intro code {
    font-size: 12px;
    color: var(--text-2);
  }

  .keys {
    gap: 2px;
  }

  h2 .hint-text {
    font: 400 12.5px var(--sans);
  }

  .check {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .check input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .check span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check strong {
    font-weight: 600;
  }

  .check small {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-dim);
  }

  .check.disabled {
    opacity: 0.5;
  }

  .row-intro {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .row-intro .btn {
    flex: none;
    height: 34px;
  }






  .remove:disabled {
    opacity: 0.3;
  }

  .link {
    align-self: flex-start;
    font-size: 12.5px;
    color: var(--accent-link);
  }

  .keys li {
    padding: 6px 6px 6px 12px;
  }

  .what {
    flex: 1;
  }

  .combo {
    min-width: 96px;
    height: 30px;
    padding: 0 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12.5px var(--mono);
  }

  .combo:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .combo.custom {
    box-shadow: inset 0 0 0 1px var(--accent);
    color: var(--accent-soft);
  }

  .listening {
    min-width: 96px;
    height: 30px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text-2);
    font-size: 12.5px;
  }

  .effects {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .effects.three {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .label {
    display: block;
    margin: 16px 0 8px;
    font-weight: 600;
  }

  .effects button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 14px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    text-align: left;
  }

  .effects button[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
  }

  .effects strong {
    font-weight: 600;
  }

  .effects .hint {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .sample {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin-bottom: 4px;
    padding: 0 14px 0 12px;
    border-radius: 4px;
    background: var(--bg-tabs);
    color: var(--text-strong);
    font-weight: 600;
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

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }
</style>
