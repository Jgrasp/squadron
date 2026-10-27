<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, untrack } from 'svelte'
  import { archive, clock, drafts, fork, interrupt, keepDraft, markUnread, note, reload, remote, rename, reply, send, setOptions, onScreen, persist } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import { changedFiles, type Changes, type Proposal } from './diff'
  import DiffPanel from './DiffPanel.svelte'
  import { ago, effortLabel, modeLabel, modelLabel, time, tokens } from './format'
  import EditorPanel from './EditorPanel.svelte'
  import InfoPanel from './InfoPanel.svelte'
  import ContextGauge from './ContextGauge.svelte'
  import { layout, setPanel, type PanelKind } from './layout.svelte'
  import { projects, toggleBoard, togglePulls } from './projects.svelte'
  import { ui } from './actions.svelte'
  import { all, detailOf, hidden, labelOf, place, shown, sortable, toggle as toggleItem } from './layout-slots.svelte'
  import Icon from './Icon.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'
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

  const seen = $derived(onScreen(agent))
  $effect(() => {
    if (seen) untrack(() => markUnread(agent.id, false))
  })
  $effect(() => {
    if (seen && agent.status === 'error' && !agent.errorSeen) untrack(() => ((agent.errorSeen = true), persist(agent)))
  })

  const saved = untrack(() => drafts[agent.id])
  let text = $state(saved?.text ?? '')
  let menuOpen = $state(false)
  let remoteOpen = $state(false)
  let scroller: HTMLElement
  let column: HTMLElement
  let stuck = $state(true)
  let diffFile = $state('')
  let diffSource = $state<'local' | 'pull' | 'branch'>('local')
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
  const WINDOW = 20
  const users = $derived(agent.entries.flatMap((e, i) => (e.kind === 'user' ? [{ entry: i, text: prompt(e.text, e.comments), at: e.at }] : [])))
  const floor = $derived(agent.entries.findLastIndex((e) => e.kind === 'compacted'))
  const foldedCount = $derived(floor > 0 ? agent.entries.slice(0, floor).filter((e) => e.kind === 'user' || e.kind === 'agent').length : 0)
  let unfolded = $state(false)
  let start = $state(
    untrack(() => Math.max(users.length > WINDOW ? users[users.length - WINDOW].entry : 0, agent.entries.findLastIndex((e) => e.kind === 'compacted'))),
  )
  const folded = $derived(!unfolded && floor > 0 && start <= floor)
  let offsets = $state<number[]>([])
  let railHeight = $state(0)
  const buckets = $derived.by(() => {
    const n = users.length
    const count = n * 10 <= railHeight ? n : Math.max(1, Math.floor(railHeight / 16))
    return n ? Array.from({ length: count }, (_, b) => ({ from: Math.floor((b * n) / count), to: Math.floor(((b + 1) * n) / count) - 1 })) : []
  })
  let earlier = $state<HTMLElement>()
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

  let comments = $state<Comment[]>(saved?.comments ?? [])

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
  const groupName = (group: string) => (group === 'Commands' ? t('agent.commands') : group === 'Skills' ? t('agent.skills') : group)

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
      note(agent.id, message, on ? t('agent.remoteTurnedOn') : t('agent.remoteTurnedOff'))
      return true
    }
    if (command?.name === 'model' && (catalogOf(agent)?.models ?? []).some((m) => m.value === call.args && m.value !== 'default')) {
      setOptions(agent.id, { ...agent.options, model: call.args })
      note(agent.id, message, t('agent.modelSet', { model: modelLabel(call.args) }))
      return true
    }
    if (command?.name === 'effort' && ['low', 'medium', 'high', 'xhigh', 'max'].includes(call.args)) {
      setOptions(agent.id, { ...agent.options, effort: call.args as Options['effort'] })
      note(agent.id, message, t('agent.effortSet', { effort: call.args }))
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
      ? t('agent.freeAnswer', { agent: agent.name })
      : pending === 'ExitPlanMode'
        ? t('agent.planChange')
        : pending
          ? t('agent.denyWith', { agent: agent.name })
          : agent.status === 'working' && !agent.held
            ? t('agent.addInstruction')
            : agent.status === 'interrupted'
              ? t('agent.newInstruction')
              : agent.status === 'error'
                ? t('agent.writeReload', { agent: agent.name })
                : t('agent.writeTo', { agent: agent.name }),
  )

  function measure() {
    const placed = new Map([...column.querySelectorAll<HTMLElement>('[data-user]')].map((el) => [Number(el.dataset.user), el.offsetTop]))
    offsets = users.map((u) => placed.get(u.entry) ?? -1)
    const view = scroller.scrollTop + scroller.clientHeight / 3
    const end = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 4
    const first = offsets.findIndex((o) => o >= 0)
    active = picked >= 0 ? picked : end ? users.length - 1 : scroller.scrollTop < 4 ? first : Math.max(first, offsets.findLastIndex((o) => o >= 0 && o <= view))
    below = offsets.filter((o) => o > scroller.scrollTop + scroller.clientHeight).length
  }

  async function showEarlier() {
    const first = users.findIndex((u) => u.entry >= start)
    const from = first - WINDOW
    const height = scroller.scrollHeight - scroller.scrollTop
    start = Math.max(from > 0 ? users[from].entry : 0, unfolded ? 0 : floor)
    await tick()
    scroller.scrollTop = scroller.scrollHeight - height
  }

  $effect(() => {
    if (!earlier) return
    const observer = new IntersectionObserver((seen) => seen.some((s) => s.isIntersecting) && showEarlier(), { root: scroller, rootMargin: '800px 0px 0px 0px' })
    observer.observe(earlier)
    return () => observer.disconnect()
  })

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

  async function jump(i: number) {
    stuck = false
    touched = Date.now()
    picked = active = i
    if (users[i].entry < start) {
      if (users[i].entry < floor) unfolded = true
      start = users[i].entry
      await tick()
    }
    const el = column.querySelector<HTMLElement>(`[data-user="${users[i].entry}"]`)
    if (el) scroller.scrollTo({ top: Math.max(0, el.offsetTop - 18), behavior: 'smooth' })
  }

  function hover(bucket: { from: number; to: number }, event: Event) {
    const target = event.currentTarget as HTMLElement
    const mark = target.getBoundingClientRect()
    const rail = target.closest('nav')!.getBoundingClientRect()
    const y = event instanceof MouseEvent ? Math.min(Math.max(event.clientY, mark.top), mark.bottom - 1) : mark.top + mark.height / 2
    hovered = event instanceof MouseEvent ? bucket.from + Math.floor(((y - mark.top) / mark.height) * (bucket.to - bucket.from + 1)) : bucket.from
    tooltipTop = y - rail.top
  }

  function bottom() {
    stuck = true
    picked = -1
    scroller.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' })
  }

  let attachments = $state<string[]>(saved?.files ?? [])

  $effect(() => {
    keepDraft(agent.id, { text, files: $state.snapshot(attachments), comments: $state.snapshot(comments) })
  })
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
    const path = await invoke<string | null>('paste_image').catch((e) => ((attachError = errorText(e)), null))
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
        attachError = errorText(e)
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

  let chilly = $state('')

  function submit(event?: Event, forced = false) {
    event?.preventDefault()
    const message = text.trim()
    const attached = $state.snapshot(comments)
    const joined = $state.snapshot(attachments)
    if (!message && !attached.length && !joined.length) return
    if (!forced && !busy && !agent.asks.length && !message.startsWith('/') && agent.tokens >= 200_000 && Date.now() - agent.since > 3_600_000) {
      chilly = agent.id
      return
    }
    chilly = ''
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
  <section class="conversation" aria-label={t('agent.conversation')}>
    <div class="top">
      <div class="title">
        {#if renaming}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="rename"
            aria-label={t('agent.agentName')}
            bind:value={newName}
            autofocus
            onkeydown={(e) => (e.key === 'Enter' ? finishRename(true) : e.key === 'Escape' && finishRename(false))}
            onblur={() => finishRename(true)}
          />
        {:else}
          <h1><button class="name" title={t('common.rename')} onclick={startRename}>{agent.name}</button></h1>
        {/if}
        <span class="meta" data-options>
          {#each inline as id (id)}
            <span class="info">
              {#if id === 'options'}
                <button class="settings" aria-haspopup="dialog" aria-expanded={!!tuning} title={t('agent.changeOptions')} onclick={() => (tuning = tuning ? null : { ...agent.options })}>
                  <span class="model">{modelLabel(agent.options.model)} · {effortLabel(agent.options.effort)}</span>
                  {modeLabel(agent.options.mode)}
                  <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
              {:else if id === 'context'}
                <ContextGauge {agent} />
              {:else if id === 'turns'}
                <span class="mono">{t('agent.turns', { count: agent.turns })}</span>
              {:else if id === 'tasks'}
                <span>{t('agent.tasksDone', { done: agent.tasks.filter((task) => task.status === 'completed').length, total: agent.tasks.length })}</span>
              {/if}
            </span>
          {/each}
          {#if tuning}
            <div class="tuning" role="dialog" aria-label={t('agent.agentOptions')}>
              <OptionsFields bind:options={tuning} />
              <p>{t('agent.optionsHint')}</p>
            </div>
          {/if}
        </span>
      </div>
      <div class="tools" class:compact={panel} class:customizing={ui.customizing} role="group" aria-label={t('agent.openInTool')} use:sortable={{ enabled: true, onplace: (id, index) => place('agent.tools', id, index, ui.customizing ? all('agent.tools') : shown('agent.tools')) }}>
    {#snippet editorTool()}
      <button class="btn btn-ghost" aria-label={t('agent.editor')} aria-pressed={panel === 'editor'} title={t('agent.editorTitle', { shortcut: hint('editor') })} onclick={() => toggle('editor')}>
        <Icon name="editor" />
        {#if !panel}{t('agent.editor')}{/if}
      </button>
    {/snippet}
    {#snippet terminalTool()}
      <button class="btn btn-ghost" aria-label={t('agent.terminal')} aria-pressed={panel === 'terminal'} title={t('agent.terminalTitle', { shortcut: hint('terminal') })} onclick={() => toggle('terminal')}>
        <Icon name="terminal" />
        {#if !panel}{t('agent.terminal')}{/if}
      </button>
    {/snippet}
    {#snippet diffTool()}
      <button class="btn btn-ghost" aria-label={t('agent.diff')} aria-pressed={panel === 'diff'} title={t('agent.diffTitle', { shortcut: hint('diff') })} onclick={() => ((diffSource = 'local'), toggle('diff'))}>
        <Icon name="diff" />
        {#if !panel}
          {t('agent.diff')}
          {#if totals.add || totals.del}
            <span class="mono"><span class="add">+{totals.add}</span>{#if totals.del}&nbsp;<span class="del">−{totals.del}</span>{/if}</span>
          {/if}
        {/if}
      </button>
    {/snippet}
    {#snippet pullsTool()}
      <button class="btn btn-ghost" aria-label={t('agent.pullRequests')} title={t('agent.pullsTitle')} onclick={togglePulls}>
        <Icon name="pull" />
        {#if !panel}{t('agent.pullRequests')}{/if}
        {#if needsYou(agent.project)}<span class="count">{needsYou(agent.project)}</span>{/if}
      </button>
    {/snippet}
    {#snippet kanbanTool()}
      <button class="btn btn-ghost" aria-label={t('agent.kanban')} title={t('agent.kanbanTitle', { shortcut: hint('kanban') })} onclick={toggleBoard}>
        <Icon name="kanban" />
        {#if !panel}{t('agent.kanban')}{/if}
      </button>
    {/snippet}
      {#each ui.customizing ? all('agent.tools') : shown('agent.tools') as id (id)}
          <span class="item" class:off={hidden('agent.tools', id)} data-item={id}>
            {@render (id === 'editor' ? editorTool : id === 'terminal' ? terminalTool : id === 'diff' ? diffTool : id === 'pulls' ? pullsTool : kanbanTool)()}
            {#if ui.customizing}
              <button class="eye" data-keep aria-label={t(hidden('agent.tools', id) ? 'agent.showItem' : 'agent.hideItem', { item: labelOf('agent.tools', id) })} onclick={() => toggleItem('agent.tools', id)}>
                <Icon name={hidden('agent.tools', id) ? 'eye-off' : 'eye'} size={12} />
              </button>
            {/if}
          </span>
        {/each}
      </div>
      {#if chips.length}
        <div
          class="context"
          role="group"
          aria-label={t('agent.sessionContext')}
          style:--project={projects.list.find((p) => p.path === agent.project)?.color}
          use:sortable={{ enabled: chips.length > 1, onplace: (id, index) => place('agent.info', id, index, chips) }}
        >
          {#each chips as id (id)}
            {#if id === 'remote'}
              <span class="anchor" data-remote-menu data-item={id}>
                <button
                  class="chip {agent.remote ? `on effect-${settings.tab_effect}` : ''}"
                  aria-haspopup="menu"
                  aria-expanded={remoteOpen}
                  title={agent.remote ? t('agent.remoteOnTitle') : t('agent.remoteOffTitle')}
                  onclick={() => (remoteOpen = !remoteOpen)}
                >
                  <span class="ping" class:off={!agent.remote} class:live={agent.remote?.state === 'connected'} aria-hidden="true"></span>
                  {t('agent.remoteControl')}
                  {#if agent.remote?.state === 'connecting'}<span class="state">{t('agent.connecting')}</span>{/if}
                  <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
                {#if remoteOpen}
                  <div class="remote-menu" class:short={!agent.remote} role="menu" aria-label={t('agent.remoteControl')} data-keep>
                    {#if agent.remote}
                      <span class="about">
                        <strong>{t('agent.remoteOn')}</strong>
                        <span>{agent.remote.since ? t('agent.remoteAboutSince', { time: time(agent.remote.since) }) : t('agent.remoteAbout')}</span>
                      </span>
                      {#if agent.remote.last}
                        <span class="last"><span class="dot" aria-hidden="true"></span><span class="grow">{t('agent.lastRemoteMessage')}</span>{ago(agent.remote.last, clock.now)}</span>
                      {/if}
                      <button
                        role="menuitem"
                        disabled={!agent.remote.url}
                        onclick={() => (agent.remote?.url && invoke('open_link', { url: agent.remote.url }), (remoteOpen = false))}
                      >
                        {t('agent.openInClaudeAi')}
                      </button>
                      <button
                        role="menuitem"
                        disabled={!agent.remote.url}
                        onclick={() => (agent.remote?.url && navigator.clipboard.writeText(agent.remote.url), (remoteOpen = false))}
                      >
                        {t('agent.copySessionLink')}
                      </button>
                      <button role="menuitem" class="stop" onclick={() => (remote(agent.id, false), (remoteOpen = false))}>{t('agent.stopRemote')}</button>
                    {:else}
                      <button role="menuitem" onclick={() => (remote(agent.id, true), (remoteOpen = false))}>{t('agent.turnOnRemote')}</button>
                    {/if}
                  </div>
                {/if}
              </span>
            {:else if id === 'branch'}
              <span
                class="chip branch"
                data-item={id}
                class:worktree={!!agent.worktree}
                title={agent.worktree ? t('agent.worktreeTitle', { path: agent.worktree.path }) : t('agent.branchTitle')}
              >
                <Icon name="branch" size={15} /><span class="mono">{branch}</span>
                {#if agent.worktree}<span class="tag">{t('agent.worktreeTag')}</span>{/if}
              </span>
            {:else if id === 'pull' && agent.review}
              {@const drafts = reviews[`${agent.review.project}#${agent.review.number}`]?.length ?? 0}
              <button
                class="chip pull"
                data-item={id}
                title={t('agent.reviewTitle', { number: String(agent.review.number), agent: agent.name })}
                onclick={() => agent.review && openReview(agent.review.project, agent.review.number)}
              >
                <Icon name="pull" size={15} /><Trans k="agent.reviewing">{#snippet number()}<span class="mono">#{agent.review?.number}</span>{/snippet}</Trans>
                <span class="fact">{t('agent.remarksInReview', { count: drafts })}</span><span class="go" aria-hidden="true">›</span>
              </button>
            {:else if id === 'pull' && pull}
              <button
                class="chip pull"
                data-item={id}
                aria-pressed={panel === 'diff' && diffSource === 'pull'}
                title={t('agent.pullTitle', { title: pull.title })}
                onclick={() => ((diffSource = 'pull'), setPanel(agent.project, 'diff'))}
              >
                <Icon name="pull" size={15} /><span class="mono">#{pull.number}</span>
                <span class="state {pull.draft && pull.state === 'open' ? 'draft' : pull.state}">{stateLabel(pull)}</span>
                {#if pull.reviewers.length}<Reviewers reviewers={pull.reviewers} size={20} />{/if}
                {#if pull.changes_requested}<span class="fact bad">{t('agent.changesRequested')}</span>
                {:else if pull.reviewers.length || pull.approvals}<span class="fact good">{progress(pull)}</span>{/if}
                {#if pull.conflicts}<span class="fact bad">{t('agent.conflicts')}</span>{/if}
                {#if pull.checks}<span class="checks {pull.checks}" title={t(pull.checks === 'passing' ? 'agent.buildsPassing' : pull.checks === 'failing' ? 'agent.buildsFailing' : 'agent.buildsPending')}></span>{/if}
                {#if pull.comments}<span class="fact">{t('agent.comments', { count: pull.comments })}</span>{/if}
              </button>
            {:else}
              <button
                class="chip subagents {running > 0 ? `effect-${settings.tab_effect}` : ''}"
                aria-pressed={panel === 'subagents'}
                title={t(
                  running
                    ? panel === 'subagents'
                      ? 'agent.subagentsWorkingClose'
                      : 'agent.subagentsWorkingOpen'
                    : panel === 'subagents'
                      ? 'agent.subagentsClose'
                      : 'agent.subagentsOpen',
                  { running, tokens: tokens(agent.subagents.reduce((sum, s) => sum + s.tokens, 0)) },
                )}
                data-item={id}
                onclick={() => toggle('subagents')}
              >
                <Icon name="subagents" size={16} />{t('agent.subagents', { count: agent.subagents.length })}{#if running}<span class="live">{t('conversation.countRunning', { count: running })}</span>{/if}<span class="go" aria-hidden="true">›</span>
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
    {#if agent.infoPanel}<InfoPanel {agent} />{/if}

    <div class="middle">
      {#if users.length}
        <nav class="rail" aria-label={t('agent.landmarks')}>
          <div class="marks" class:grouped={buckets.length < users.length} bind:clientHeight={railHeight}>
            {#each buckets as bucket, i (i)}
              <button
                aria-label={t('agent.goToMessage', { n: bucket.from + 1, text: users[bucket.from].text.slice(0, 80) })}
                aria-current={active >= bucket.from && active <= bucket.to ? 'true' : undefined}
                onclick={() => jump(hovered >= bucket.from && hovered <= bucket.to ? hovered : bucket.from)}
                onmousemove={(e) => hover(bucket, e)}
                onmouseleave={() => (hovered = -1)}
                onfocus={(e) => hover(bucket, e)}
                onblur={() => (hovered = -1)}
              >
                <span></span>
              </button>
            {/each}
          </div>
          {#if users[hovered]}
            <div class="tooltip" role="tooltip" style:top="{tooltipTop}px">
              <small>{t('agent.messageOf', { n: hovered + 1, total: users.length, time: time(users[hovered].at) })}</small>
              <span>{users[hovered].text}</span>
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
          {#if folded}
            {@const mark = agent.entries[floor]}
            <button class="earlier folded" onclick={() => ((unfolded = true), showEarlier())}>
              {t('agent.beforeCompaction', { count: foldedCount, when: mark.kind === 'compacted' ? new Date(mark.at).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }) : '' })}
              <span>{t('agent.showThem')}</span>
            </button>
          {:else if start > 0}
            <button class="earlier" bind:this={earlier} onclick={showEarlier}>{t('agent.earlierMessages')}</button>
          {/if}
          <Conversation
          {agent}
          start={folded ? floor : start}
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
          {t('agent.latestMessage')}{#if below}<span>&nbsp;· {t('agent.messagesBelow', { count: below })}</span>{/if}
        </button>
      {/if}
    </div>

    {#if dropping}<div class="dropzone" aria-hidden="true">{t('agent.dropToAttach', { agent: agent.name })}</div>{/if}
    <form class="composer" onsubmit={submit}>
      {#if chilly === agent.id}
        <div class="chilly" role="alert">
          <p><strong>{t('agent.coldTitle', { tokens: tokens(agent.tokens), idle: ago(agent.since, Date.now()) })}</strong> {t('agent.coldDetail')}</p>
          <div class="actions">
            <button type="button" class="btn btn-primary" onclick={async () => (await send(agent.id, '/compact'), submit(undefined, true))}>{t('agent.compactThenSend')}</button>
            <button type="button" class="btn btn-secondary" onclick={() => submit(undefined, true)}>{t('agent.sendAnyway')}</button>
            <button type="button" class="close" aria-label={t('common.cancel')} title={t('common.cancel')} onclick={() => (chilly = '')}>×</button>
          </div>
        </div>
      {/if}
      {#if attachments.length || attachError}
        <div class="attached files" aria-label={t('agent.filesSent')}>
          {#each attachments as file, i (file)}<Attachment path={file} onremove={() => attachments.splice(i, 1)} />{/each}
          {#if attachError}<span class="attach-error" role="alert">{attachError}</span>{/if}
        </div>
      {/if}
      {#if comments.length}
        <div class="attached" aria-label={t('agent.commentsSent')}>
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
        <div class="commands" role="listbox" aria-label={stage?.command ? t('agent.argumentsOf', { command: stage.command.name }) : t('agent.commands')}>
          {#if stage?.command}
            <div class="usage" role="presentation">
              <span class="slash">/{stage.command.name}</span>
              <span class="what">{stage.command.description}</span>
              {#if stage.command.argumentHint}<code class="usage-hint">{stage.command.argumentHint}</code>{/if}
            </div>
          {/if}
          <div class="choices">
            {#each sections as section (section.name ?? '')}
              <div class="group" role="group" aria-label={section.name && groupName(section.name)}>
                {#if section.name}
                  <div class="section" class:current={suggestions[pick]?.group === section.name} role="presentation">{groupName(section.name)}</div>
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
            {#if suggestions.length}<Trans k="agent.keysMove">{#snippet up()}<kbd>↑</kbd>{/snippet}{#snippet down()}<kbd>↓</kbd>{/snippet}</Trans>{' · '}<Trans k="agent.keysChoose"
                >{#snippet tab()}<kbd>Tab</kbd>{/snippet}{#snippet enter()}<kbd>{t('agent.keyEnter')}</kbd>{/snippet}</Trans
              >{' · '}{/if}{#if text === chosen && text.endsWith(' ')}<Trans k="agent.keysBack">{#snippet key()}<kbd>⌫</kbd>{/snippet}</Trans>{' · '}{/if}<Trans k="agent.keysClose"
              >{#snippet key()}<kbd>{t('agent.keyEsc')}</kbd>{/snippet}</Trans
            >
          </div>
        </div>
      {/if}
      <div class="bar">
        <label for="composer" class="sr-only">{t('agent.messageToAgent')}</label>
        <textarea id="composer" rows="1" bind:value={text} bind:this={composer} {placeholder} onkeydown={(e) => (pasteKey(e), keys(e))} onpaste={pasted}></textarea>
        <button type="button" class="btn-icon" aria-label={t('agent.attachFiles')} title={t('agent.attachTitle')} onclick={browseFiles}>
          <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            ><path d="M21 11.5l-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6l7.9-7.9" /></svg
          >
        </button>
        {#if busy}
          <button type="button" class="btn-icon" aria-label={t('agent.interrupt')} title={t('agent.interruptTitle', { shortcut: hint('interrupt') })} onclick={() => interrupt(agent.id)}>
            <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3" fill="currentColor" /></svg>
          </button>
        {/if}
        <div class="anchor" data-agent-menu>
          <button
            type="button"
            class="btn-icon"
            class:open={menuOpen}
            aria-label={t('agent.moreActions')}
            title={t('agent.moreActions')}
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            onclick={() => (menuOpen = !menuOpen)}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor"
              ><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg
            >
          </button>
          {#if menuOpen}
            <div class="menu" role="menu" aria-label={t('agent.agentActions')}>
              {#each shown('agent.menu') as id (id)}
                {#if id === 'archive'}<span class="rule" aria-hidden="true"></span>{/if}
                <button type="button" role="menuitem" disabled={id === 'fork' && !agent.sessionId} onclick={() => ((menuOpen = false), MENU[id]())}>
                  <strong>{id === 'remote' && agent.remote ? t('agent.turnOffRemote') : labelOf('agent.menu', id)}</strong>
                  <span>{id === 'remote' && agent.remote ? t('agent.turnOffRemoteDetail') : detailOf('agent.menu', id)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
        <button class="btn btn-primary send" disabled={!text.trim() && !comments.length && !attachments.length}>
          {pending === 'AskUserQuestion' ? t('agent.answer') : pending === 'ExitPlanMode' ? t('agent.sendFeedback') : pending ? t('agent.deny') : t('common.send')}
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
    <EditorPanel project={agent.project} {changes} onclose={() => setPanel(agent.project, null)} />
  {:else if panel === 'terminal'}
    <TerminalPanel dir={workdir(agent)} to={agent.name} onsend={excerpt} onclose={() => setPanel(agent.project, null)} />
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

  .tools .item:global(.dragging),
  .context > :global(.dragging) {
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

  .marks.grouped button {
    flex: 1 1 0;
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

  .chip.subagents .live {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--sub-running);
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

  .settings .model {
    white-space: nowrap;
    padding: 1px 7px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-strong);
    font-weight: 600;
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

  .earlier {
    align-self: center;
    padding: 6px 14px;
    border-radius: 14px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .earlier.folded {
    display: inline-flex;
    gap: 10px;
    border: 1px dashed var(--border-dashed);
  }

  .earlier.folded span {
    color: var(--accent-link);
  }

  .earlier:hover {
    background: var(--surface-1);
    color: var(--text);
  }

  .composer {
    flex: none;
    padding: 12px 20px 18px;
    border-top: 1px solid var(--border);
  }

  .chilly {
    max-width: 740px;
    margin: 0 auto 8px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
    padding: 10px 10px 10px 14px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    font-size: 13px;
    color: var(--wait-text-2);
  }

  .chilly p {
    flex: 1 1 320px;
    margin: 0;
    line-height: 1.45;
  }

  .chilly strong {
    color: var(--wait-text);
  }

  .chilly .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chilly .btn {
    height: 30px;
    padding: 0 12px;
  }

  .chilly .close {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    font-size: 18px;
    line-height: 1;
    color: var(--wait-muted);
  }

  .chilly .close:hover {
    background: var(--wait-btn);
    color: var(--wait-text);
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
