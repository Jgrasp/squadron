<script lang="ts">
  import { copyText } from './clipboard'
  import { ticketBlocks } from './tickets.svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { clock, forkPoint, SSH_KEY, SUDO, VALUES } from './agents.svelte'
  import { namesOf } from './catalogs.svelte'
  import Attachment from './Attachment.svelte'
  import CommentChip from './CommentChip.svelte'
  import type { Proposal } from './diff'
  import { highlight } from './commands'
  import { splitUrls, urlsIn } from './urls'
  import LinkCard from './LinkCard.svelte'
  import LinkChip from './LinkChip.svelte'
  import PullCard from './PullCard.svelte'
  import { isPullLink } from './linked.svelte'
import { fences } from './codepaste'
  import ErrorCard from './ErrorCard.svelte'
  import Icon from './Icon.svelte'
  import { duration, time, tokens } from './format'
  import { list, t, type Key } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'
  import NoticeCard, { cardOf } from './NoticeCard.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import PlanCard from './PlanCard.svelte'
  import QuestionCard from './QuestionCard.svelte'
  import ValuesCard from './ValuesCard.svelte'
  import SecretCard from './SecretCard.svelte'
  import { doing, INTERNAL, type Advice, type Agent, type Block, type Subagent, type ToolCall, workdir } from './stream'
  import { settings } from './settings.svelte'
  import SubagentsCard from './SubagentsCard.svelte'
  import BackgroundCard from './BackgroundCard.svelte'
  import ToolRow from './ToolRow.svelte'
  import FileChip from './FileChip.svelte'
  import { filesIn, splitCited } from './mention.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import PluginLine from './PluginLine.svelte'
  import { useOf, type Use } from './pluginuse'
  import Shots from './Shots.svelte'
  import ShellCard from './ShellCard.svelte'
  import AdvisorCard from './AdvisorCard.svelte'

  let {
    agent,
    start = 0,
    ondiff,
    onsubagent,
    onsubagents,
    onpreview,
    onfork,
    onreplay = () => {},
    onterminal = () => {},
    onrun,
    onsend,
    onticket,
  }: {
    agent: Agent
    start?: number
    ondiff: (call: ToolCall) => void
    onfork?: (index: number) => void
    onsubagent?: (sub: Subagent) => void
    onsubagents?: () => void
    onpreview?: (proposal: Proposal) => void
    onreplay?: (command: string) => void
    onterminal?: (command: string) => void
    onrun?: (command: string) => void
    onsend?: (text: string) => void
    onticket?: (key: string) => void
  } = $props()

  const SPAWN = ['Agent', 'Task']

  const folded = new SvelteSet<number>()

  let menu = $state<{ index: number; text: string; x: number; y: number } | null>(null)

  function openMenu(event: MouseEvent, index: number, text: string) {
    event.preventDefault()
    menu = { index, text, x: event.clientX, y: event.clientY }
  }

  function copy() {
    const selected = window.getSelection()?.toString()
    copyText(selected || menu?.text || '')
    menu = null
  }

  const answer = (list: Segment[]) => list.flatMap((s) => (s.type === 'text' ? [s.text] : [])).join('\n\n')

  const names = $derived(namesOf(agent))

  let projectFiles = $state<string[]>([])
  const known = $derived(new Set(projectFiles))
  const linkCards = (text: string) => [...new Set(fences(text).flatMap((chunk) => (chunk.code ? [] : urlsIn(chunk.text).map((u) => u.url))))].slice(0, 3)
  const citing = $derived(agent.entries.filter((entry) => entry.kind === 'user' && entry.text.includes('@')).length)

  $effect(() => {
    if (!citing) return
    invoke<string[]>('project_files', { dir: workdir(agent) })
      .then((list) => (projectFiles = list))
      .catch(() => (projectFiles = []))
  })

  const kinds: [string[], Key][] = [
    [['Read', 'Grep', 'Glob', 'LS'], 'conversation.reads'],
    [['Edit', 'MultiEdit', 'Write', 'NotebookEdit'], 'conversation.edits'],
    [['Bash'], 'conversation.commands'],
    [['WebFetch', 'WebSearch'], 'conversation.webLookups'],
  ]

  const shotsOf = (list: Segment[]) => list.flatMap((s) => (s.type === 'work' ? s.calls.flatMap((c) => c.shots ?? []) : []))

  function summary(calls: ToolCall[]) {
    const parts: string[] = []
    let rest = calls.length
    for (const [names, key] of kinds) {
      const n = calls.filter((c) => names.includes(c.name)).length
      if (n) parts.push(t(key, { count: n }))
      rest -= n
    }
    if (rest) parts.push(t('conversation.others', { count: rest }))
    return t('conversation.toolsSummary', { actions: t('conversation.actions', { count: calls.length }), kinds: list(parts, 'unit') })
  }

  type Segment =
    | { type: 'work'; ms: number | undefined; thinking: boolean; thought: string; calls: ToolCall[] }
    | { type: 'spawn'; calls: ToolCall[] }
    | { type: 'plugin'; call: ToolCall; use: Use }
    | { type: 'advisor'; advice: Advice }
    | { type: 'ask' }
    | Extract<Block, { type: 'text' }>

  const ASKING = ['AskUserQuestion', 'ExitPlanMode']

  function segments(blocks: Block[]) {
    const out: Segment[] = []
    for (const block of blocks) {
      if (block.type === 'text') {
        if (block.text) out.push(block)
        continue
      }
      if (block.type === 'thinking') {
        const work = workOf(out)
        work.thinking = true
        work.ms = block.ms === undefined || work.ms === undefined ? undefined : work.ms + block.ms
        work.thought = [work.thought, block.text?.trim()].filter(Boolean).join('\n\n')
        continue
      }
      if (block.type === 'advisor') {
        out.push({ type: 'advisor', advice: block })
        continue
      }
      for (const call of block.calls.filter((c) => !INTERNAL.includes(c.name))) {
        const use = settings.plugin_lines === 'off' ? null : useOf(agent.plugins ?? [], call)
        if (ASKING.includes(call.name)) out.push({ type: 'ask' })
        else if (use && !(SPAWN.includes(call.name) && agent.subagents.some((s) => s.id === call.id))) out.push({ type: 'plugin', call, use })
        else if (!SPAWN.includes(call.name) || !agent.subagents.some((s) => s.id === call.id)) workOf(out).calls.push(call)
        else {
          const last = out.at(-1)
          if (last?.type === 'spawn') last.calls.push(call)
          else out.push({ type: 'spawn', calls: [call] })
        }
      }
    }
    return out
  }

  function groups(blocks: Block[]) {
    const out: ({ type: 'box'; segments: Segment[] } | Extract<Segment, { type: 'spawn' | 'advisor' | 'plugin' }>)[] = []
    let asked = false
    for (const segment of segments(blocks)) {
      const last = out.at(-1)
      if (segment.type === 'ask') asked = true
      else if (segment.type === 'spawn' || segment.type === 'advisor' || segment.type === 'plugin') out.push(segment)
      else if (last?.type === 'box' && !asked) last.segments.push(segment)
      else out.push({ type: 'box', segments: [segment] })
      if (segment.type !== 'ask') asked = false
    }
    return out
  }

  const verbs: [string[], Key][] = [
    [['Bash'], 'conversation.ranCommands'],
    [['Read', 'Grep', 'Glob', 'LS'], 'conversation.readFiles'],
    [['Edit', 'MultiEdit', 'Write', 'NotebookEdit'], 'conversation.editedFiles'],
    [['WebFetch', 'WebSearch'], 'conversation.lookedUp'],
  ]

  function now(steps: Segment[]) {
    const last = steps.at(-1)
    const call = last?.type === 'work' ? last.calls.at(-1) : undefined
    if (call && !call.result) return doing(call)
    if (last?.type === 'work' && last.thinking && last.ms === undefined) return t('conversation.thinking')
    return last?.type === 'text' ? t('conversation.writing') : t('conversation.working')
  }

  function runs(list: Segment[], thoughts: boolean) {
    const out: ({ thought: string; ms: number | undefined } | Extract<Segment, { type: 'text' }> | Segment[])[] = []
    for (const segment of list) {
      if (segment.type === 'text') {
        out.push(segment)
        continue
      }
      if (thoughts && segment.type === 'work' && segment.thought) {
        out.push({ thought: segment.thought, ms: segment.ms })
        if (segment.calls.length) out.push([{ ...segment, thinking: false, thought: '' }])
        continue
      }
      const last = out.at(-1)
      if (Array.isArray(last)) last.push(segment)
      else out.push([segment])
    }
    return out.length ? out : [[]]
  }

  function split(list: Segment[]) {
    let cut = list.length
    while (cut > 0 && list[cut - 1].type === 'text') cut--
    return { steps: list.slice(0, cut), answer: list.slice(cut) }
  }

  function stepsLine(steps: Segment[], live: boolean) {
    const works = steps.flatMap((s) => (s.type === 'work' ? [s] : []))
    const calls = works.flatMap((w) => w.calls)
    const parts: string[] = []
    const ms = works.reduce((t, w) => t + (w.thinking ? (w.ms ?? 0) : 0), 0)
    if (settings.thinking !== 'hidden' && ms >= 1000) parts.push(t('conversation.thoughtFor', { duration: duration(ms) }))
    let rest = calls.length
    for (const [names, key] of verbs) {
      const n = calls.filter((c) => names.includes(c.name)).length
      if (n) parts.push(t(key, { count: n }))
      rest -= n
    }
    if (rest) parts.push(t('conversation.usedTools', { count: rest }))
    let line = list(parts, 'unit')
    if (live) line = line ? t('conversation.liveSteps', { now: now(steps), steps: line }) : now(steps)
    return line.charAt(0).toUpperCase() + line.slice(1) || t('conversation.steps')
  }

  function workOf(out: Segment[]) {
    const last = out.at(-1)
    if (last?.type === 'work') return last
    const work: Segment = { type: 'work', ms: 0, thinking: false, thought: '', calls: [] }
    out.push(work)
    return work
  }

  const took = (ms: number | undefined) =>
    ms === undefined ? t('conversation.thinking') : ms < 1000 ? t('conversation.thoughtMoment') : t('conversation.thoughtForTitle', { duration: duration(ms) })

  const lastAgent = $derived.by(() => {
    const at = agent.entries.findLastIndex((e) => e.kind === 'agent')
    return at > agent.entries.findLastIndex((e) => e.kind === 'user') ? at : -1
  })
  const working = $derived(agent.status === 'working' && !agent.held)

  const tall = new SvelteSet<string>()

  function measure(el: HTMLElement, key: string) {
    const scroller = el.closest<HTMLElement>('.scroller')
    if (!scroller) return
    const check = () => (el.offsetHeight > scroller.clientHeight * 1.5 ? tall.add(key) : tall.delete(key))
    const watch = new ResizeObserver(check)
    watch.observe(el)
    watch.observe(scroller)
    return {
      update(next: string) {
        tall.delete(key)
        key = next
        check()
      },
      destroy() {
        watch.disconnect()
        tall.delete(key)
      },
    }
  }

  function toTop(event: MouseEvent) {
    const el = (event.currentTarget as HTMLElement).closest<HTMLElement>('.agent')
    const scroller = el?.closest<HTMLElement>('.scroller')
    if (!el || !scroller) return
    scroller.scrollTo({ top: scroller.scrollTop + el.getBoundingClientRect().top - scroller.getBoundingClientRect().top - 18, behavior: 'smooth' })
  }
