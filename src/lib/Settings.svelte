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
  import { saveSettings, settings, type App, type TabEffect } from './settings.svelte'
  import { ACTIONS, binding, combo, label, usable, type Action } from './shortcuts'

  type Tab = SettingsTab

  let { onclose }: { onclose: () => void } = $props()

  const tab = $derived(ui.settings ?? 'defaults')

  const groups: [string, [Tab, string][]][] = [
    [
      'Projects',
      [
        ['projects', 'Projects'],
        ['archived', 'Archived agents'],
      ],
    ],
    [
      'Agents',
      [
        ['defaults', 'Agent defaults'],
        ['rules', 'Permission rules'],
      ],
    ],
    [
      'Tools',
      [
        ['claude', 'Claude Code'],
        ['editors', 'Editors'],
        ['hosting', 'Code hosting'],
      ],
    ],
    [
      'Interface',
      [
        ['preferences', 'Preferences'],
        ['shortcuts', 'Shortcuts'],
        ['layout', 'Layout'],
      ],
    ],
    ['Squadron', [['about', 'About']]],
  ]
  const current = $derived(groups.flatMap(([, items]) => items).find(([id]) => id === tab)?.[1] ?? '')
  const effects: [TabEffect, string, string][] = [
    ['snake', 'Snake', 'A light trail runs around the tab, one lap every 2.4 s'],
    ['pulse', 'Pulse', 'The tab background breathes, one cycle every 2 s'],
  ]

  let defaults = $state({ ...settings.defaults })
  let claude = $state(settings.claude)
  let effect = $state<TabEffect>(settings.tab_effect)
  let pilot = $state(settings.pilot)
  let pilotAll = $state(settings.pilot_all)
  let archiveAfter = $state(settings.archive_after_days)
  let steps = $state(settings.steps)
  let thinking = $state(settings.thinking)
  const STEPS: [typeof steps, string, string][] = [
    ['grouped', 'Grouped', 'One line per reply for the thinking, the notes and the commands; the answer stands on its own'],
    ['detailed', 'In full', 'Every thought, note and command in the flow of the conversation'],
  ]
  const THINKING: [typeof thinking, string][] = [
    ['hidden', 'Hidden'],
    ['duration', 'How long'],
    ['summary', 'With its summary'],
  ]
  const AGES: [number, string][] = [
    [0, 'Never'],
    [1, 'After a day'],
    [3, 'After 3 days'],
    [7, 'After a week'],
    [14, 'After two weeks'],
    [30, 'After a month'],
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
    if (other) conflict = `${label(value)} is already used by “${other[1]}”.`
    else if (!usable(value)) conflict = `${label(value)} would fire while you type. Add Ctrl or Alt, or use a function key.`
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
      await saveSettings({ claude, defaults: $state.snapshot(defaults), tab_effect: effect, keys: $state.snapshot(keys), editors: $state.snapshot(editors), pilot, pilot_all: pilotAll, archive_after_days: archiveAfter, steps, thinking })
      saved = true
      setTimeout(() => (saved = false), 1500)
    } catch (e) {
      error = String(e)
    }
  }

  async function check() {
    version = ''
    error = ''
    try {
      version = await invoke<string>('claude_version', { path: claude })
    } catch (e) {
      error = String(e) || 'Claude Code did not answer.'
    }
  }

  async function browse() {
    const chosen = await open({ directory: false })
    if (typeof chosen === 'string') claude = chosen
  }
</script>

<svelte:window onkeydowncapture={record} />

