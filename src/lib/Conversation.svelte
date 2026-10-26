<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity'
  import { clock, forkPoint } from './agents.svelte'
  import { namesOf } from './catalogs.svelte'
  import Attachment from './Attachment.svelte'
  import CommentChip from './CommentChip.svelte'
  import type { Proposal } from './diff'
  import { highlight } from './commands'
  import ErrorCard from './ErrorCard.svelte'
  import { cost, duration, time, tokens } from './format'
  import Markdown from './Markdown.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import PlanCard from './PlanCard.svelte'
  import QuestionCard from './QuestionCard.svelte'
  import { doing, INTERNAL, type Agent, type Block, type Subagent, type ToolCall } from './stream'
  import { settings } from './settings.svelte'
  import SubagentsCard from './SubagentsCard.svelte'
  import ToolRow from './ToolRow.svelte'

  let {
    agent,
    start = 0,
    ondiff,
    onsubagent,
    onsubagents,
    onpreview,
    onfork,
  }: {
    agent: Agent
    start?: number
    ondiff: (call: ToolCall) => void
    onfork?: (index: number) => void
    onsubagent?: (sub: Subagent) => void
    onsubagents?: () => void
    onpreview?: (proposal: Proposal) => void
  } = $props()

  const SPAWN = ['Agent', 'Task']

  const reviewing = new SvelteSet<number>()

  let menu = $state<{ index: number; text: string; x: number; y: number } | null>(null)

  function openMenu(event: MouseEvent, index: number, text: string) {
    event.preventDefault()
    menu = { index, text, x: event.clientX, y: event.clientY }
  }

  function copy() {
    const selected = window.getSelection()?.toString()
    navigator.clipboard.writeText(selected || menu?.text || '')
    menu = null
  }

  const names = $derived(namesOf(agent))

  const kinds: [string[], string, string][] = [
    [['Read', 'Grep', 'Glob', 'LS'], 'read', 'reads'],
    [['Edit', 'MultiEdit', 'Write', 'NotebookEdit'], 'edit', 'edits'],
    [['Bash'], 'command', 'commands'],
    [['WebFetch', 'WebSearch'], 'web lookup', 'web lookups'],
  ]

  function summary(calls: ToolCall[]) {
    const parts: string[] = []
    let rest = calls.length
    for (const [names, one, many] of kinds) {
      const n = calls.filter((c) => names.includes(c.name)).length
      if (n) parts.push(`${n} ${n > 1 ? many : one}`)
      rest -= n
    }
    if (rest) parts.push(`${rest} other${rest > 1 ? 's' : ''}`)
    return `${calls.length} action${calls.length > 1 ? 's' : ''} · ${parts.join(', ')}`
  }

  type Segment =
    | { type: 'work'; ms: number | undefined; thinking: boolean; thought: string; calls: ToolCall[] }
    | { type: 'spawn'; calls: ToolCall[] }
    | Extract<Block, { type: 'text' }>

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
      for (const call of block.calls.filter((c) => !INTERNAL.includes(c.name))) {
        if (!SPAWN.includes(call.name) || !agent.subagents.some((s) => s.id === call.id)) workOf(out).calls.push(call)
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
    const out: ({ type: 'box'; segments: Segment[] } | Extract<Segment, { type: 'spawn' }>)[] = []
    for (const segment of segments(blocks)) {
      const last = out.at(-1)
      if (segment.type === 'spawn') out.push(segment)
      else if (last?.type === 'box') last.segments.push(segment)
      else out.push({ type: 'box', segments: [segment] })
    }
    return out
  }

  const verbs: [string[], string, string][] = [
    [['Bash'], 'ran a command', 'ran {n} commands'],
    [['Read', 'Grep', 'Glob', 'LS'], 'read a file', 'read {n} files'],
    [['Edit', 'MultiEdit', 'Write', 'NotebookEdit'], 'edited a file', 'edited {n} files'],
    [['WebFetch', 'WebSearch'], 'looked something up', 'made {n} web lookups'],
  ]

  function now(steps: Segment[]) {
    const last = steps.at(-1)
    const call = last?.type === 'work' ? last.calls.at(-1) : undefined
    if (call && !call.result) return doing(call)
    if (last?.type === 'work' && last.thinking && last.ms === undefined) return 'Thinking…'
    return last?.type === 'text' ? 'Writing…' : 'Working…'
  }

  function runs(list: Segment[]) {
    const out: ({ thought: string; ms: number | undefined } | Segment[])[] = []
    for (const segment of list) {
      if (segment.type === 'work' && segment.thought) {
        out.push({ thought: segment.thought, ms: segment.ms })
        if (segment.calls.length) out.push([{ ...segment, thinking: false, thought: '' }])
        continue
      }
      const last = out.at(-1)
      if (Array.isArray(last)) last.push(segment)
      else out.push([segment])
    }
    return out
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
    if (settings.thinking !== 'hidden' && ms >= 1000) parts.push(`thought for ${duration(ms)}`)
    let rest = calls.length
    for (const [names, one, many] of verbs) {
      const n = calls.filter((c) => names.includes(c.name)).length
      if (n) parts.push(n > 1 ? many.replace('{n}', String(n)) : one)
      rest -= n
    }
    if (rest) parts.push(rest > 1 ? `used ${rest} tools` : 'used a tool')
    const notes = steps.filter((s) => s.type === 'text').length
    if (notes) parts.push(`${notes} note${notes > 1 ? 's' : ''}`)
    let line = parts.join(', ')
    if (live) line = line ? `${now(steps)} · ${line}` : now(steps)
    return line.charAt(0).toUpperCase() + line.slice(1) || 'Steps'
  }

  function workOf(out: Segment[]) {
    const last = out.at(-1)
    if (last?.type === 'work') return last
    const work: Segment = { type: 'work', ms: 0, thinking: false, thought: '', calls: [] }
    out.push(work)
    return work
  }

  const lastAgent = $derived(agent.entries.findLastIndex((e) => e.kind === 'agent'))
  const working = $derived(agent.status === 'working')
