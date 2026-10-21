<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, untrack } from 'svelte'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { archive, fork, interrupt, note, reload, remote, rename, reply, send, setOptions } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { changedFiles, type Changes } from './diff'
  import DiffPanel from './DiffPanel.svelte'
  import { cost, meta, modelLabel, time, tokens } from './format'
  import EditorPanel from './EditorPanel.svelte'
  import { layout, setPanel, type PanelKind } from './layout.svelte'
  import { projects, toggleBoard } from './projects.svelte'
  import { ui } from './actions.svelte'
  import { all, detailOf, hidden, labelOf, shown, sortable, toggle as toggleItem } from './layout-slots.svelte'
  import Icon from './Icon.svelte'
  import { hint } from './settings.svelte'
  import TerminalPanel from './TerminalPanel.svelte'
  import CommentChip from './CommentChip.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import { catalogOf, commandsOf } from './catalogs.svelte'
  import { find as findCommand, options, parse, type Command } from './commands'
  import { calls, known, prompt, relative, type Agent, type Comment, type Options, type ToolCall } from './stream'

  let { agent }: { agent: Agent } = $props()

  let text = $state('')
  let menuOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = $state(true)
  let diffFile = $state('')
  const panel = $derived(layout.panels[agent.project] ?? null)
  const toggle = (kind: PanelKind) => setPanel(agent.project, panel === kind ? null : kind)
  const openDiff = (path: string) => ((diffFile = path), setPanel(agent.project, 'diff'))
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
  const panelOpen = $derived(panel)
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

  let comments = $state<Comment[]>([])
  let tuning = $state<Options | null>(null)

  $effect(() => {
    const next = tuning && $state.snapshot(tuning)
    if (!next) return
    const now = untrack(() => agent.options)
    if (next.model !== now.model || next.effort !== now.effort || next.mode !== now.mode) setOptions(agent.id, next)
  })

  const MENU: Record<string, () => void> = {
    reload: () => reload(agent.id),
    rename: () => startRename(),
    fork: () => fork(agent.id),
    remote: () => remote(agent.id, !agent.remote),
    archive: () => archive(agent.id),
  }

  const branch = $derived(projects.list.find((p) => p.path === agent.project)?.branch)
  const info = $derived(
    shown('agent.info').filter(
      (id) => (id !== 'remote' || agent.remote) && (id !== 'branch' || branch) && (id !== 'tasks' || agent.tasks.length),
    ),
  )

  type Item = { value: string; label: string; hint?: string; detail?: string; tag?: string; text: string }

  let dismissed = $state('')
  let pick = $state(0)
  let composer: HTMLTextAreaElement
  const commandList = $derived(commandsOf(agent))
  const stage = $derived.by(() => {
    if (text === dismissed) return null
    const m = /^\/(\S*)(?:(\s+)(\S*))?$/.exec(text)
    if (!m) return null
    if (m[2] === undefined) return { query: m[1].toLowerCase() }
    const command = findCommand(commandList, m[1])
    return command ? { command, query: (m[3] ?? '').toLowerCase() } : null
  })
  const tagOf = (c: Command) =>
    c.squadron ? 'Squadron' : c.builtin ? 'Claude Code' : (agent.skills.length ? agent.skills : known.skills).includes(c.name) ? 'Skill' : c.name.includes(':') ? 'Plugin' : 'Command'
  const suggestions = $derived.by((): Item[] => {
    if (!stage) return []
    if (!stage.command) {
      const q = stage.query
      return commandList
        .filter((c) => !c.name.startsWith('_') && !c.description.startsWith('(removed)'))
        .filter((c) => c.name.toLowerCase().includes(q) || c.aliases?.some((a) => a.startsWith(q)))
        .sort((a, b) => Number(!a.name.startsWith(q)) - Number(!b.name.startsWith(q)) || Number(!a.builtin) - Number(!b.builtin) || a.name.localeCompare(b.name))
        .slice(0, 80)
        .map((c) => ({ value: c.name, label: `/${c.name}`, detail: c.description, tag: tagOf(c), text: `/${c.name} ` }))
    }
    const command = stage.command
    return options(command, catalogOf(agent))
      .filter((o) => `${o.value} ${o.label ?? ''}`.toLowerCase().includes(stage.query))
      .map((o) => ({ value: o.value, label: o.label ?? o.value, detail: o.detail, hint: o.label ? o.value : undefined, text: `/${command.name} ${o.value} ` }))
  })
  const guide = $derived(stage?.command && !suggestions.length && !stage.query ? stage.command : null)

  $effect(() => {
    void stage?.command
    void stage?.query
    pick = 0
  })

  function complete(item: Item) {
    text = item.text
    dismissed = ''
    composer.focus()
  }

  function local(message: string) {
    const call = parse(message)
    if (!call) return false
    const command = findCommand(commandList, call.name)
    if (command?.name === 'remote-control') {
      const on = call.args ? call.args === 'on' : !agent.remote
      remote(agent.id, on)
      note(agent.id, message, on ? 'Remote control turned on' : 'Remote control turned off')
      return true
    }
    if (command?.name === 'model' && ['opus', 'sonnet', 'haiku'].includes(call.args)) {
      setOptions(agent.id, { ...agent.options, model: call.args as Options['model'] })
      note(agent.id, message, `Model set to ${modelLabel(call.args as Options['model'])}`)
      return true
    }
    if (command?.name === 'effort' && ['low', 'medium', 'high', 'xhigh', 'max'].includes(call.args)) {
      setOptions(agent.id, { ...agent.options, effort: call.args as Options['effort'] })
      note(agent.id, message, `Effort set to ${call.args}, from the next message`)
      return true
    }
    return false
  }


  function excerpt(selection: string) {
    const block = `\`\`\`\n${selection.trimEnd()}\n\`\`\``
    text = text.trim() ? `${text.trimEnd()}\n\n${block}` : block
  }
  let target = $state<Comment | null>(null)
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
    marks = els.map((el, i) => ({ offset: el.offsetTop, text: prompt(users[i]?.text ?? '', users[i]?.comments), at: users[i]?.at ?? 0 }))
    const view = scroller.scrollTop + scroller.clientHeight / 3
    const end = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 4
    active = picked >= 0 ? picked : end ? marks.length - 1 : scroller.scrollTop < 4 ? 0 : marks.findLastIndex((m) => m.offset <= view)
    below = marks.filter((m) => m.offset > scroller.scrollTop + scroller.clientHeight).length
  }

  let touched = 0
  let picked = -1

  function touch() {
    touched = Date.now()
    picked = -1
  }

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
    picked = active = i
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
    picked = -1
    scroller.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' })
  }

  function submit(event?: Event) {
    event?.preventDefault()
    const message = text.trim()
    const attached = $state.snapshot(comments)
    if (!message && !attached.length) return
    text = ''
    recall = -1
    stuck = true
    if (!attached.length && !agent.asks.length && local(message)) return
    comments = []
    if (!reply(agent, prompt(message, attached))) send(agent.id, message, attached)
  }

  function show(area: HTMLTextAreaElement, value: string) {
    text = value
    tick().then(() => area.setSelectionRange(value.length, value.length))
  }

  function keys(event: KeyboardEvent) {
    const area = event.currentTarget as HTMLTextAreaElement
    if (suggestions.length) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault()
        pick = (pick + (event.key === 'ArrowDown' ? 1 : -1) + suggestions.length) % suggestions.length
        tick().then(() => document.querySelector('.choices [aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }))
        return
      }
      if (event.key === 'Tab' || (event.key === 'Enter' && !event.shiftKey && suggestions[pick].text.trim() !== text.trim())) {
        event.preventDefault()
        complete(suggestions[pick])
        return
      }
    }
    if ((suggestions.length || guide) && event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      dismissed = text
      return
    }
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
    openDiff(relative(agent, call.result?.data?.filePath ?? call.input.file_path ?? ''))
  }
