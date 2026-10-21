<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import Dialog from './Dialog.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import { home, projects, removeRule } from './projects.svelte'
  import { ruleLabel } from './rules'
  import { ui, type SettingsTab } from './actions.svelte'
  import { reset } from './layout-slots.svelte'
  import { detect } from './OpenWith.svelte'
  import SlotPreview from './SlotPreview.svelte'
  import { SLOTS, type Slot } from './slots'
  import { saveSettings, settings, type App, type TabEffect } from './settings.svelte'
  import { ACTIONS, binding, combo, label, usable, type Action } from './shortcuts'

  type Tab = SettingsTab

  let { onclose }: { onclose: () => void } = $props()

  const tab = $derived(ui.settings ?? 'defaults')

  const tabs: [Tab, string][] = [
    ['defaults', 'Agent defaults'],
    ['rules', 'Permission rules'],
    ['claude', 'Claude Code'],
    ['editors', 'Editors'],
    ['preferences', 'Preferences'],
    ['shortcuts', 'Shortcuts'],
    ['layout', 'Layout'],
  ]
  const effects: [TabEffect, string, string][] = [
    ['snake', 'Snake', 'A light trail runs around the tab, one lap every 2.4 s'],
    ['pulse', 'Pulse', 'The tab background breathes, one cycle every 2 s'],
  ]

  let defaults = $state({ ...settings.defaults })
  let claude = $state(settings.claude)
  let effect = $state<TabEffect>(settings.tab_effect)
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
      await saveSettings({ claude, defaults: $state.snapshot(defaults), tab_effect: effect, keys: $state.snapshot(keys), editors: $state.snapshot(editors) })
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

<Dialog title="Settings" {onclose} width={800}>
  <div class="tabs" role="tablist" aria-label="Settings sections">
    {#each tabs as [id, label] (id)}
      <button role="tab" aria-selected={tab === id} onclick={() => (ui.settings = id)}>{label}</button>
    {/each}
  </div>

  <div class="body">
    {#if tab === 'defaults'}
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
              <span class="dot" aria-hidden="true"></span>BMC
            </span>
            <strong>{title}</strong>
            <span class="hint">{hint}</span>
          </button>
        {/each}
      </div>
    {:else}
      <p class="intro">Squadron starts every agent with this executable.</p>
      <label for="claude-path">Executable</label>
      <div class="row">
        <input id="claude-path" class="field mono" bind:value={claude} spellcheck="false" />
        <button class="btn btn-secondary" onclick={browse}>Browse…</button>
        <button class="btn btn-secondary" onclick={check}>Check</button>
      </div>
      {#if version}<p class="good">{version}</p>{/if}
    {/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
  </div>

  {#if tab !== 'rules' && tab !== 'layout'}
    <footer>
      {#if saved}<span class="good">Saved</span>{/if}
      <button class="btn btn-secondary" onclick={onclose}>Close</button>
      <button class="btn btn-primary" onclick={save}>Save</button>
    </footer>
  {/if}
</Dialog>

<style>
  .tabs {
    display: flex;
    gap: 4px;
    padding: 10px 22px 0;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    height: 36px;
    padding: 0 12px;
    white-space: nowrap;
    border-bottom: 2px solid transparent;
    color: var(--text-muted);
  }

  .tabs button[aria-selected='true'] {
    border-color: var(--accent);
    color: var(--text-strong);
    font-weight: 600;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 280px;
    max-height: 60vh;
    overflow-y: auto;
    padding: 18px 22px 20px;
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
