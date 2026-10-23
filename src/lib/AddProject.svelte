<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { untrack } from 'svelte'
  import { unshelve } from './agents.svelte'
  import Dialog from './Dialog.svelte'
  import { add, home, inspect, projects, type Inspection } from './projects.svelte'

  let { path: initial, onclose }: { path: string; onclose: () => void } = $props()

  let path = $state(untrack(() => initial))
  let name = $state('')
  let info = $state<Inspection | null>(null)
  let error = $state('')
  let busy = $state(false)

  const problem = $derived.by(() => {
    if (!info || !path.trim()) return ''
    if (!info.exists) return "This folder does not exist or cannot be read."
    if (info.taken_by && info.taken_archived) return ''
    if (info.taken_by) return `This folder is already the ${info.taken_by} project.`
    return ''
  })
  const revive = $derived(!!info?.taken_by && info.taken_archived)
  const ready = $derived(!busy && (revive || (!!info?.exists && !info.taken_by && !!name.trim())))

  $effect(() => {
    const target = path.trim()
    if (!target) {
      info = null
      return
    }
    inspect(target).then((result) => {
      if (target !== path.trim()) return
      if (!name || name === info?.name) name = result.name
      info = result
    })
  })

  async function browse() {
    const chosen = await open({ directory: true, defaultPath: path || undefined })
    if (typeof chosen === 'string') path = chosen
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    if (!ready) return
    busy = true
    try {
      if (revive) await unshelve(projects.archived.find((p) => p.name === info?.taken_by)?.path ?? path.trim())
      else await add(path.trim(), name)
      onclose()
    } catch (e) {
      error = String(e)
      busy = false
    }
  }

  function goTo(project: string) {
    projects.current = projects.list.find((p) => p.name === project)?.path ?? projects.current
    onclose()
  }
</script>

<Dialog title="Add a project" {onclose} width={520}>
  <form onsubmit={submit}>
    <p class="intro">A project is a local folder where Squadron runs your agents.</p>
    <div class="fields">
      <div class="group">
        <label for="folder">Project folder</label>
        <div class="row">
          <input
            id="folder"
            class="field mono"
            bind:value={path}
            placeholder="/path/to/project"
            aria-invalid={!!problem}
            aria-describedby="folder-state"
            spellcheck="false"
          />
          <button type="button" class="btn btn-secondary browse" onclick={browse}>Browse…</button>
        </div>
        <span id="folder-state" class="state">
          {#if problem}
            <span class="bad">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
                ><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16h.01" /></svg
              >
              {problem}
              {#if info?.taken_by}
                <button type="button" class="link" onclick={() => goTo(info!.taken_by!)}
                  >Go to {info.taken_by}</button
                >
              {:else if info?.near}
                <span>Closest folder:</span>
                <button type="button" class="link mono" onclick={() => (path = info!.near!)}>{home(info.near)}</button>
              {/if}
            </span>
          {:else if revive}
            <span class="good">This folder is the archived project {info?.taken_by}. Adding it unarchives {info?.taken_by} with its agents.</span>
          {:else if info?.git}
            <span class="good">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l5 5L20 6" /></svg>
              Git repository found{#if info.branch}&nbsp;· branch <code>{info.branch}</code>{/if}
            </span>
          {:else if info}
            <span>No Git repository: Squadron works without one too.</span>
          {/if}
        </span>
      </div>
      <div class="group">
        <label for="name">Name</label>
        <input id="name" class="field" bind:value={name} disabled={!info?.exists || !!info.taken_by} />
        <span class="state">Filled in from the folder name.</span>
      </div>
      {#if error}<p class="bad" role="alert">{error}</p>{/if}
    </div>
    <footer>
      <button type="button" class="btn btn-secondary" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" disabled={!ready}>{revive ? `Unarchive ${info?.taken_by}` : 'Add project'}</button>
    </footer>
  </form>
</Dialog>

<style>
  .intro {
    margin: 0;
    padding: 0 22px 16px;
    color: var(--text-dim);
    font-size: 13px;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 4px 22px 20px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    font-weight: 600;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .mono {
    font: 13px var(--mono);
  }

  .browse {
    height: 42px;
    background: var(--surface-4);
    color: var(--text);
    font-weight: 600;
  }

  .state {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .state > span {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .good {
    color: var(--done-text);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  code {
    font-size: 12.5px;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .link {
    color: var(--accent-link);
    text-decoration: underline;
  }

  .link.mono {
    font-family: var(--mono);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }
</style>
