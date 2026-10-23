<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, untrack } from 'svelte'
  import { archive, clock, drafts, fork, interrupt, note, reload, remote, rename, reply, send, setOptions } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { changedFiles, type Changes, type Proposal } from './diff'
  import DiffPanel from './DiffPanel.svelte'
  import { ago, cost, meta, modelLabel, time, tokens } from './format'
  import EditorPanel from './EditorPanel.svelte'
  import { layout, setPanel, type PanelKind } from './layout.svelte'
  import { projects, toggleBoard, togglePulls } from './projects.svelte'
  import { ui } from './actions.svelte'
  import { all, detailOf, hidden, labelOf, shown, sortable, toggle as toggleItem } from './layout-slots.svelte'
  import Icon from './Icon.svelte'
  import { hint, settings } from './settings.svelte'
  import SubagentsPanel from './SubagentsPanel.svelte'
  import TerminalPanel from './TerminalPanel.svelte'
  import CommentChip from './CommentChip.svelte'
  import Attachment from './Attachment.svelte'
  import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import OptionsFields from './OptionsFields.svelte'
  import { lookUp, needsYou, openReview, progress, pulls, reviews, stateLabel } from './pulls.svelte'
  import Reviewers from './Reviewers.svelte'
  import { catalogOf, commandsOf } from './catalogs.svelte'
  import { find as findCommand, options, parse, type Command } from './commands'
  import { calls, known, prompt, relative, workdir, type Agent, type Comment, type Options, type Subagent, type ToolCall } from './stream'

  let { agent }: { agent: Agent } = $props()

  let text = $state(
    untrack(() => {
      const draft = drafts[agent.id] ?? ''
      delete drafts[agent.id]
      return draft
    }),
  )
  let menuOpen = $state(false)
  let remoteOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = $state(true)
  let diffFile = $state('')
  let diffSource = $state<'local' | 'pull'>('local')
  let proposal = $state<Proposal | null>(null)

  $effect(() => {
    if (proposal && !agent.asks.some((a) => a.id === proposal?.ask)) proposal = null
  })
  const panel = $derived(layout.panels[agent.project] ?? null)
  const toggle = (kind: PanelKind) => setPanel(agent.project, panel === kind ? null : kind)

  let viewing = $state<string | null>(null)

  function openSubagent(sub: Subagent | null) {
    viewing = sub?.id ?? null
    setPanel(agent.project, 'subagents')
  }
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
    invoke<Changes>('git_changes', { project: workdir(agent) })
      .then((result) => {
        if (run === refreshes) changes = result
      })
      .catch(() => {
        if (run === refreshes) changes = null
      })
  })

  const pull = $derived(pulls[workdir(agent)] ?? null)
  $effect(() => {
    void agent.turns
    lookUp(workdir(agent))
  })

  let comments = $state<Comment[]>([])

  function sendReview(summary: string, list: Comment[]) {
    const message = summary || `Review of pull request #${pull?.number}`
    stuck = true
    setPanel(agent.project, null)
    if (!reply(agent, prompt(message, list))) send(agent.id, message, list)
  }

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

  const branch = $derived(
    agent.worktree ? agent.worktree.branch || agent.worktree.path.split('/').at(-1) : projects.list.find((p) => p.path === agent.project)?.branch,
  )
  const info = $derived(
    shown('agent.info').filter(
      (id) =>
        (id !== 'subagents' || agent.subagents.length) &&
        (id !== 'branch' || branch) &&
        (id !== 'pull' || pull || agent.review) &&
        (id !== 'tasks' || agent.tasks.length) &&
        (id !== 'turns' || agent.turns),
    ),
  )
  const CHIPS = ['remote', 'branch', 'pull', 'subagents']
  const inline = $derived(info.filter((id) => !CHIPS.includes(id)))
  const chips = $derived(info.filter((id) => CHIPS.includes(id)))
  const running = $derived(agent.subagents.filter((s) => s.status === 'running').length)

  type Item = { value: string; label: string; hint?: string; detail?: string; usage?: string; group?: string; text: string }

  const GROUPS = ['Commands', 'Skills', 'Claude Code']

  let dismissed = $state('')
  let pick = $state(0)
  let chosen = $state('')
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
  function groupOf(c: Command) {
    if (c.builtin) return 'Claude Code'
    const skills = agent.skills.length ? agent.skills : known.skills
    return skills.some((s) => s === c.name || s.endsWith(`:${c.name}`)) ? 'Skills' : 'Commands'
  }
  const suggestions = $derived.by((): Item[] => {
    if (!stage) return []
    if (!stage.command) {
      const q = stage.query
      return commandList
        .filter((c) => !c.name.startsWith('_') && !c.description.startsWith('(removed)'))
        .filter((c) => c.name.toLowerCase().includes(q) || c.aliases?.some((a) => a.startsWith(q)))
        .map((c) => ({ value: c.name, label: `/${c.name}`, detail: c.description, usage: c.argumentHint, group: groupOf(c), text: `/${c.name} ` }))
        .sort(
          (a, b) =>
            GROUPS.indexOf(a.group) - GROUPS.indexOf(b.group) ||
            Number(!a.value.startsWith(q)) - Number(!b.value.startsWith(q)) ||
            a.value.localeCompare(b.value),
        )
    }
    const command = stage.command
    return options(command, catalogOf(agent))
      .filter((o) => `${o.value} ${o.label ?? ''}`.toLowerCase().includes(stage.query))
      .map((o) => ({ value: o.value, label: o.label ?? o.value, detail: o.detail, hint: o.label ? o.value : undefined, text: `/${command.name} ${o.value} ` }))
  })
  const sections = $derived(
    suggestions.reduce<{ name?: string; items: { item: Item; i: number }[] }[]>((list, item, i) => {
      if (!list.length || list.at(-1)!.name !== item.group) list.push({ name: item.group, items: [] })
      list.at(-1)!.items.push({ item, i })
      return list
    }, []),
  )
  const guide = $derived(stage?.command && !suggestions.length && !stage.query ? stage.command : null)

  $effect(() => {
    const query = stage?.query ?? ''
    void stage?.command
    pick = query ? Math.max(0, untrack(() => suggestions).findIndex((s) => s.value.toLowerCase().startsWith(query))) : 0
    reveal()
  })

  function reveal() {
    tick().then(() => document.querySelector('.choices [aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }))
  }

  function complete(item: Item) {
    text = chosen = item.text
    dismissed = ''
    composer.focus()
  }

  function local(message: string) {
    const call = parse(message)
    if (!call) return false
    const command = findCommand(commandList, call.name.toLowerCase())
    if (command?.name === 'remote-control') {
      const on = call.args ? call.args === 'on' : !agent.remote
      remote(agent.id, on)
      note(agent.id, message, on ? 'Remote control turned on' : 'Remote control turned off')
      return true
    }
    if (command?.name === 'model' && (catalogOf(agent)?.models ?? []).some((m) => m.value === call.args && m.value !== 'default')) {
      setOptions(agent.id, { ...agent.options, model: call.args })
      note(agent.id, message, `Model set to ${modelLabel(call.args)}`)
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

  let attachments = $state<string[]>([])
  let dropping = $state(false)
  let attachError = $state('')

  const attach = (paths: string[]) => (attachments = [...attachments, ...paths.filter((p) => !attachments.includes(p))])

  async function browseFiles() {
    const chosen = await chooseFiles({ multiple: true })
    if (chosen) attach(Array.isArray(chosen) ? chosen : [chosen])
  }

  function base64(file: Blob) {
    return new Promise<string>((done, fail) => {
      const reader = new FileReader()
      reader.onload = () => done(String(reader.result).split(',')[1] ?? '')
      reader.onerror = () => fail(reader.error)
      reader.readAsDataURL(file)
    })
  }

  let pasting = 0

  async function systemImage() {
    const path = await invoke<string | null>('paste_image').catch((e) => ((attachError = String(e)), null))
    if (path) attach([path])
  }

  function pasteKey(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'v') return
    const mine = ++pasting
    setTimeout(() => mine === pasting && systemImage(), 120)
  }

  async function pasted(event: ClipboardEvent) {
    pasting++
    const data = event.clipboardData
    const images = [...(data?.items ?? [])]
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    if (!images.length) {
      if (!data?.getData('text/plain')) systemImage()
      return
    }
    if (!data?.getData('text/plain')) event.preventDefault()
    attachError = ''
    for (const image of images) {
      const name = image.name && image.name !== 'image.png' ? image.name : `screenshot-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.${image.type.split('/')[1] || 'png'}`
      try {
        attach([await invoke<string>('save_attachment', { name, data: await base64(image) })])
      } catch (e) {
        attachError = String(e)
      }
    }
  }

  $effect(() => {
    let stop: (() => void) | undefined
    let gone = false
    getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        if (payload.type === 'over' || payload.type === 'enter') dropping = true
        else if (payload.type === 'leave') dropping = false
        else if (payload.type === 'drop') {
          dropping = false
          attach(payload.paths)
          composer?.focus()
        }
      })
      .then((unlisten) => (gone ? unlisten() : (stop = unlisten)))
      .catch(() => {})
    return () => {
      gone = true
      stop?.()
    }
  })

  function submit(event?: Event) {
    event?.preventDefault()
    const message = text.trim()
    const attached = $state.snapshot(comments)
    const joined = $state.snapshot(attachments)
    if (!message && !attached.length && !joined.length) return
    text = ''
    recall = -1
    stuck = true
    if (!attached.length && !joined.length && !agent.asks.length && local(message)) return
    comments = []
    if (!reply(agent, prompt(message, attached))) {
      attachments = []
      send(agent.id, message, attached, joined)
    }
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
        reveal()
        return
      }
      if (event.key === 'Tab' || (event.key === 'Enter' && !event.shiftKey && suggestions[pick].text.trim() !== text.trim())) {
        event.preventDefault()
        complete(suggestions[pick])
        return
      }
    }
    if (event.key === 'Backspace' && text === chosen && text.endsWith(' ') && area.selectionStart === text.length) {
      event.preventDefault()
      text = chosen = text.replace(/[^\s/]+ $/, '')
      return
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
    if (!target.closest('[data-remote-menu]')) remoteOpen = false
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
          {#each inline as id (id)}
            <span class="info">
              {#if id === 'options'}
                <button class="settings" aria-haspopup="dialog" aria-expanded={!!tuning} title="Change the model, the effort or the permission mode" onclick={() => (tuning = tuning ? null : { ...agent.options })}>
                  {meta(agent)}
                  <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
              {:else if id === 'cost'}
                <span class="mono">{cost(agent.cost)}</span>
              {:else if id === 'context'}
                <span class="mono">{tokens(agent.tokens)} context tokens</span>
              {:else if id === 'turns'}
                <span class="mono">{agent.turns} turn{agent.turns > 1 ? 's' : ''}</span>
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
      <button class="btn btn-ghost" aria-label="Diff" aria-pressed={panel === 'diff'} title="See the agent’s changes ({hint('diff')})" onclick={() => ((diffSource = 'local'), toggle('diff'))}>
        <Icon name="diff" />
        {#if !panel}
          Diff
          {#if totals.add || totals.del}
            <span class="mono"><span class="add">+{totals.add}</span>{#if totals.del}&nbsp;<span class="del">−{totals.del}</span>{/if}</span>
          {/if}
        {/if}
      </button>
    {/snippet}
    {#snippet pullsTool()}
      <button class="btn btn-ghost" aria-label="Pull requests" title="Pull requests you review and you wrote" onclick={togglePulls}>
        <Icon name="pull" />
        {#if !panel}Pull requests{/if}
        {#if needsYou(agent.project)}<span class="count">{needsYou(agent.project)}</span>{/if}
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
            {@render (id === 'editor' ? editorTool : id === 'terminal' ? terminalTool : id === 'diff' ? diffTool : id === 'pulls' ? pullsTool : kanbanTool)()}
            {#if ui.customizing}
              <button class="eye" data-keep aria-label="{hidden('agent.tools', id) ? 'Show' : 'Hide'} {labelOf('agent.tools', id)}" onclick={() => toggleItem('agent.tools', id)}>
                <Icon name={hidden('agent.tools', id) ? 'eye-off' : 'eye'} size={12} />
              </button>
            {/if}
          </span>
        {/each}
      </div>
      {#if chips.length}
        <div class="context" role="group" aria-label="Session context" style:--project={projects.list.find((p) => p.path === agent.project)?.color}>
          {#each chips as id (id)}
            {#if id === 'remote'}
              <span class="anchor" data-remote-menu>
                <button
                  class="chip {agent.remote ? `on effect-${settings.tab_effect}` : ''}"
                  aria-haspopup="menu"
                  aria-expanded={remoteOpen}
                  title={agent.remote ? 'Remote control is on · this conversation can also be steered from claude.ai' : 'Remote control is off'}
                  onclick={() => (remoteOpen = !remoteOpen)}
                >
                  <span class="ping" class:off={!agent.remote} class:live={agent.remote?.state === 'connected'} aria-hidden="true"></span>
                  Remote control
                  {#if agent.remote?.state === 'connecting'}<span class="state">connecting…</span>{/if}
                  <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
                {#if remoteOpen}
                  <div class="remote-menu" class:short={!agent.remote} role="menu" aria-label="Remote control">
                    {#if agent.remote}
                      <span class="about">
                        <strong>Remote control on</strong>
                        <span
                          >{agent.remote.since ? `Since ${time(agent.remote.since)}. ` : ''}This conversation can be followed and steered from claude.ai or the Claude app.
                          Squadron stays the source: everything shows here too.</span
                        >
                      </span>
                      {#if agent.remote.last}
                        <span class="last"><span class="dot" aria-hidden="true"></span><span class="grow">Last remote message</span>{ago(agent.remote.last, clock.now)}</span>
                      {/if}
                      <button
                        role="menuitem"
                        disabled={!agent.remote.url}
                        onclick={() => (agent.remote?.url && invoke('open_link', { url: agent.remote.url }), (remoteOpen = false))}
                      >
                        Open in claude.ai
                      </button>
                      <button
                        role="menuitem"
                        disabled={!agent.remote.url}
                        onclick={() => (agent.remote?.url && navigator.clipboard.writeText(agent.remote.url), (remoteOpen = false))}
                      >
                        Copy the session link
                      </button>
                      <button role="menuitem" class="stop" onclick={() => (remote(agent.id, false), (remoteOpen = false))}>Stop remote control</button>
                    {:else}
                      <button role="menuitem" onclick={() => (remote(agent.id, true), (remoteOpen = false))}>Turn on remote control</button>
                    {/if}
                  </div>
                {/if}
              </span>
            {:else if id === 'branch'}
              <span
                class="chip branch"
                class:worktree={!!agent.worktree}
                title={agent.worktree ? `Works in the worktree ${agent.worktree.path}` : 'Branch of the project checkout'}
              >
                <Icon name="branch" size={15} /><span class="mono">{branch}</span>
                {#if agent.worktree}<span class="tag">worktree</span>{/if}
              </span>
            {:else if id === 'pull' && agent.review}
              {@const drafts = reviews[`${agent.review.project}#${agent.review.number}`]?.length ?? 0}
              <button
                class="chip pull"
                title="Open the review of pull request #{agent.review.number}, where {agent.name} drops its remarks"
                onclick={() => agent.review && openReview(agent.review.project, agent.review.number)}
              >
                <Icon name="pull" size={15} />Reviewing <span class="mono">#{agent.review.number}</span>
                <span class="fact">{drafts} remark{drafts === 1 ? '' : 's'} in your review</span><span class="go" aria-hidden="true">›</span>
              </button>
            {:else if id === 'pull' && pull}
              <button
                class="chip pull"
                aria-pressed={panel === 'diff' && diffSource === 'pull'}
                title="{pull.title} · review it in the Diff panel"
                onclick={() => ((diffSource = 'pull'), setPanel(agent.project, 'diff'))}
              >
                <Icon name="pull" size={15} /><span class="mono">#{pull.number}</span>
                <span class="state {pull.draft && pull.state === 'open' ? 'draft' : pull.state}">{stateLabel(pull)}</span>
                {#if pull.reviewers.length}<Reviewers reviewers={pull.reviewers} size={20} />{/if}
                {#if pull.changes_requested}<span class="fact bad">Changes requested</span>
                {:else if pull.reviewers.length || pull.approvals}<span class="fact good">{progress(pull)}</span>{/if}
                {#if pull.conflicts}<span class="fact bad">Conflicts</span>{/if}
                {#if pull.checks}<span class="checks {pull.checks}" title="Builds {pull.checks}"></span>{/if}
                {#if pull.comments}<span class="fact">{pull.comments} comment{pull.comments > 1 ? 's' : ''}</span>{/if}
              </button>
            {:else}
              <button
                class="chip subagents {running > 0 ? `effect-${settings.tab_effect}` : ''}"
                aria-pressed={panel === 'subagents'}
                title="{running ? `${running} working · ` : ''}{tokens(agent.subagents.reduce((t, s) => t + s.tokens, 0))} tokens in subagents · {panel === 'subagents'
                  ? 'close'
                  : 'open'} the panel"
                onclick={() => toggle('subagents')}
              >
                <Icon name="subagents" size={16} />{agent.subagents.length} subagent{agent.subagents.length > 1 ? 's' : ''}<span class="go" aria-hidden="true">›</span>
              </button>
            {/if}
          {/each}
        </div>
      {/if}
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
          <Conversation
          {agent}
          {ondiff}
          onfork={(i) => fork(agent.id, i)}
          onsubagent={openSubagent}
          onsubagents={() => openSubagent(null)}
          onpreview={(p) => ((proposal = p), openDiff(p.path))}
        />
        </div>
      </div>

      {#if !stuck}
        <button class="latest" onclick={bottom}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M6 13l6 6 6-6" /></svg>
          Latest message{#if below}<span>&nbsp;· {below} message{below > 1 ? 's' : ''} below</span>{/if}
        </button>
      {/if}
    </div>

    {#if dropping}<div class="dropzone" aria-hidden="true">Drop to attach to your message to {agent.name}</div>{/if}
    <form class="composer" onsubmit={submit}>
      {#if attachments.length || attachError}
        <div class="attached files" aria-label="Files sent with your message">
          {#each attachments as file, i (file)}<Attachment path={file} onremove={() => attachments.splice(i, 1)} />{/each}
          {#if attachError}<span class="attach-error" role="alert">{attachError}</span>{/if}
        </div>
      {/if}
      {#if comments.length}
        <div class="attached" aria-label="Comments on the diff, sent with your message">
          {#each comments as comment, i (i)}
            <CommentChip
              {comment}
              onopen={() => ((target = comment), (diffSource = comment.pull ? 'pull' : 'local'), openDiff(comment.path))}
              onremove={() => comments.splice(i, 1)}
            />
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
            {#each sections as section (section.name ?? '')}
              <div class="group" role="group" aria-label={section.name}>
                {#if section.name}
                  <div class="section" class:current={suggestions[pick]?.group === section.name} role="presentation">{section.name}</div>
                {/if}
                {#each section.items as { item, i } (item.value)}
                  <button
                    type="button"
                    role="option"
                    aria-selected={i === pick}
                    onmousemove={() => (pick = i)}
                    onmousedown={(e) => e.preventDefault()}
                    onclick={() => complete(item)}
                  >
                    <span class="line">
                      <span class={stage?.command ? 'option' : 'slash'}>{item.label}</span>
                      {#if item.hint}<span class="hint">{item.hint}</span>{/if}
                      {#if i === pick && item.usage}<code class="syntax">{item.usage}</code>{/if}
                    </span>
                    {#if i === pick && item.detail}<span class="about">{item.detail}</span>{/if}
                  </button>
                {/each}
              </div>
            {/each}
          </div>
          <div class="keys-help" role="presentation">
            {#if suggestions.length}<kbd>↑</kbd><kbd>↓</kbd> to move · <kbd>Tab</kbd> or <kbd>Enter</kbd> to choose · {/if}{#if text === chosen && text.endsWith(' ')}<kbd>⌫</kbd> to go back · {/if}<kbd>Esc</kbd> to close
          </div>
        </div>
      {/if}
      <div class="bar">
        <label for="composer" class="sr-only">Message to the agent</label>
        <textarea id="composer" rows="1" bind:value={text} bind:this={composer} {placeholder} onkeydown={(e) => (pasteKey(e), keys(e))} onpaste={pasted}></textarea>
        <button type="button" class="btn-icon" aria-label="Attach files" title="Attach files or screenshots, or paste and drop them here" onclick={browseFiles}>
          <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            ><path d="M21 11.5l-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6l7.9-7.9" /></svg
          >
        </button>
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
        <button class="btn btn-primary send" disabled={!text.trim() && !comments.length && !attachments.length}>
          {pending === 'AskUserQuestion' ? 'Answer' : pending === 'ExitPlanMode' ? 'Send feedback' : pending ? 'Deny' : 'Send'}
        </button>
      </div>
    </form>
  </section>

  {#if panel === 'diff'}
    <DiffPanel
      {agent}
      {changes}
      {proposal}
      {pull}
      bind:mode={diffSource}
      onreview={sendReview}
      onhand={(quote) => ((text = text.trim() ? `${text.trimEnd()}\n\n${quote}` : quote), composer.focus())}
      bind:file={diffFile}
      bind:comments
      bind:target
      onclose={() => setPanel(agent.project, null)}
    />
  {:else if panel === 'editor'}
    <EditorPanel {agent} {changes} onclose={() => setPanel(agent.project, null)} />
  {:else if panel === 'terminal'}
    <TerminalPanel {agent} onsend={excerpt} onclose={() => setPanel(agent.project, null)} />
  {:else if panel === 'subagents' && agent.subagents.length}
    <SubagentsPanel
      {agent}
      {viewing}
      onview={(id) => (viewing = id)}
      onprefill={(message) => ((text = message), composer.focus())}
      onquote={(quote) => ((text = text.trim() ? `${text.trimEnd()} ${quote} ` : `${quote} `), composer.focus())}
      onclose={() => setPanel(agent.project, null)}
    />
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
    position: relative;
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
    min-height: 60px;
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 12px;
    padding: 10px 20px;
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
    row-gap: 4px;
    min-width: 0;
  }

  .info {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .info:not(:last-of-type)::after {
    content: '·';
    margin: 0 6px;
  }

  .context {
    flex-basis: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }

  .context .anchor {
    position: relative;
    display: inline-flex;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--menu);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-muted);
    font-size: 14px;
    font-weight: 500;
  }

  .chip:hover,
  .chip[aria-pressed='true'],
  .chip[aria-expanded='true'] {
    background: var(--surface-5);
  }

  .chip.on,
  .chip.subagents {
    color: var(--text-strong);
    font-weight: 600;
  }

  .chip.effect-snake {
    box-shadow: none;
  }

  .chip.on {
    --project: var(--remote-dot);
  }

  .chip.branch {
    gap: 8px;
    cursor: default;
  }

  .chip.branch:hover {
    background: var(--menu);
  }

  .count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font: 700 11px/18px var(--sans);
    text-align: center;
  }

  .chip.pull {
    gap: 8px;
  }

  .chip.pull .state {
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
  }

  .chip.pull .state.open {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .chip.pull .state.merged {
    background: rgba(185, 163, 240, 0.16);
    color: #cdbcf7;
  }

  .chip.pull :is(.state.declined, .state.closed) {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .chip.pull .fact {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .chip.pull .fact.good {
    color: var(--ok-text);
  }

  .chip.pull .fact.bad {
    color: var(--err-text);
  }

  .checks {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .checks.passing {
    background: var(--ok);
  }

  .checks.failing {
    background: var(--err);
  }

  .chip.worktree,
  .chip.worktree:hover {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--ok-text);
  }

  .chip.worktree :global(svg) {
    stroke: var(--ok);
  }

  .tag {
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--ok);
    color: var(--ink);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .chip svg {
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 2.2;
    stroke-linecap: round;
  }

  .ping {
    position: relative;
    width: 8px;
    height: 8px;
    border-radius: 4px;
    background: var(--remote-dot);
  }

  .ping.off {
    background: var(--text-faint);
  }

  .ping.live::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 4px;
    background: var(--remote-dot);
    animation: ping 1.8s ease-out infinite;
  }

  .state {
    font-weight: 500;
    color: var(--remote-text);
  }

  .remote-menu {
    position: absolute;
    top: 42px;
    left: 0;
    z-index: 20;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 14px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 16px 40px rgba(0, 0, 0, 0.5);
  }

  .remote-menu.short {
    width: 220px;
    padding: 6px;
  }

  .remote-menu .about {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-bottom: 4px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .remote-menu strong {
    font-size: 13.5px;
    color: var(--text);
  }

  .remote-menu .last {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: 4px;
    background: var(--bg-main);
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .remote-menu .last .dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--remote-dot);
  }

  .remote-menu .last .grow {
    flex: 1;
    color: var(--text);
  }

  .remote-menu button {
    height: 32px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    text-align: left;
  }

  .remote-menu button:disabled {
    opacity: 0.5;
  }

  .remote-menu .stop {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
    font-weight: 600;
  }

  .go {
    font-size: 16px;
    color: var(--text-muted);
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
    max-height: 360px;
    overflow-y: auto;
    padding: 0 6px 6px;
    scroll-padding-top: 30px;
  }

  .group + .group {
    margin-top: 4px;
  }

  .section {
    position: sticky;
    top: 0;
    z-index: 1;
    margin: 0 -6px;
    padding: 10px 16px 6px;
    background: var(--menu);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .section.current {
    color: var(--accent-soft);
  }

  .commands button {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    width: 100%;
    min-width: 0;
    min-height: 34px;
    padding: 7px 10px;
    border-radius: 4px;
    text-align: left;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
  }

  .syntax {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12px;
    color: var(--accent-soft);
  }

  .about {
    display: -webkit-box;
    overflow: hidden;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    font-size: 12.5px;
    line-height: 18px;
    color: var(--text-muted);
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
    background: var(--surface-2);
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
    align-items: center;
    gap: 6px;
  }



  .attached.files {
    gap: 12px;
    padding-top: 8px;
  }

  .attach-error {
    flex-basis: 100%;
    font-size: 12.5px;
    color: var(--err-text);
  }

  .dropzone {
    position: absolute;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--accent);
    border-radius: 8px;
    background: rgba(26, 21, 19, 0.82);
    color: var(--text-strong);
    font-size: 16px;
    font-weight: 600;
    pointer-events: none;
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
