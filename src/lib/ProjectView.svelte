<script lang="ts">
  import { ask, open } from '@tauri-apps/plugin-dialog'
  import AgentList from './AgentList.svelte'
  import AgentView from './AgentView.svelte'
  import { archiveProject, interrupt, ofProject, selected } from './agents.svelte'
  import NewAgent from './NewAgent.svelte'
  import { load, move, remove, type Project } from './projects.svelte'

  let { project }: { project: Project } = $props()
  let error = $state('')
  let creating = $state(false)

  const agent = $derived(selected(project.path))

  async function retire() {
    const running = ofProject(project.path).length
    const confirmed = await ask(
      `Retirer ${project.name} de Squadron ? Le dossier n’est pas touché.${running ? ` Ses ${running} agent${running > 1 ? 's seront arrêtés' : ' sera arrêté'}.` : ''}`,
      { title: 'Retirer le projet', kind: 'warning', okLabel: 'Retirer', cancelLabel: 'Annuler' },
    )
    if (!confirmed) return
    await archiveProject(project.path)
    await remove(project.path)
  }

  async function relocate() {
    const chosen = await open({ directory: true })
    if (typeof chosen !== 'string') return
    try {
      await move(project.path, chosen)
      error = ''
    } catch (e) {
      error = String(e)
    }
  }

  function keys(event: KeyboardEvent) {
    if (document.querySelector('dialog[open]')) return
    if (event.ctrlKey && event.key === 'n' && project.exists) {
      event.preventDefault()
      creating = true
    } else if (event.key === 'Escape' && agent) {
      interrupt(agent.id)
    }
  }
</script>

<svelte:window onkeydown={keys} />

<div class="body">
  <AgentList {project} current={agent} onnew={() => (creating = true)} onretire={retire} />

  {#if !project.exists}
    <main class="center">
      <section class="missing" role="alert" aria-labelledby="introuvable">
        <div class="title">
          <span class="icon" aria-hidden="true">
            <svg width="16" height="16" viewBox="0 0 24 24"><path d="M12 3l9 16H3zM12 10v4M12 17h.01" /></svg>
          </span>
          <h1 id="introuvable">Le dossier de {project.name} est introuvable</h1>
        </div>
        <code>{project.path}</code>
        <p>
          Il a peut-être été déplacé, renommé, ou il est sur un disque non monté. Indique son nouvel
          emplacement pour créer des agents dans {project.name}.
        </p>
        {#if error}<p class="bad">{error}</p>{/if}
        <div class="row">
          <button class="btn btn-primary" onclick={relocate}>Choisir le nouvel emplacement…</button>
          <button class="btn btn-danger" onclick={load}>Réessayer</button>
          <button class="btn btn-danger" onclick={retire}>Retirer le projet</button>
        </div>
      </section>
    </main>
  {:else if agent}
    {#key agent.id}
      <AgentView {agent} />
    {/key}
  {:else}
    <main class="center">
      <div class="box">
        <span class="plus" aria-hidden="true">
          <svg width="24" height="24" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
        </span>
        <h1>Lance le premier agent de {project.name}</h1>
        <p>Donne-lui un nom, choisis le modèle, l’effort et le mode de permission, puis écris sa première consigne.</p>
        <div class="row">
          <button class="btn btn-primary big" onclick={() => (creating = true)}>Nouvel agent</button>
          <kbd>Ctrl N</kbd>
        </div>
      </div>
    </main>
  {/if}
</div>

{#if creating}
  <NewAgent {project} onclose={() => (creating = false)} />
{/if}

<style>
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    background: var(--bg-main);
  }

  .box {
    width: 440px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
  }

  .plus {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: rgba(224, 138, 91, 0.12);
    box-shadow: inset 0 0 0 1px rgba(224, 138, 91, 0.5);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .plus svg {
    stroke: var(--accent-soft);
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.6;
    text-wrap: pretty;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .big {
    height: 44px;
    padding: 0 22px;
  }

  kbd {
    font: 12.5px var(--mono);
    color: var(--text-dim);
  }

  .missing {
    width: 560px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 22px;
    border-radius: 6px;
    background: var(--err-bg);
    box-shadow: 0 0 0 1px var(--err-border);
  }

  .missing .title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .missing h1 {
    font-size: 16px;
    color: var(--text-strong);
  }

  .missing .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--err);
    color: var(--err-ink);
  }

  .missing code {
    padding: 8px 12px;
    border-radius: 4px;
    background: var(--err-pre);
    color: var(--err-text);
    font-size: 12.5px;
  }

  .missing p {
    color: var(--err-muted);
  }

  .missing .btn {
    height: 36px;
  }

  .btn-danger {
    background: #3a2420;
    color: #f8d5cc;
  }

  .bad {
    color: var(--err-text);
  }
</style>