</script>

<svelte:window
  onclick={(e) => {
    const target = e.target as HTMLElement
    if (!target.closest('[data-agent-menu]')) menuOpen = false
    if (!target.closest('[data-options]')) tuning = null
  }}
/>

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
        <span class="meta" data-options>
          {#each info as id (id)}
            <span class="info">
              {#if id === 'options'}
                <button class="settings" aria-haspopup="dialog" aria-expanded={!!tuning} title="Change the model, the effort or the permission mode" onclick={() => (tuning = tuning ? null : { ...agent.options })}>
                  {meta(agent)}
                  <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
              {:else if id === 'remote'}
                <button
                  class="remote"
                  class:connected={agent.remote?.state === 'connected'}
                  title="Remote control is on ({agent.remote?.state}). Open this conversation in claude.ai"
                  onclick={() => agent.remote?.url && openUrl(agent.remote.url)}
                >
                  <Icon name="remote" size={12} />Remote
                </button>
              {:else if id === 'cost'}
                <span class="mono">{cost(agent.cost)}</span>
              {:else if id === 'context'}
                <span class="mono">{tokens(agent.tokens)} context tokens</span>
              {:else if id === 'branch'}
                <span class="mono" title="Git branch of the project"><Icon name="branch" size={11} /> {branch}</span>
              {:else if id === 'tasks'}
                <span>{agent.tasks.filter((t) => t.status === 'completed').length}/{agent.tasks.length} tasks</span>
              {/if}
            </span>
          {/each}
          {#if tuning}
            <div class="tuning" role="dialog" aria-label="Agent options">
              <OptionsFields bind:options={tuning} />
              <p>Model and permission mode change at once, even mid-turn. The effort applies from your next message.</p>
            </div>
          {/if}
        </span>
      </div>
      <div class="tools" class:compact={panel} class:customizing={ui.customizing} role="group" aria-label="Open in a tool" use:sortable={{ slot: 'agent.tools', enabled: ui.customizing }}>
    {#snippet editorTool()}
      <button class="btn btn-ghost" aria-label="Editor" aria-pressed={panel === 'editor'} title="Browse and edit the project files ({hint('editor')})" onclick={() => toggle('editor')}>
        <Icon name="editor" />
        {#if !panel}Editor{/if}
      </button>
    {/snippet}
    {#snippet terminalTool()}
      <button class="btn btn-ghost" aria-label="Terminal" aria-pressed={panel === 'terminal'} title="A shell in the project folder ({hint('terminal')})" onclick={() => toggle('terminal')}>
        <Icon name="terminal" />
        {#if !panel}Terminal{/if}
      </button>
    {/snippet}
    {#snippet diffTool()}
      <button class="btn btn-ghost" aria-label="Diff" aria-pressed={panel === 'diff'} title="See the agent’s changes ({hint('diff')})" onclick={() => toggle('diff')}>
        <Icon name="diff" />
        {#if !panel}
          Diff
          {#if totals.add || totals.del}
            <span class="mono"><span class="add">+{totals.add}</span>{#if totals.del}&nbsp;<span class="del">−{totals.del}</span>{/if}</span>
          {/if}
        {/if}
      </button>
    {/snippet}
    {#snippet kanbanTool()}
      <button class="btn btn-ghost" aria-label="Kanban" title="Kanban ({hint('kanban')})" onclick={toggleBoard}>
        <Icon name="kanban" />
        {#if !panel}Kanban{/if}
      </button>
    {/snippet}
      {#each ui.customizing ? all('agent.tools') : shown('agent.tools') as id (id)}
          <span class="item" class:off={hidden('agent.tools', id)} data-item={id}>
            {@render (id === 'editor' ? editorTool : id === 'terminal' ? terminalTool : id === 'diff' ? diffTool : kanbanTool)()}
            {#if ui.customizing}
              <button class="eye" data-keep aria-label="{hidden('agent.tools', id) ? 'Show' : 'Hide'} {labelOf('agent.tools', id)}" onclick={() => toggleItem('agent.tools', id)}>
                <Icon name={hidden('agent.tools', id) ? 'eye-off' : 'eye'} size={12} />
              </button>
            {/if}
          </span>
        {/each}
      </div>
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
        onwheel={touch}
        onpointerdown={touch}
        onkeydown={touch}
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
      {#if comments.length}
        <div class="attached" aria-label="Comments on the diff, sent with your message">
          {#each comments as comment, i (i)}
            <CommentChip {comment} onopen={() => ((target = comment), openDiff(comment.path))} onremove={() => comments.splice(i, 1)} />
          {/each}
        </div>
      {/if}
      {#if suggestions.length || guide}
        <div class="commands" role="listbox" aria-label={stage?.command ? `Arguments of /${stage.command.name}` : 'Commands'}>
          {#if stage?.command}
            <div class="usage" role="presentation">
              <span class="slash">/{stage.command.name}</span>
              <span class="what">{stage.command.description}</span>
              {#if stage.command.argumentHint}<code class="usage-hint">{stage.command.argumentHint}</code>{/if}
            </div>
          {/if}
          <div class="choices">
            {#each suggestions as item, i (item.value)}
              <button type="button" role="option" aria-selected={i === pick} onmousedown={(e) => e.preventDefault()} onclick={() => complete(item)}>
                <span class={stage?.command ? 'option' : 'slash'}>{item.label}</span>
                {#if item.hint}<span class="hint">{item.hint}</span>{/if}
                <span class="what">{item.detail ?? ''}</span>
                {#if item.tag}<span class="tag">{item.tag}</span>{/if}
              </button>
            {/each}
          </div>
          <div class="keys-help" role="presentation">
            {#if suggestions.length}<kbd>↑</kbd><kbd>↓</kbd> to move · <kbd>Tab</kbd> or <kbd>Enter</kbd> to choose · {/if}<kbd>Esc</kbd> to close
          </div>
        </div>
      {/if}
      <div class="bar">
        <label for="composer" class="sr-only">Message to the agent</label>
        <textarea id="composer" rows="1" bind:value={text} bind:this={composer} {placeholder} onkeydown={keys}></textarea>
        {#if busy}
          <button type="button" class="btn-icon" aria-label="Interrupt" title="Interrupt ({hint('interrupt')})" onclick={() => interrupt(agent.id)}>
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
              {#each shown('agent.menu') as id (id)}
                {#if id === 'archive'}<span class="rule" aria-hidden="true"></span>{/if}
                <button type="button" role="menuitem" disabled={id === 'fork' && !agent.sessionId} onclick={() => ((menuOpen = false), MENU[id]())}>
                  <strong>{id === 'remote' && agent.remote ? 'Turn off remote control' : labelOf('agent.menu', id)}</strong>
                  <span>{id === 'remote' && agent.remote ? 'The conversation goes back to this computer only' : detailOf('agent.menu', id)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button class="btn btn-primary send" disabled={!text.trim() && !comments.length}>
          {pending === 'AskUserQuestion' ? 'Answer' : pending === 'ExitPlanMode' ? 'Send feedback' : pending ? 'Deny' : 'Send'}
        </button>
      </div>
    </form>
  </section>

  {#if panel === 'diff'}
    <DiffPanel {agent} {changes} bind:file={diffFile} bind:comments bind:target onclose={() => setPanel(agent.project, null)} />
  {:else if panel === 'editor'}
    <EditorPanel {agent} {changes} onclose={() => setPanel(agent.project, null)} />
  {:else if panel === 'terminal'}
    <TerminalPanel {agent} onsend={excerpt} onclose={() => setPanel(agent.project, null)} />
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

  .tools {
    display: flex;
    gap: 4px;
  }

  .tools.compact {
    gap: 2px;
  }

  .item {
    position: relative;
    display: flex;
  }

  .customizing {
    padding: 3px;
    border-radius: 6px;
    outline: 1px dashed var(--accent);
    outline-offset: 2px;
  }

  .customizing .item {
    cursor: grab;
  }

  .customizing .item > :global(:not(.eye)) {
    pointer-events: none;
  }

  .customizing .item:global(.dragging) {
    z-index: 2;
    cursor: grabbing;
    filter: drop-shadow(0 6px 14px rgba(0, 0, 0, 0.5));
  }

  .item.off > :global(:not(.eye)) {
    opacity: 0.35;
  }

  .eye {
    position: absolute;
    top: -8px;
    right: -6px;
    z-index: 1;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .tools.compact .btn-ghost {
    width: 32px;
    padding: 0;
    box-shadow: none;
    color: var(--text-muted);
  }

  .top .tools .btn-ghost[aria-pressed='true'] {
    background: var(--surface-5);
    box-shadow: inset 0 0 0 1px var(--text-faint);
    color: var(--text-strong);
  }

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

  .meta {
    position: relative;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    min-width: 0;
  }

  .info {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .info + .info::before {
    content: '·';
    margin: 0 6px;
  }

  .remote {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: -2px -4px;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--accent-soft);
    font-size: 12px;
  }

  .remote:not(.connected) {
    opacity: 0.7;
  }

  .remote:hover {
    background: var(--surface-3);
  }

  .composer {
    position: relative;
  }

  .commands {
    position: absolute;
    bottom: calc(100% - 6px);
    left: 50%;
    z-index: 20;
    width: min(740px, calc(100% - 40px));
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .choices {
    max-height: 300px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 6px;
  }

  .commands button {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }

  .commands button[aria-selected='true'] {
    background: var(--menu-hover);
  }

  .slash {
    flex: none;
    white-space: nowrap;
    font: 13px var(--mono);
    color: var(--text-strong);
  }

  .option {
    flex: none;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .usage {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: baseline;
    gap: 4px 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }

  .usage-hint {
    grid-column: 1 / -1;
    overflow-wrap: anywhere;
    font-size: 12px;
    line-height: 1.5;
    color: var(--accent-soft);
  }

  .hint {
    flex: none;
    white-space: nowrap;
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .tag {
    flex: none;
    margin-left: auto;
    white-space: nowrap;
    font-size: 11.5px;
    color: var(--text-faint);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .keys-help {
    padding: 7px 16px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .keys-help kbd {
    padding: 0 4px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 11px var(--mono);
    color: var(--text-2);
  }


  .settings {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: -2px -4px;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .settings:hover,
  .settings[aria-expanded='true'] {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .settings svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .tuning {
    position: absolute;
    top: 26px;
    left: -4px;
    z-index: 30;
    width: 560px;
    max-width: calc(100vw - 40px);
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    font-size: 13.5px;
    color: var(--text);
  }

  .tuning p {
    margin: 0;
    font-size: 12.5px;
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

  .attached {
    max-width: 740px;
    margin: 0 auto 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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
