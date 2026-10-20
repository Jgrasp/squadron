<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import Dialog from './Dialog.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import { home, projects, removeRule } from './projects.svelte'
  import { ruleLabel } from './rules'
  import { saveSettings, settings } from './settings.svelte'

  export type Tab = 'defaults' | 'rules' | 'claude'

  let { tab = $bindable(), onclose }: { tab: Tab; onclose: () => void } = $props()

  const tabs: [Tab, string][] = [
    ['defaults', 'Agent defaults'],
    ['rules', 'Permission rules'],
    ['claude', 'Claude Code'],
  ]

  let defaults = $state({ ...settings.defaults })
  let claude = $state(settings.claude)
  let version = $state('')
  let error = $state('')
  let saved = $state(false)

  const withRules = $derived(projects.list.filter((p) => p.rules.length))

  async function save() {
    error = ''
    try {
      await saveSettings({ claude, defaults: $state.snapshot(defaults) })
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

<Dialog title="Settings" {onclose} width={640}>
  <div class="tabs" role="tablist" aria-label="Settings sections">
    {#each tabs as [id, label] (id)}
      <button role="tab" aria-selected={tab === id} onclick={() => (tab = id)}>{label}</button>
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

  {#if tab !== 'rules'}
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
