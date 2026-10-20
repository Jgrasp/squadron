<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { add, inspect, projects, type Inspection } from './projects.svelte'

  let { path: initial, onclose }: { path: string; onclose: () => void } = $props()

  let path = $state(untrack(() => initial))
  let name = $state('')
  let info = $state<Inspection | null>(null)
  let error = $state('')
  let busy = $state(false)

  const problem = $derived.by(() => {
    if (!info || !path.trim()) return ''
    if (!info.exists) return "Ce dossier n'existe pas ou n'est pas accessible."
    if (info.taken_by) return `Ce dossier est déjà le projet ${info.taken_by}.`
    return ''
  })
  const ready = $derived(!!info?.exists && !info.taken_by && !!name.trim() && !busy)

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
      await add(path.trim(), name)
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

<Dialog title="Ajouter un projet" {onclose} width={520}>
  <form onsubmit={submit}>
    <p class="intro">Un projet, c'est un dossier local où Squadron lance tes agents.</p>
    <div class="fields">
      <div class="group">
        <label for="dossier">Dossier du projet</label>
        <div class="row">
          <input
            id="dossier"
            class="field mono"
            bind:value={path}
            placeholder="/chemin/du/projet"
            aria-invalid={!!problem}
            aria-describedby="dossier-etat"
            spellcheck="false"
          />
          <button type="button" class="btn btn-secondary browse" onclick={browse}>Parcourir…</button>
        </div>
        <span id="dossier-etat" class="state">
          {#if problem}
            <span class="bad">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
                ><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16h.01" /></svg
              >
              {problem}
              {#if info?.taken_by}
                <button type="button" class="link" onclick={() => goTo(info!.taken_by!)}
                  >Aller à {info.taken_by}</button
                >
              {/if}
            </span>
          {:else if info?.git}
            <span class="good">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l5 5L20 6" /></svg>
              Dépôt Git détecté{#if info.branch}&nbsp;· branche <code>{info.branch}</code>{/if}
            </span>
          {:else if info}
            <span>Pas de dépôt Git : Squadron fonctionne aussi sans.</span>
          {/if}
        </span>
      </div>
      <div class="group">
        <label for="nom">Nom</label>
        <input id="nom" class="field" bind:value={name} disabled={!info?.exists || !!info.taken_by} />
        <span class="state">Prérempli avec le nom du dossier.</span>
      </div>
      {#if error}<p class="bad" role="alert">{error}</p>{/if}
    </div>
    <footer>
      <button type="button" class="btn btn-secondary" onclick={onclose}>Annuler</button>
      <button class="btn btn-primary" disabled={!ready}>Ajouter le projet</button>
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
