<script lang="ts">
  import { archive, interrupt, reload, reply, send } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { changedFiles } from './diff'
  import DiffPanel from './DiffPanel.svelte'
  import { cost, meta, time, tokens } from './format'
  import { calls, type Agent, type ToolCall } from './stream'

  let { agent }: { agent: Agent } = $props()

  let text = $state('')
  let menuOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = $state(true)
  let diffFile = $state<string | null>(null)
  let marks = $state<{ top: number; offset: number; text: string; at: number }[]>([])
  let active = $state(-1)
  let below = $state(0)
  let hovered = $state(-1)

  const files = $derived(changedFiles(calls(agent)))
  const totals = $derived(files.reduce((t, f) => ({ add: t.add + f.add, del: t.del + f.del }), { add: 0, del: 0 }))

  const busy = $derived(agent.status === 'working' || agent.status === 'waiting')
  const pending = $derived(agent.asks[0]?.tool_name)
  const placeholder = $derived(
    pending === 'AskUserQuestion'
      ? `Réponse libre à ${agent.name}…`
      : pending === 'ExitPlanMode'
        ? 'Ce qui doit changer dans le plan…'
        : pending
          ? `Refuser avec une consigne pour ${agent.name}…`
          : agent.status === 'working'
      ? 'Ajouter une consigne, lue dès la fin de l’action en cours…'
      : agent.status === 'interrupted'
        ? 'Nouvelle consigne, ou « continue » pour reprendre'
        : agent.status === 'error'
          ? `Écrire à ${agent.name} : la session sera rechargée avant l’envoi`
          : `Écrire à ${agent.name}…`,
  )

  function measure() {
    const users = agent.entries.flatMap((e) => (e.kind === 'user' ? [e] : []))
    const els = [...column.querySelectorAll<HTMLElement>('[data-user]')]
    const height = Math.max(scroller.scrollHeight, 1)
    marks = els.map((el, i) => ({
      top: (el.offsetTop / height) * 100,
      offset: el.offsetTop,
      text: users[i]?.text ?? '',
      at: users[i]?.at ?? 0,
    }))
    const view = scroller.scrollTop + scroller.clientHeight / 3
    active = marks.findLastIndex((m) => m.offset <= view)
    below = marks.filter((m) => m.offset > scroller.scrollTop + scroller.clientHeight).length
  }

  let touched = 0

  function scrolled() {
    if (scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40) stuck = true
    else if (Date.now() - touched < 500) stuck = false
    measure()
  }

  $effect(() => {
    const observer = new ResizeObserver(() => {
      if (stuck) scroller.scrollTo({ top: scroller.scrollHeight })
      measure()
    })
    observer.observe(column)
    return () => observer.disconnect()
  })

  function jump(i: number) {
    stuck = false
    touched = Date.now()
    scroller.scrollTo({ top: Math.max(0, marks[i].offset - 18), behavior: 'smooth' })
  }

  function bottom() {
    stuck = true
    scroller.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' })
  }

  function submit(event?: Event) {
    event?.preventDefault()
    const message = text.trim()
    if (!message) return
    text = ''
    stuck = true
    if (!reply(agent, message)) send(agent.id, message)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) submit(event)
  }

  function ondiff(call: ToolCall) {
    diffFile = String(call.result?.data?.filePath ?? call.input.file_path ?? '')
  }
</script>

<svelte:window onclick={(e) => !(e.target as HTMLElement).closest('[data-agent-menu]') && (menuOpen = false)} />

