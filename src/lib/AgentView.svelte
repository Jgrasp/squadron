<script lang="ts">
  import { archive, interrupt, reload, send } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { cost, meta, tokens } from './format'
  import type { Agent, ToolCall } from './stream'

  let { agent }: { agent: Agent } = $props()

  let text = $state('')
  let menuOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = true

  const busy = $derived(agent.status === 'working' || agent.status === 'waiting')
  const placeholder = $derived(
    agent.status === 'working'
      ? 'Ajouter une consigne, lue dès la fin de l’action en cours…'
      : agent.status === 'interrupted'
        ? 'Nouvelle consigne, ou « continue » pour reprendre'
        : agent.status === 'error'
          ? `Écrire à ${agent.name} : la session sera rechargée avant l’envoi`
          : `Écrire à ${agent.name}…`,
  )

  $effect(() => {
    const observer = new ResizeObserver(() => stuck && scroller.scrollTo({ top: scroller.scrollHeight }))
    observer.observe(column)
    return () => observer.disconnect()
  })

  function submit(event?: Event) {
    event?.preventDefault()
    const message = text.trim()
    if (!message) return
    text = ''
    stuck = true
    send(agent.id, message)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) submit(event)
  }

  function ondiff(_call: ToolCall) {}
</script>

<svelte:window onclick={(e) => !(e.target as HTMLElement).closest('[data-agent-menu]') && (menuOpen = false)} />

<main>
  <div class="top">
    <div class="title">
      <h1>{agent.name}</h1>
      <span>{meta(agent)} · <span class="mono">{cost(agent.cost)} · {tokens(agent.tokens)} tokens</span></span>
    </div>
  </div>

  <div class="scroller" bind:this={scroller} onscroll={() => (stuck = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40)}>
    <div class="column" bind:this={column}>
      <Conversation {agent} {ondiff} />
    </div>
  </div>

  <form class="composer" onsubmit={submit}>
    <div class="bar">
      <label for="saisie" class="sr-only">Message à l’agent</label>
      <textarea id="saisie" rows="1" bind:value={text} {placeholder} onkeydown={keys}></textarea>
      {#if busy}
        <button type="button" class="btn-icon" aria-label="Interrompre" title="Interrompre (Échap)" onclick={() => interrupt(agent.id)}>
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3" fill="currentColor" /></svg>
        </button>
      {/if}
      <div class="anchor" data-agent-menu>
        <button
          type="button"
          class="btn-icon"
          class:open={menuOpen}
          aria-label="Plus d’actions"
          title="Plus d’actions"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor"
            ><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg
          >
        </button>
        {#if menuOpen}
          <div class="menu" role="menu" aria-label="Actions de l’agent">
            <button type="button" role="menuitem" onclick={() => ((menuOpen = false), reload(agent.id))}>
              <strong>Recharger</strong><span>Relance la session, garde la conversation</span>
            </button>
            <span class="rule" aria-hidden="true"></span>
            <button type="button" role="menuitem" onclick={() => ((menuOpen = false), archive(agent.id))}>
              <strong>Archiver</strong><span>Arrête l’agent et le retire de la liste</span>
            </button>
          </div>
        {/if}
      </div>
      <button class="btn btn-primary send" disabled={!text.trim()}>Envoyer</button>
    </div>
  </form>
</main>

<style>
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-main);
  }

  .top {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title > span {
    font-size: 12px;
    color: var(--text-dim);
  }

  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px;
  }

  .column {
    max-width: 740px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .composer {
    flex: none;
    padding: 12px 20px 18px;
  }

  .bar {
    max-width: 740px;
    margin: 0 auto;
    display: flex;
    align-items: flex-end;
    gap: 10px;
    padding: 8px 8px 8px 18px;
    border-radius: 6px;
    background: var(--surface-2);
  }

  textarea {
    flex: 1;
    min-height: 38px;
    max-height: 200px;
    padding: 9px 0;
    border: none;
    outline: none;
    resize: none;
    background: transparent;
    line-height: 1.5;
    field-sizing: content;
  }

  .btn-icon.open {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .send {
    font-weight: 600;
  }

  .anchor {
    position: relative;
  }

  .menu {
    position: absolute;
    right: 0;
    bottom: 48px;
    z-index: 5;
    width: 270px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .menu button {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 9px 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
  }

  .menu strong {
    font-weight: 600;
  }

  .menu span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .rule {
    height: 1px;
    margin: 4px;
    background: var(--border-strong);
  }
</style>
