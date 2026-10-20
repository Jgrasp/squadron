<script lang="ts">
  import { ask, open } from '@tauri-apps/plugin-dialog'
  import { home, load, move, remove, type Project } from './projects.svelte'

  let { project }: { project: Project } = $props()
  let error = $state('')

  async function retire() {
    const confirmed = await ask(`Retirer ${project.name} de Squadron ? Le dossier n'est pas touché.`, {
      title: 'Retirer le projet',
      kind: 'warning',
      okLabel: 'Retirer',
      cancelLabel: 'Annuler',
    })
    if (confirmed) await remove(project.path)
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
</script>

<div class="body">
  <aside aria-label="Agents du projet">
    <div class="head">
      <div class="who">
        <span class="square" aria-hidden="true"></span>
        <div>
          <strong>{project.name}</strong>
          <small class:bad={!project.exists}>{project.exists ? 'Aucun agent' : 'Dossier introuvable'}</small>
        </div>
      </div>
      <button class="new" disabled={!project.exists}>+ Agent</button>
    </div>

    <div class="empty">
      <svg width="28" height="28" viewBox="0 0 24 24" aria-hidden="true"
        ><rect x="4" y="4" width="16" height="6" rx="2" /><rect x="4" y="14" width="16" height="6" rx="2" stroke-dasharray="3 3" /></svg
      >
      {project.exists
        ? `Les agents de ${project.name} apparaîtront ici, avec leur état et leur coût.`
        : 'Tu pourras créer des agents dès que le dossier sera retrouvé.'}
    </div>

    <div class="info">
      <span class="mono">{home(project.path)}</span>
      <span>{project.git ? `Git · ${project.branch ?? 'dépôt'}` : 'Pas de dépôt Git détecté'}</span>
      <button class="retire" onclick={retire}>Retirer le projet</button>
    </div>
  </aside>

  <main>
    {#if project.exists}
      <div class="box">
        <span class="plus" aria-hidden="true">
          <svg width="24" height="24" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
        </span>
        <h1>Lance le premier agent de {project.name}</h1>
        <p>Donne-lui un nom, choisis le modèle, l'effort et le mode de permission, puis écris sa première consigne.</p>
        <div class="row">
          <button class="btn btn-primary big">Nouvel agent</button>
          <kbd>Ctrl N</kbd>
        </div>
      </div>
    {:else}
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
    {/if}
  </main>
</div>

<style>
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  aside {
    width: 304px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 4px 10px;
  }

  .who {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .who div {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .who small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .who small.bad {
    color: var(--err-text);
  }

  .square {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--accent);
  }

  .new {
    height: 32px;
    padding: 0 12px;
    border: 1px dashed var(--border-dashed);
    border-radius: 4px;
    color: var(--text-2);
    font-weight: 500;
  }

  .new:disabled {
    opacity: 0.4;
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 24px;
    text-align: center;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .empty svg {
    fill: none;
    stroke: var(--text-faint);
    stroke-width: 1.6;
  }

  .info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border-radius: 4px;
    background: var(--surface-1);
    font-size: 12px;
    color: var(--text-dim);
  }

  .info .mono {
    color: var(--text-3);
    overflow-wrap: anywhere;
  }

  .retire {
    align-self: flex-start;
    margin-top: 4px;
    color: var(--text-dim);
    text-decoration: underline;
  }

  .retire:hover {
    color: var(--err-text);
  }

  main {
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
