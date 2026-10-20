<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick } from 'svelte'
  import { archive, fork, interrupt, reload, rename, reply, send } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { changedFiles, type Changes } from './diff'
  import DiffPanel from './DiffPanel.svelte'
  import { cost, meta, time, tokens } from './format'
  import { toggleBoard } from './projects.svelte'
  import { calls, relative, type Agent, type ToolCall } from './stream'

  let { agent }: { agent: Agent } = $props()

  let text = $state('')
  let menuOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = $state(true)
  let diffFile = $state<string | null>(null)
  let marks = $state<{ offset: number; text: string; at: number }[]>([])
  let active = $state(-1)
  let below = $state(0)
  let hovered = $state(-1)
  let tooltipTop = $state(0)
  let renaming = $state(false)
  let newName = $state('')
  let recall = -1
  let draft = ''

  const edited = $derived(changedFiles(calls(agent)))
  const edits = $derived(edited.reduce((n, f) => n + f.calls.length, 0))
  const panelOpen = $derived(diffFile !== null)
  let changes = $state<Changes | null>(null)
  let refreshes = 0
  const files = $derived(changes ? changes.files.map((f) => ({ add: f.add ?? 0, del: f.del ?? 0 })) : edited)
  const totals = $derived(files.reduce((t, f) => ({ add: t.add + f.add, del: t.del + f.del }), { add: 0, del: 0 }))

  $effect(() => {
    void agent.turns
    void edits
    void panelOpen
    const run = ++refreshes
    invoke<Changes>('git_changes', { project: agent.project, base: agent.base })
      .then((result) => {
        if (run === refreshes) changes = result
      })
      .catch(() => {
        if (run === refreshes) changes = null
      })
  })

  function addComment(block: string) {
    text = text.trim() ? `${text.trimEnd()}\n\n${block}` : block
  }
  const sent = $derived(agent.entries.flatMap((e) => (e.kind === 'user' ? [e.text] : [])))

  const busy = $derived(agent.status === 'working' || agent.status === 'waiting')
  const pending = $derived(agent.asks[0]?.tool_name)
  const placeholder = $derived(
    pending === 'AskUserQuestion'
      ? `Free answer to ${agent.name}…`
      : pending === 'ExitPlanMode'
        ? 'What should change in the plan…'
        : pending
          ? `Deny with an instruction for ${agent.name}…`
          : agent.status === 'working'
            ? 'Add an instruction, read once the current action ends…'
            : agent.status === 'interrupted'
              ? 'New instruction, or “continue” to resume'
              : agent.status === 'error'
                ? `Write to ${agent.name}: the session will reload before sending`
                : `Write to ${agent.name}…`,
  )

  function measure() {
    const users = agent.entries.flatMap((e) => (e.kind === 'user' ? [e] : []))
    const els = [...column.querySelectorAll<HTMLElement>('[data-user]')]
    marks = els.map((el, i) => ({ offset: el.offsetTop, text: users[i]?.text ?? '', at: users[i]?.at ?? 0 }))
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

  function hover(i: number, event: Event) {
    hovered = i
    const mark = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const rail = (event.currentTarget as HTMLElement).closest('nav')!.getBoundingClientRect()
    tooltipTop = mark.top - rail.top + mark.height / 2
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
    recall = -1
    stuck = true
    if (!reply(agent, message)) send(agent.id, message)
  }

  function show(area: HTMLTextAreaElement, value: string) {
    text = value
    tick().then(() => area.setSelectionRange(value.length, value.length))
  }

  function keys(event: KeyboardEvent) {
    const area = event.currentTarget as HTMLTextAreaElement
    if (event.key === 'Enter' && !event.shiftKey) {
      submit(event)
    } else if (event.key === 'ArrowUp' && !text.slice(0, area.selectionStart).includes('\n') && recall < sent.length - 1) {
      event.preventDefault()
      if (recall === -1) draft = text
      recall++
      show(area, sent[sent.length - 1 - recall])
    } else if (event.key === 'ArrowDown' && !text.slice(area.selectionEnd).includes('\n') && recall >= 0) {
      event.preventDefault()
      recall--
      show(area, recall === -1 ? draft : sent[sent.length - 1 - recall])
    }
  }

  function startRename() {
    newName = agent.name
    renaming = true
  }

  function finishRename(save: boolean) {
    if (save) rename(agent.id, newName)
    renaming = false
  }

  function ondiff(call: ToolCall) {
    diffFile = relative(agent, call.result?.data?.filePath ?? call.input.file_path ?? '')
  }
</script>

<svelte:window onclick={(e) => !(e.target as HTMLElement).closest('[data-agent-menu]') && (menuOpen = false)} />

<main>
  <section class="conversation" aria-label="Conversation">
    <div class="top">
      <div class="title">
        {#if renaming}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="rename"
            aria-label="Agent name"
            bind:value={newName}
            autofocus
            onkeydown={(e) => (e.key === 'Enter' ? finishRename(true) : e.key === 'Escape' && finishRename(false))}
            onblur={() => finishRename(true)}
          />
        {:else}
          <h1><button class="name" title="Rename" onclick={startRename}>{agent.name}</button></h1>
        {/if}
        <span>{meta(agent)} · <span class="mono">{cost(agent.cost)} · {tokens(agent.tokens)} context tokens</span></span>
      </div>
      {#if files.length}
        <button
          class="btn btn-ghost"
          aria-pressed={diffFile !== null}
          title="See the agent’s changes"
          onclick={() => (diffFile = diffFile === null ? '' : null)}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
            ><circle cx="6" cy="6" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8a2 2 0 0 0 2 2h8M18 16V8a2 2 0 0 0-2-2H8" /></svg
          >
          Diff <span class="mono"><span class="add">+{totals.add}</span>{#if totals.del}&nbsp;<span class="del">−{totals.del}</span>{/if}</span>
        </button>
      {/if}
      <button class="btn btn-ghost" title="Kanban (Ctrl K)" onclick={toggleBoard}>
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="5" height="16" rx="1" /><rect x="10" y="4" width="5" height="10" rx="1" /><rect x="17" y="4" width="4" height="13" rx="1" /></svg>
        Kanban
      </button>
    </div>

    <div class="middle">
      {#if marks.length}
        <nav class="rail" aria-label="Conversation landmarks">
          <div class="marks">
            {#each marks as mark, i (i)}
              <button
                aria-label="Go to message {i + 1}: {mark.text.slice(0, 80)}"
                aria-current={i === active ? 'true' : undefined}
                onclick={() => jump(i)}
                onmouseenter={(e) => hover(i, e)}
                onmouseleave={() => (hovered = -1)}
                onfocus={(e) => hover(i, e)}
                onblur={() => (hovered = -1)}
              >
                <span></span>
              </button>
            {/each}
          </div>
          {#if hovered >= 0}
            <div class="tooltip" role="tooltip" style:top="{tooltipTop}px">
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
          Latest message{#if below}<span>&nbsp;· {below} message{below > 1 ? 's' : ''} below</span>{/if}
        </button>
      {/if}
    </div>

    <form class="composer" onsubmit={submit}>
      <div class="bar">
        <label for="composer" class="sr-only">Message to the agent</label>
        <textarea id="composer" rows="1" bind:value={text} {placeholder} onkeydown={keys}></textarea>
        {#if busy}
          <button type="button" class="btn-icon" aria-label="Interrupt" title="Interrupt (Esc)" onclick={() => interrupt(agent.id)}>
            <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3" fill="currentColor" /></svg>
          </button>
        {/if}
        <div class="anchor" data-agent-menu>
          <button
            type="button"
            class="btn-icon"
            class:open={menuOpen}
            aria-label="More actions"
            title="More actions"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onclick={() => (menuOpen = !menuOpen)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor"
              ><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg
            >
          </button>
          {#if menuOpen}
            <div class="menu" role="menu" aria-label="Agent actions">
              <button type="button" role="menuitem" onclick={() => ((menuOpen = false), reload(agent.id))}>
                <strong>Reload</strong><span>Restarts the session, keeps the conversation</span>
              </button>
              <button type="button" role="menuitem" onclick={() => ((menuOpen = false), startRename())}>
                <strong>Rename</strong><span>Give the agent a clearer name</span>
              </button>
              <button
                type="button"
                role="menuitem"
                disabled={!agent.sessionId}
                onclick={() => ((menuOpen = false), fork(agent.id))}
              >
                <strong>Fork</strong><span>New agent, same context</span>
              </button>
              <span class="rule" aria-hidden="true"></span>
              <button type="button" role="menuitem" onclick={() => ((menuOpen = false), archive(agent.id))}>
                <strong>Archive</strong><span>Stops the agent and removes it from the list</span>
              </button>
            </div>
          {/if}
        </div>
        <button class="btn btn-primary send" disabled={!text.trim()}>
          {pending === 'AskUserQuestion' ? 'Answer' : pending === 'ExitPlanMode' ? 'Send feedback' : pending ? 'Deny' : 'Send'}
        </button>
      </div>
    </form>
  </section>

  {#if diffFile !== null}
    <DiffPanel {agent} {changes} bind:file={diffFile} oncomment={addComment} onclose={() => (diffFile = null)} />
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

  .marks {
    position: absolute;
    inset: 12px 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
  }

  .rail button {
    flex: 0 1 10px;
    min-height: 3px;
    width: 28px;
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
    transform: translateY(-50%);
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

  h1 .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    border-radius: 3px;
    text-align: left;
  }

  h1 .name:hover {
    box-shadow: 0 0 0 4px var(--surface-2);
    background: var(--surface-2);
  }

  .rename {
    height: 26px;
    padding: 0 6px;
    margin-left: -6px;
    border: none;
    border-radius: 3px;
    outline: none;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 2px var(--border-focus);
    font-size: 15px;
    font-weight: 600;
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

  .menu button:disabled {
    opacity: 0.45;
  }

  .menu button:not(:disabled):hover {
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