<main>
  <section class="conversation" aria-label="Conversation">
    <div class="top">
      <div class="title">
        <h1>{agent.name}</h1>
        <span>{meta(agent)} · <span class="mono">{cost(agent.cost)} · {tokens(agent.tokens)} tokens de contexte</span></span>
      </div>
      {#if files.length}
        <button
          class="btn btn-ghost"
          aria-pressed={diffFile !== null}
          title="Voir les modifications de l’agent"
          onclick={() => (diffFile = diffFile === null ? '' : null)}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
            ><circle cx="6" cy="6" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8a2 2 0 0 0 2 2h8M18 16V8a2 2 0 0 0-2-2H8" /></svg
          >
          Diff <span class="mono"><span class="add">+{totals.add}</span>{#if totals.del}&nbsp;<span class="del">−{totals.del}</span>{/if}</span>
        </button>
      {/if}
    </div>

    <div class="middle">
      {#if marks.length}
        <nav class="rail" aria-label="Repères de la conversation">
          {#each marks as mark, i (i)}
            <button
              style:top="{mark.top}%"
              aria-label="Aller au message {i + 1} : {mark.text.slice(0, 80)}"
              aria-current={i === active ? 'true' : undefined}
              onclick={() => jump(i)}
              onmouseenter={() => (hovered = i)}
              onmouseleave={() => (hovered = -1)}
              onfocus={() => (hovered = i)}
              onblur={() => (hovered = -1)}
            >
              <span></span>
            </button>
          {/each}
          {#if hovered >= 0}
            <div class="tooltip" role="tooltip" style:top="calc({marks[hovered].top}% - 10px)">
              <small>Message {hovered + 1} · {time(marks[hovered].at)}</small>
              <span>{marks[hovered].text}</span>
            </div>
          {/if}
        </nav>
      {/if}

      <div
        class="scroller"
        bind:this={scroller}
        onscroll={scrolled}
        onwheel={() => (touched = Date.now())}
        onpointerdown={() => (touched = Date.now())}
        onkeydown={() => (touched = Date.now())}
        role="presentation"
      >
        <div class="column" bind:this={column}>
          <Conversation {agent} {ondiff} />
        </div>
      </div>

      {#if !stuck}
        <button class="latest" onclick={bottom}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M6 13l6 6 6-6" /></svg>
          Dernier message{#if below}<span>&nbsp;· {below} message{below > 1 ? 's' : ''} plus bas</span>{/if}
        </button>
      {/if}
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
      <button class="btn btn-primary send" disabled={!text.trim()}>
        {pending === 'AskUserQuestion' ? 'Répondre' : pending === 'ExitPlanMode' ? 'Envoyer la remarque' : pending ? 'Refuser' : 'Envoyer'}
      </button>
    </div>
  </form>
  </section>

  {#if diffFile !== null}
    <DiffPanel {agent} bind:file={diffFile} onclose={() => (diffFile = null)} />
  {/if}
</main>

<style>
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    background: var(--bg-main);
  }

  .conversation {
    flex: 1;
    min-width: 360px;
    display: flex;
    flex-direction: column;
  }

  .top .btn-ghost[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .top svg,
  .latest svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .add {
    color: var(--ok);
  }

  .del {
    color: var(--err);
  }

  .middle {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .rail {
    position: relative;
    width: 40px;
    flex: none;
    border-right: 1px solid var(--border);
    background: var(--bg-rail);
  }

  .rail button {
    position: absolute;
    left: 6px;
    width: 28px;
    height: 16px;
    margin-top: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
  }

  .rail button:hover {
    background: var(--menu);
  }

  .rail button span {
    width: 10px;
    height: 2px;
    border-radius: 1px;
    background: var(--text-faint);
  }

  .rail button:hover span {
    width: 14px;
    background: var(--text);
  }

  .rail button[aria-current='true'] span {
    width: 18px;
    height: 3px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 8px rgba(224, 138, 91, 0.6);
  }

  .tooltip {
    position: absolute;
    left: 44px;
    z-index: 6;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 12px 30px rgba(0, 0, 0, 0.5);
    pointer-events: none;
  }

  .tooltip small {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-dim);
  }

  .tooltip span {
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 13px;
    line-height: 1.45;
  }

  .latest {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 16px;
    border-radius: 18px;
    background: var(--text);
    color: var(--ink);
    font-weight: 600;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
  }

  .latest span {
    font-weight: 500;
    color: var(--border-dashed);
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
    min-width: 0;
    overflow-y: auto;
    padding: 20px 20px 60px;
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
    border-top: 1px solid var(--border);
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