<Dialog title="Settings" {onclose} full>
  <div class="frame">
  <div class="side" role="tablist" aria-label="Settings sections" aria-orientation="vertical">
    {#each groups as [title, items] (title)}
      <span class="group">{title}</span>
      {#each items as [id, label] (id)}
        <button role="tab" aria-selected={tab === id} onclick={() => (ui.settings = id)}>{label}</button>
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
      <p class="intro">Used to fill in the New agent dialog.</p>
      <OptionsFields bind:options={defaults} />
    {:else if tab === 'rules'}
      <p class="intro">Added with Always on a permission card. Agents of the project no longer ask for what a rule allows.</p>
      {#each withRules as project (project.path)}
        <section>
          <h2>{project.name} <span>{home(project.path)}</span></h2>
          <ul>
            {#each project.rules as rule (rule)}
              <li>
                <code>{ruleLabel(rule)}</code>
                <span class="tool">{rule.startsWith('Bash(') ? 'Bash' : rule.split('(')[0]}</span>
                <button class="remove" onclick={() => removeRule(project.path, rule)}>Remove</button>
              </li>
            {/each}
          </ul>
        </section>
      {:else}
        <p class="empty">No rules yet. Choosing Always on a permission card adds one here.</p>
      {/each}
    {:else if tab === 'editors'}
      <p class="intro">
        Apps behind “Open in …” in the editor and diff panels. In a command, <code>{'{project}'}</code> and <code>{'{file}'}</code> become their
        paths. The last app you used becomes the default.
      </p>
      {#each editors as app, i (i)}
        <div class="app">
          <input class="field" bind:value={app.name} placeholder="Name" aria-label="Name" />
          <input class="field mono" bind:value={app.command} placeholder={'zed {project} {file}'} aria-label="Command" spellcheck="false" />
          <button class="remove" onclick={() => editors.splice(i, 1)}>Remove</button>
        </div>
      {:else}
        <p class="empty">
          {found.length
            ? `None set: Squadron offers the editors found on this computer (${found.map((a) => a.name).join(', ')}).`
            : 'None set and none found: “Open in external editor” uses the system default app.'}
        </p>
      {/each}
      <div class="row">
        <button class="btn btn-secondary" onclick={() => editors.push({ name: '', command: '' })}>Add an app</button>
        <button class="btn btn-secondary" disabled={!found.some((f) => !editors.some((e) => e.name === f.name))} onclick={addFound}>
          Add the apps found
        </button>
      </div>
    {:else if tab === 'layout'}
      <div class="intro row-intro">
        <p class="intro">
          Drag an item to move it, click its eye to hide it, or focus it and use Alt and the arrows. Hidden items still answer to their shortcut.
        </p>
        <button class="btn btn-secondary" onclick={() => ((ui.settings = null), (ui.customizing = true))}>Customize in place</button>
      </div>
      {#each Object.entries(SLOTS) as [slot, def] (slot)}
        <section>
          <h2>{def.title} <span class="hint-text">{def.hint}</span></h2>
          <SlotPreview slot={slot as Slot} />
          <button class="link" onclick={() => reset(slot as Slot)}>Back to the default</button>
        </section>
      {/each}
    {:else if tab === 'shortcuts'}
      <p class="intro">Click a shortcut, then press the new keys. They do not fire while you type in the terminal or the editor.</p>
      <ul class="keys">
        {#each ACTIONS as [id, name] (id)}
          <li>
            <span class="what">{name}</span>
            {#if recording === id}
              <span class="listening" role="status">Press the new keys…</span>
              <button class="remove" onclick={() => ((recording = null), (conflict = ''))}>Cancel</button>
            {:else}
              <button class="combo" class:custom={!!keys[id]} title="Change" onclick={() => ((recording = id), (conflict = ''))}>
                <kbd>{label(binding(keys, id))}</kbd>
              </button>
              {#if keys[id]}
                <button class="remove" onclick={() => delete keys[id]}>Default</button>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
      {#if conflict}<p class="bad" role="alert">{conflict}</p>{/if}
    {:else if tab === 'preferences'}
      <p class="intro">How a project tab shows that one of its agents is working. The animation stops when the system asks for reduced motion.</p>
      <div class="effects" role="group" aria-label="Working tab effect">
        {#each effects as [id, title, hint] (id)}
          <button aria-pressed={effect === id} onclick={() => (effect = id)}>
            <span class="sample effect-{id}">
              <span class="dot" aria-hidden="true"></span>Orbit
            </span>
            <strong>{title}</strong>
            <span class="hint">{hint}</span>
          </button>
        {/each}
      </div>
      <h2>Conversation</h2>
      <span class="label">Agent steps</span>
      <div class="effects" role="group" aria-label="Agent steps">
        {#each STEPS as [id, title, detail] (id)}
          <button aria-pressed={steps === id} onclick={() => (steps = id)}>
            <strong>{title}</strong>
            <span class="hint">{detail}</span>
          </button>
        {/each}
      </div>
      <span class="label">Thinking</span>
      <div class="effects three" role="group" aria-label="Thinking">
        {#each THINKING as [id, title] (id)}
          <button aria-pressed={thinking === id} onclick={() => (thinking = id)}><strong>{title}</strong></button>
        {/each}
      </div>
      <h2>Agents and Squadron</h2>
      <label for="archive-after">Archive idle agents</label>
      <select id="archive-after" class="field" bind:value={archiveAfter}>
        {#each AGES as [days, text] (days)}<option value={days}>{text}</option>{/each}
      </select>
      <p class="intro">An agent that is done, interrupted or stopped on an error for that long goes to the archived agents. Working agents and agents waiting for you stay.</p>
      <label class="check">
        <input type="checkbox" bind:checked={pilot} />
        <span>
          <strong>Agents can drive Squadron</strong>
          <small>They see the projects and agents, start new agents, add and move cards, and change the layout. They never answer or approve anything in your place. Read only actions run freely; the others ask you first, like any tool.</small>
        </span>
      </label>
      <label class="check" class:disabled={!pilot}>
        <input type="checkbox" bind:checked={pilotAll} disabled={!pilot} />
        <span>
          <strong>On every project</strong>
          <small>Otherwise an agent only starts agents and edits cards in its own project.</small>
        </span>
      </label>
      <p class="intro">Applies to agents started or reloaded after saving.</p>
    {:else}
      <p class="intro">Squadron starts every agent with this executable.</p>
      <label for="claude-path">Executable</label>
      <div class="row">
        <input id="claude-path" class="field mono" bind:value={claude} spellcheck="false" />
        <button class="btn btn-secondary" onclick={browse}>Browse…</button>
        <button class="btn btn-secondary" onclick={check}>Check</button>
      </div>
      {#if version}<p class="good">{version}</p>{/if}
      {#if account.value}
        <p class="intro">
          Signed in as {[account.value.email, account.value.organization, account.value.subscriptionType].filter(Boolean).join(' · ')}
        </p>
      {/if}
    {/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
  </div>

  {#if !['rules', 'layout', 'projects', 'archived', 'about', 'hosting'].includes(tab)}
    <footer>
      {#if saved}<span class="good">Saved</span>{/if}
      <button class="btn btn-secondary" onclick={onclose}>Close</button>
      <button class="btn btn-primary" onclick={save}>Save</button>
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