</script>

{#snippet musing(thought: string, ms: number | undefined)}
  <div class="musing">
    <span class="musing-head">{ms === undefined ? 'Thinking…' : ms < 1000 ? 'Thought for a moment' : `Thought for ${duration(ms)}`}</span>
    <Markdown text={thought} streaming={ms === undefined} />
  </div>
{/snippet}

{#snippet detail(list: Segment[], i: number, tail: Block | undefined, quiet: boolean)}
  {#each list as segment, s (s)}
    {#if segment.type === 'text'}
      <div class:quiet><Markdown text={segment.text} streaming={!!tail && segment === tail} /></div>
    {:else if segment.type === 'work'}
      {#if segment.thinking && (settings.thinking !== 'hidden' || segment.ms === undefined)}
        {@const took = segment.ms === undefined ? 'Thinking…' : segment.ms < 1000 ? 'Thought for a moment' : `Thought for ${duration(segment.ms)}`}
        {#if segment.thought && settings.thinking === 'shown'}
          {@render musing(segment.thought, segment.ms)}
        {:else if segment.thought && settings.thinking === 'summary'}
          <details class="thinking">
            <summary><span class="chevron" aria-hidden="true">›</span>{took}<span class="gist">— {segment.thought.split(/(?<=[.!?])\s|\n/)[0]}</span></summary>
            <p class="thought-text">{segment.thought}</p>
          </details>
        {:else}
          <div class="thought">{took}</div>
        {/if}
      {/if}
      {#if segment.calls.length}
        <details class="tools" open={i === lastAgent && working}>
          <summary><span class="chevron" aria-hidden="true">›</span>{summary(segment.calls)}</summary>
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
  {#if entry.kind === 'user'}
    <div class="user" data-user={i} oncontextmenu={(e) => openMenu(e, i, entry.text)} role="presentation">
      {#if entry.comments}
        <div class="chips">{#each entry.comments as comment, c (c)}<CommentChip {comment} light />{/each}</div>
      {/if}
      {#if entry.files}
        <div class="files">{#each entry.files as file (file)}<Attachment path={file} large />{/each}</div>
      {/if}
      {#if entry.text}<div class="said">{#each highlight(entry.text, names) as part, p (p)}{#if part.command}<span class="cmd">{part.text}</span>{:else}{part.text}{/if}{/each}</div>{/if}
    </div>
    {#if entry.from}<span class="via"><span class="dot" aria-hidden="true"></span>Sent by {entry.from}, approved by you · {time(entry.at)}</span>{/if}
    {#if entry.remote}<span class="via"><span class="dot" aria-hidden="true"></span>Sent via remote control · {time(entry.at)}</span>{/if}
  {:else if entry.kind === 'agent'}
    {@const reply = groups(entry.blocks)}
    {@const tail = working && i === agent.entries.length - 1 ? entry.blocks.at(-1) : undefined}
    {#each reply as group, g (g)}
      {#if group.type === 'spawn'}
        <SubagentsCard {agent} ids={group.calls.map((c) => c.id)} onopen={onsubagent} onpanel={onsubagents} />
      {:else if settings.steps === 'detailed'}
        <div class="agent">{@render detail(group.segments, i, tail, false)}</div>
      {:else}
        {@const parts = split(group.segments)}
        {@const live = working && i === lastAgent && g === reply.length - 1}
        {@const chunks = settings.thinking === 'shown' ? runs(parts.steps) : [parts.steps]}
        {#each chunks as chunk, c (c)}
          {#if !Array.isArray(chunk)}
            {@render musing(chunk.thought, chunk.ms)}
          {:else if chunk.length || (live && !parts.answer.length)}
            <details class="steps" class:live={live && c === chunks.length - 1}>
              <summary><span class="chevron" aria-hidden="true">›</span><span class="line">{stepsLine(chunk, live && c === chunks.length - 1)}</span></summary>
              <div class="step-list">{@render detail(chunk, i, tail, true)}</div>
            </details>
          {/if}
        {/each}
        {#if parts.answer.length}
          <div class="agent">{@render detail(parts.answer, i, tail, false)}</div>
        {/if}
      {/if}
    {/each}
  {:else if entry.kind === 'turn'}
    <span class="turn">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l5 5L20 6" /></svg>
      Turn {entry.index} done · {cost(entry.cost)} · {tokens(entry.input)} in · {tokens(entry.output)} out · {duration(entry.ms)}
    </span>
  {:else if entry.kind === 'interrupted'}
    <div class="interrupted">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
      <strong>Interrupted by you · {time(entry.at)}</strong>
      <span>The reply stops here. Nothing was undone in the files.</span>
    </div>
  {:else if entry.kind === 'compacted'}
    <div class="separator" role="separator">
      <span></span>
      Conversation compacted · {tokens(entry.before)} → {tokens(entry.after)} tokens · {time(entry.at)}
      <span></span>
    </div>
  {:else if entry.kind === 'note' && entry.command === 'notice'}
    <div class="notice">
      <span class="from">Claude Code · {time(entry.at)}</span>
      {entry.text}
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
      Conversation cleared · Claude starts over from here · {time(entry.at)}
      <span></span>
    </div>
  {:else if entry.kind === 'launched'}
    <div class="separator" role="separator">
      <span></span>
      Launched by {entry.by} · {time(entry.at)}
      <span></span>
    </div>
  {:else if entry.kind === 'forked'}
    <div class="separator" role="separator">
      <span></span>
      Forked from {entry.from} · {time(entry.at)} · context kept
      <span></span>
    </div>
  {:else if entry.kind === 'reloaded'}
    <div class="separator" role="separator">
      <span></span>
      Session reloaded · {time(entry.at)} · conversation kept
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
        <button class="review" aria-expanded={reviewing.has(i)} onclick={() => (reviewing.has(i) ? reviewing.delete(i) : reviewing.add(i))}>
          {reviewing.has(i) ? 'Hide' : `Review the question${entry.questions.length > 1 ? 's' : ''}`}
        </button>
      {/if}
      <span class="at">{time(entry.at)}</span>
    </div>
    {#if entry.questions && reviewing.has(i)}
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
  {:else}
    <PermissionCard {agent} {ask} onlog={onsubagent} {onpreview} />
  {/if}
{/each}

{#if agent.tasks.length}
  <details class="tasks" open={working}>
    <summary>
      <span class="chevron" aria-hidden="true">›</span>
      Tasks · {agent.tasks.filter((t) => t.status === 'completed').length} of {agent.tasks.length}
    </summary>
    <ul>
      {#each agent.tasks as task (task.toolUseId)}
        <li class={task.status}>
          <span class="mark" aria-hidden="true">{task.status === 'completed' ? '✓' : task.status === 'in_progress' ? '●' : '○'}</span>
          {task.status === 'in_progress' ? (task.activeForm ?? task.subject) : task.subject}
        </li>
      {/each}
    </ul>
  </details>
{/if}

{#if working}
  <span class="turn live">
    <svg class="spin" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
      ><circle cx="12" cy="12" r="9" stroke-dasharray="40 20" /></svg
    >
    Turn in progress · {tokens(agent.live.input)} in · {tokens(agent.live.outputDone + agent.live.output)} out · {duration(
      Math.max(0, clock.now - agent.live.started),
    )}
  </span>
{/if}

<svelte:window onclick={() => (menu = null)} onkeydown={(e) => e.key === 'Escape' && (menu = null)} />

{#if menu}
  {@const index = menu.index}
  <div class="menu" role="menu" aria-label="Message actions" style:left="{menu.x}px" style:top="{menu.y}px">
    {#if onfork && forkPoint(agent, index)}
      <button role="menuitem" onclick={() => (onfork(index), (menu = null))}>Fork from here</button>
    {/if}
    <button role="menuitem" onclick={copy}>Copy</button>
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
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover {
    background: var(--menu-hover);
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

  .files + .said {
    margin-top: 10px;
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }

  .said {
    white-space: pre-wrap;
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

  .musing-head {
    font-size: 12px;
    font-style: normal;
    font-weight: 600;
    color: var(--text-dim);
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

  .tasks {
    padding: 12px 16px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .tasks ul {
    margin: 8px 0 0;
    padding: 0 0 0 26px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }

  .tasks li {
    display: flex;
    gap: 8px;
    color: var(--text-muted);
  }

  .tasks li.completed {
    color: var(--text-dim);
    text-decoration: line-through;
    text-decoration-color: var(--text-faint);
  }

  .tasks li.in_progress {
    color: var(--text);
    font-weight: 600;
  }

  .mark {
    width: 12px;
    flex: none;
    text-align: center;
  }

  .completed .mark {
    color: var(--done);
  }

  .in_progress .mark {
    color: var(--accent);
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
