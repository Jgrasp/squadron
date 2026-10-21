<script lang="ts">
  import { clock } from './agents.svelte'
  import { namesOf } from './catalogs.svelte'
  import CommentChip from './CommentChip.svelte'
  import { highlight } from './commands'
  import ErrorCard from './ErrorCard.svelte'
  import { cost, duration, time, tokens } from './format'
  import Markdown from './Markdown.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import PlanCard from './PlanCard.svelte'
  import QuestionCard from './QuestionCard.svelte'
  import { INTERNAL, type Agent, type Block, type ToolCall } from './stream'
  import ToolRow from './ToolRow.svelte'

  let { agent, ondiff }: { agent: Agent; ondiff: (call: ToolCall) => void } = $props()

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

  type Segment = { type: 'work'; ms: number | undefined; thinking: boolean; calls: ToolCall[] } | Extract<Block, { type: 'text' }>

  function segments(blocks: Block[]) {
    const out: Segment[] = []
    for (const block of blocks) {
      if (block.type === 'text') {
        if (block.text) out.push(block)
        continue
      }
      let work = out.at(-1)
      if (work?.type !== 'work') {
        work = { type: 'work', ms: 0, thinking: false, calls: [] }
        out.push(work)
      }
      if (block.type === 'thinking') {
        work.thinking = true
        work.ms = block.ms === undefined || work.ms === undefined ? undefined : work.ms + block.ms
      } else work.calls.push(...block.calls.filter((c) => !INTERNAL.includes(c.name)))
    }
    return out
  }

  const lastAgent = $derived(agent.entries.findLastIndex((e) => e.kind === 'agent'))
  const working = $derived(agent.status === 'working')
</script>

{#each agent.entries as entry, i (i)}
  {#if entry.kind === 'user'}
    <div class="user" data-user={i}>
      {#if entry.comments}
        <div class="chips">{#each entry.comments as comment, c (c)}<CommentChip {comment} light />{/each}</div>
      {/if}
      {#if entry.text}<div class="said">{#each highlight(entry.text, names) as part, p (p)}{#if part.command}<span class="cmd">{part.text}</span>{:else}{part.text}{/if}{/each}</div>{/if}
    </div>
  {:else if entry.kind === 'agent'}
    <div class="agent">
      {#each segments(entry.blocks) as segment, s (s)}
        {#if segment.type === 'text'}
          <Markdown text={segment.text} streaming={working && i === agent.entries.length - 1 && segment === entry.blocks.at(-1)} />
        {:else}
          {#if segment.thinking}
            <div class="thought">
              {segment.ms === undefined ? 'Thinking…' : segment.ms < 1000 ? 'Thought for a moment' : `Thought for ${duration(segment.ms)}`}
            </div>
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
    </div>
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
      <span class="at">{time(entry.at)}</span>
    </div>
    {#if entry.note}<span class="note">{entry.note}</span>{/if}
  {/if}
{/each}

{#each agent.asks as ask (ask.id)}
  {#if ask.tool_name === 'AskUserQuestion'}
    <QuestionCard {agent} {ask} />
  {:else if ask.tool_name === 'ExitPlanMode'}
    <PlanCard {agent} {ask} />
  {:else}
    <PermissionCard {agent} {ask} />
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

<style>
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