</script>

{#snippet back(key: string)}
  {#if tall.has(key)}
    <div class="back"><button class="btn btn-ghost" type="button" onclick={toTop}><Icon name="arrow-up" size={12} />{t('conversation.backToTop')}</button></div>
  {/if}
{/snippet}

{#snippet musing(thought: string, ms: number | undefined)}
  <div class="musing" class:now={ms === undefined}>
    <span class="musing-head">{#if ms === undefined}<span class="pulse" aria-hidden="true"></span>{/if}{took(ms)}</span>
    <Markdown text={thought} streaming={ms === undefined} />
  </div>
{/snippet}

{#snippet detail(list: Segment[], i: number, tail: Block | undefined, quiet: boolean)}
  {#each list as segment, s (s)}
    {#if segment.type === 'text'}
      <div class:quiet><Markdown text={segment.text} streaming={!!tail && segment === tail} cards project={agent.project} {onrun} /></div>
    {:else if segment.type === 'work'}
      {#if segment.thinking && (settings.thinking !== 'hidden' || segment.ms === undefined)}
        {#if segment.thought && settings.thinking === 'shown'}
          {@render musing(segment.thought, segment.ms)}
        {:else if segment.thought && settings.thinking === 'summary'}
          <details class="thinking">
            <summary><span class="chevron" aria-hidden="true">›</span>{took(segment.ms)}<span class="gist">— {segment.thought.split(/(?<=[.!?])\s|\n/)[0]}</span></summary>
            <p class="thought-text">{segment.thought}</p>
          </details>
        {:else}
          <div class="thought">{took(segment.ms)}</div>
        {/if}
      {/if}
      {#if segment.calls.length}
        <details class="tools" open={i === lastAgent && working}>
          <summary><span class="chevron" aria-hidden="true">›</span>{summary(segment.calls)}{#if !quiet}<Shots paths={shotsOf([segment])} />{/if}</summary>
          <div class="calls">
            {#each segment.calls as call (call.id)}
              <ToolRow {agent} {call} {ondiff} />
            {/each}
          </div>
        </details>
      {/if}
    {/if}
  {/each}
{/snippet}

{#each agent.entries.slice(start) as entry, k (start + k)}
  {@const i = start + k}
  {#if entry.kind === 'user' && (entry.peer !== undefined || entry.from || entry.squadron)}
    <div class="peer" data-user={i} oncontextmenu={(e) => openMenu(e, i, entry.text)} role="presentation">
      <div class="head">
        <span class="origin">{entry.squadron ? t('conversation.fromSquadron') : t('conversation.fromSession')}</span>
        <span class="from">{#if !entry.squadron}<strong>{entry.from || entry.peer || '?'}</strong>{' · '}{/if}{time(entry.at)}{#if entry.from}{' · '}{t('conversation.approvedByYou')}{/if}</span>
      </div>
      {#if entry.comments}<div class="chips">{#each entry.comments as comment, c (c)}<CommentChip {comment} />{/each}</div>{/if}
      <Markdown text={entry.text} />
    </div>
  {:else if entry.kind === 'user'}
    {@const said = ticketBlocks(entry.text)}
    {@const cards = linkCards(said.text)}
    {#if entry.files?.length}
      <div class="files" data-user={i}>{#each entry.files as file (file)}<Attachment path={file} large look="card" />{/each}</div>
    {/if}
    {#if entry.text || entry.comments?.length || !entry.files?.length}
      <div class="user" data-user={entry.files?.length ? undefined : i} oncontextmenu={(e) => openMenu(e, i, entry.text)} role="presentation">
        {#if entry.comments}
          <div class="chips">{#each entry.comments as comment, c (c)}<CommentChip {comment} light />{/each}</div>
        {/if}
        {#if said.text}
          {#each fences(said.text) as chunk, c (c)}
            {#if chunk.code}<div class="snippet"><Markdown text={chunk.text} /></div>
            {:else}<div class="said">{#each highlight(chunk.text, names) as part, p (p)}{#if part.command}<span class="cmd">{part.text}</span>{:else}{#each splitCited(part.text, known) as bit, b (b)}{#if bit.path}<FileChip path={bit.path} count={filesIn(projectFiles, bit.path)} light />{:else}{#each splitUrls(bit.text) as piece, u (u)}{#if piece.url}<LinkChip url={piece.url} light link />{:else}{piece.text}{/if}{/each}{/if}{/each}{/if}{/each}</div>{/if}
          {/each}
        {/if}
        {#if said.blocks.length}
          <div class="tickets">
            {#each said.blocks as block, b (b)}
              <button class="ticket-chip" title={t('tickets.openTicket', { key: block.key })} onclick={() => onticket?.(block.key)}>
                <span class="logo" aria-hidden="true">J</span><strong>{block.key}</strong>{#if block.title}<span class="title">{block.title}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
    {#if cards.length}
      <div class="link-cards">{#each cards as url (url)}{#if isPullLink(url)}<PullCard {url} />{:else}<LinkCard {url} />{/if}{/each}</div>
    {/if}
    {#if entry.remote}<span class="via"><span class="dot" aria-hidden="true"></span>{t('conversation.sentRemote', { time: time(entry.at) })}</span>{/if}
  {:else if entry.kind === 'shell'}
    <ShellCard {entry} {onreplay} {onterminal} />
  {:else if entry.kind === 'agent'}
    {@const reply = groups(entry.blocks)}
    {@const tail = working && i === agent.entries.length - 1 ? entry.blocks.at(-1) : undefined}
    {#each reply as group, g (g)}
      {#if group.type === 'spawn'}
        <SubagentsCard {agent} ids={group.calls.map((c) => c.id)} onopen={onsubagent} onpanel={onsubagents} />
      {:else if group.type === 'advisor'}
        <AdvisorCard advice={group.advice} model={agent.options.advisor} />
      {:else if group.type === 'plugin'}
        {@const before = reply[g - 1]}
        {@const after = reply[g + 1]}
        <PluginLine {agent} call={group.call} use={group.use} first={before?.type !== 'plugin' || before.use.plugin !== group.use.plugin} last={after?.type !== 'plugin' || after.use.plugin !== group.use.plugin} />
      {:else if settings.steps === 'detailed'}
        <div class="agent" use:measure={`${i}.${g}`} oncontextmenu={(e) => openMenu(e, i, answer(group.segments))} role="presentation">{@render detail(group.segments, i, tail, false)}{@render back(`${i}.${g}`)}</div>
      {:else}
        {@const parts = split(group.segments)}
        {@const live = working && i === lastAgent && g === reply.length - 1}
        {@const chunks = runs(parts.steps, settings.thinking === 'shown' || (live && !parts.answer.length && settings.thinking !== 'hidden'))}
        {#each chunks as chunk, c (c)}
          {#if !Array.isArray(chunk) && 'thought' in chunk}
            {@render musing(chunk.thought, chunk.ms)}
          {:else if !Array.isArray(chunk)}
            <div class="agent" use:measure={`${i}.${g}.${c}`} oncontextmenu={(e) => openMenu(e, i, chunk.text)} role="presentation">
              {#if chunk.at}<span class="stamp">{time(chunk.at)}</span>{/if}
              <Markdown text={chunk.text} streaming={!!tail && chunk === tail} cards project={agent.project} {onrun} />
              {@render back(`${i}.${g}.${c}`)}
            </div>
          {:else if chunk.length || (live && !parts.answer.length)}
            <details class="steps" class:live={live && c === chunks.length - 1}>
              <summary><span class="chevron" aria-hidden="true">›</span><span class="line">{stepsLine(chunk, live && c === chunks.length - 1)}</span><Shots paths={shotsOf(chunk)} /></summary>
              <div class="step-list">{@render detail(chunk, i, tail, true)}</div>
            </details>
          {/if}
        {/each}
        {#if parts.answer.length}
          <div class="agent" use:measure={`${i}.${g}.a`} oncontextmenu={(e) => openMenu(e, i, answer(parts.answer))} role="presentation">{@render detail(parts.answer, i, tail, false)}{@render back(`${i}.${g}.a`)}</div>
        {/if}
      {/if}
    {/each}
  {:else if entry.kind === 'turn'}
    <span class="turn">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l5 5L20 6" /></svg>
      {t('conversation.turnDone', { index: entry.index, input: tokens(entry.input), output: tokens(entry.output), duration: duration(entry.ms) })}
      {#if entry.advised}<span class="advised">· {t('conversation.turnAdvised', { count: entry.advised.calls, input: tokens(entry.advised.input), output: tokens(entry.advised.output) })}</span>{/if}
    </span>
  {:else if entry.kind === 'interrupted'}
    <div class="interrupted">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
      <strong>{t('conversation.interruptedBy', { time: time(entry.at) })}</strong>
      <span>{t('conversation.replyStops')}</span>
    </div>
  {:else if entry.kind === 'compacted'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.compacted', { before: tokens(entry.before), after: tokens(entry.after), time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'note' && entry.command === 'notice' && onsend && cardOf(entry.text)}
    <NoticeCard id={agent.id} text={entry.text} at={entry.at} {onsend} />
  {:else if entry.kind === 'note' && entry.command === 'notice'}
    <div class="notice">
      <span class="from">Claude Code · {time(entry.at)}</span>
      <span class="out">{entry.text}</span>
    </div>
  {:else if entry.kind === 'note'}
    <div class="separator" role="separator">
      <span></span>
      <code class="cmd-note">{entry.command}</code> · {entry.text} · {time(entry.at)}
      <span></span>
    </div>
  {:else if entry.kind === 'cleared'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.cleared', { time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'launched'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.launchedBy', { by: entry.by, time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'forked'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.forkedFrom', { from: entry.from, time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'reloaded'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.reloaded', { time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'resumed'}
    <div class="separator" role="separator">
      <span></span>
      {t('conversation.resumed', { time: time(entry.at) })}
      <span></span>
    </div>
  {:else if entry.kind === 'error'}
    <ErrorCard {agent} {entry} />
  {:else if entry.kind === 'decision'}
    <div class="decision {entry.tone}">
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true">
        {#if entry.tone === 'ok'}<path d="M4 12l5 5L20 6" />{:else}<path d="M6 6l12 12M18 6L6 18" />{/if}
      </svg>
      <strong>{entry.title}</strong>
      <span class="detail">{entry.detail}</span>
      {#if entry.questions?.length}
        <button class="review" aria-expanded={!folded.has(i)} onclick={() => (folded.has(i) ? folded.delete(i) : folded.add(i))}>
          {!folded.has(i) ? t('conversation.hide') : t('conversation.reviewQuestions', { count: entry.questions.length })}
        </button>
      {/if}
      <span class="at">{time(entry.at)}</span>
    </div>
    {#if entry.questions && !folded.has(i)}
      <ol class="asked">
        {#each entry.questions as q (q.question)}
          {@const picked = q.answer.split(', ')}
          <li>
            <strong>{q.question}</strong>
            <ul>
              {#each q.options as option (option)}
                <li class:picked={picked.includes(option)}>{picked.includes(option) ? '✓' : '·'} {option}</li>
              {/each}
              {#if q.answer && !q.options.some((o) => picked.includes(o))}<li class="picked">✓ {q.answer}</li>{/if}
            </ul>
          </li>
        {/each}
      </ol>
    {/if}
    {#if entry.note}<span class="note">{entry.note}</span>{/if}
  {/if}
{/each}

{#each agent.asks as ask (ask.id)}
  {#if ask.tool_name === 'AskUserQuestion'}
    <QuestionCard {agent} {ask} />
  {:else if ask.tool_name === 'ExitPlanMode'}
    <PlanCard {agent} {ask} />
  {:else if ask.tool_name === VALUES}
    <ValuesCard {agent} {ask} />
  {:else if ask.tool_name === SUDO || ask.tool_name === SSH_KEY}
    <SecretCard {agent} {ask} />
  {:else}
    <PermissionCard {agent} {ask} onlog={onsubagent} {onpreview} />
  {/if}
{/each}

{#if agent.background.length}
  <BackgroundCard {agent} {onterminal} />
{/if}

{#if working}
  <span class="turn live">
    <svg class="spin" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
      ><circle cx="12" cy="12" r="9" stroke-dasharray="40 20" /></svg
    >
    {t('conversation.turnLive', { input: tokens(agent.live.input), output: tokens(agent.live.outputDone + agent.live.output), duration: duration(Math.max(0, clock.now - agent.live.started)) })}
  </span>
{/if}

<svelte:window onclick={() => (menu = null)} onkeydown={(e) => e.key === 'Escape' && (menu = null)} />

{#if menu}
  {@const index = menu.index}
  <div class="menu" role="menu" aria-label={t('conversation.messageActions')} style:left="{menu.x}px" style:top="{menu.y}px">
    {#if onfork && agent.entries[index]?.kind === 'user' && forkPoint(agent, index)}
      <button class="accent" role="menuitem" onclick={() => (onfork(index), (menu = null))}><Icon name="branch" />{t('conversation.forkFromHere')}</button>
      <hr />
    {/if}
    <button role="menuitem" onclick={copy}><Icon name="copy" />{t('common.copy')}</button>
  </div>
{/if}

<style>
  .steps > summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    color: var(--text-dim);
    font-size: 13px;
    list-style: none;
    cursor: pointer;
  }

  summary:has(:global(.shots.all)) {
    flex-wrap: wrap;
  }

  .steps > summary::-webkit-details-marker {
    display: none;
  }

  .steps > summary .line {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .steps > summary:hover {
    color: var(--text-2);
  }

  .steps[open] > summary .chevron {
    transform: rotate(90deg);
  }

  .steps.live .line {
    animation: breathe 1.6s ease-in-out infinite;
  }

  @keyframes breathe {
    50% {
      opacity: 0.55;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .steps.live .line {
      animation: none;
    }
  }

  .step-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 8px 0 4px 5px;
    padding-left: 16px;
    border-left: 1px solid var(--border);
  }

  .notice {
    align-self: flex-start;
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-width: 640px;
    padding: 8px 12px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .peer {
    align-self: flex-start;
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-width: 640px;
    margin-left: 20px;
    padding-left: 14px;
    border-left: 2px solid var(--accent);
    overflow-wrap: anywhere;
  }

  .peer .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .peer .origin {
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--accent);
  }

  .peer .from {
    font-size: 12px;
    color: var(--text-dim);
  }

  .peer .from strong {
    font-weight: 600;
    color: var(--text);
  }

  .notice .out {
    white-space: pre-wrap;
  }

  .notice .from {
    color: var(--text-faint);
    font-size: 11.5px;
  }

  .quiet {
    font-size: 13px;
    color: var(--text-muted);
  }

  .menu {
    position: fixed;
    z-index: 20;
    min-width: 180px;
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
    height: 32px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }
  .menu button :global(.icon) {
    color: var(--text-dim);
  }
  .menu .accent,
  .menu .accent :global(.icon) {
    color: var(--accent);
  }
  .menu hr {
    height: 1px;
    margin: 4px 2px;
    border: 0;
    background: var(--border-strong);
  }

  .menu button:hover {
    background: var(--menu-hover);
  }

  .tickets {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .said + .tickets {
    margin-top: 9px;
  }

  .ticket-chip {
    max-width: 100%;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 5px;
    border-radius: 12px;
    background: var(--accent-ink);
    color: var(--accent-soft);
    font-size: 12px;
  }

  .ticket-chip:hover {
    background: var(--wait-btn);
  }

  .ticket-chip strong {
    flex: none;
    font: 600 12px var(--mono);
  }

  .ticket-chip .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ticket-chip .logo {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: 3px;
    background: var(--jira);
    color: #fff;
    font: 700 9px var(--sans);
  }

  .user {
    align-self: flex-end;
    max-width: 540px;
    padding: 12px 16px;
    border-radius: 6px 6px 2px 6px;
    background: var(--accent);
    color: var(--accent-ink);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }


  .via {
    align-self: flex-end;
    margin-top: -6px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .via .dot {
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--remote-dot);
  }

  .files {
    align-self: flex-end;
    max-width: 540px;
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .files:has(+ .user) {
    margin-bottom: -8px;
  }

  .said {
    white-space: pre-wrap;
  }

  .link-cards {
    align-self: flex-end;
    width: min(440px, 100%);
    margin-top: -8px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .link-cards:empty {
    display: none;
  }

  .link-cards :global(.card) {
    margin: 0;
  }

  .said + .snippet,
  .snippet + .said,
  .snippet + .snippet,
  .snippet + .tickets {
    margin-top: 10px;
  }

  .user .snippet :global(.code) {
    margin: 0;
    background: rgb(20 14 10 / 0.82);
    box-shadow: none;
  }

  .user .snippet :global(.code-head) {
    border-color: rgb(255 255 255 / 0.08);
    color: rgb(255 255 255 / 0.5);
  }

  .user .snippet :global(pre) {
    color: #e8ddd4;
    overflow-wrap: normal;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chips:not(:last-child) {
    margin-bottom: 8px;
  }

  .agent {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .agent:empty {
    display: none;
  }

  .back {
    display: flex;
    justify-content: flex-end;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .back .btn {
    height: 28px;
    gap: 6px;
    padding: 0 11px;
    font-size: 12.5px;
  }

  .thought,
  summary {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .gist {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-dim);
  }

  details[open] .gist {
    display: none;
  }

  .musing {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 2px 0 2px 14px;
    border-left: 2px solid var(--border-strong);
    font-size: 13px;
    font-style: italic;
    color: var(--text-muted);
  }

  .stamp {
    margin-bottom: -8px;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .musing.now {
    border-color: var(--accent);
  }

  .musing-head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-style: normal;
    font-weight: 600;
    color: var(--text-dim);
  }

  .now .musing-head {
    color: var(--accent);
  }

  .pulse {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }

  .thought-text {
    margin: 6px 0 0 26px;
    white-space: pre-wrap;
    font-size: 12.5px;
    line-height: 1.6;
    color: var(--text-muted);
  }

  .thought::before {
    content: '';
    width: 18px;
    height: 18px;
    border-radius: 3px;
    background: var(--surface-3)
      url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Ccircle cx='12' cy='12' r='3' fill='%23B3A79F'/%3E%3C/svg%3E")
      center / 12px no-repeat;
  }

  summary {
    cursor: pointer;
    list-style: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  .chevron {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    background: var(--surface-3);
    transition: transform 0.15s;
  }

  details[open] > summary .chevron {
    transform: rotate(90deg);
  }

  .calls {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 6px;
    padding-left: 26px;
  }

  .turn {
    display: flex;
    align-items: center;
    gap: 8px;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .turn svg {
    color: var(--done);
  }

  .advised {
    color: var(--advisor);
  }

  .live svg {
    color: var(--ok);
  }

  .spin {
    animation: spin 1.4s linear infinite;
  }

  .interrupted {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 12px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px #4e443e;
    font-size: 12.5px;
  }

  .interrupted svg {
    fill: var(--text-2);
    stroke: none;
  }

  .interrupted strong {
    font-weight: 600;
  }

  .interrupted span {
    color: var(--text-muted);
  }

  .decision {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    border-radius: 4px;
    background: #1f2321;
    box-shadow: inset 0 0 0 1px #33403b;
    font-size: 12.5px;
  }

  .decision svg {
    color: var(--ok);
    stroke-width: 2.6;
  }

  .decision strong {
    font-weight: 600;
    color: var(--ok-text);
    white-space: nowrap;
  }

  .decision.bad {
    background: #241917;
    box-shadow: inset 0 0 0 1px #4a2e29;
  }

  .decision.bad svg,
  .decision.bad strong {
    color: var(--err-text);
  }

  .detail {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
    color: var(--text-3);
  }

  .at {
    color: var(--text-dim);
  }

  .review {
    flex: none;
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-link);
  }

  .review:hover {
    color: var(--accent-soft);
  }

  .asked {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: -4px 0 0;
    padding: 12px 14px 12px 34px;
    border-radius: 4px;
    background: var(--surface-1);
    font-size: 12.5px;
  }

  .asked ul {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    color: var(--text-dim);
  }

  .asked .picked {
    color: var(--ok-text);
    font-weight: 600;
  }

  .note {
    margin-top: -8px;
    padding-left: 38px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .separator {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .separator span {
    flex: 1;
    height: 1px;
    background: var(--border-strong);
  }

  .cmd {
    font-weight: 700;
  }

  .cmd-note {
    font-size: 12px;
    color: var(--accent-soft);
  }
</style>
