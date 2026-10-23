<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick } from 'svelte'
  import { clock } from './agents.svelte'
  import { tokens } from './format'
  import Markdown from './Markdown.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import { argument, replayLog, subagentOf, type Agent, type Subagent, type ToolCall } from './stream'
  import { asking, LABEL, lookOf } from './subagents'

  let {
    agent,
    sub,
    onview,
    onquote,
    size,
    onsize,
  }: {
    agent: Agent
    sub: Subagent
    onview: (id: string | null) => void
    onquote: (text: string) => void
    size: 'panel' | 'wide' | 'screen'
    onsize: (size: 'panel' | 'wide' | 'screen') => void
  } = $props()

  const SIZES = [
    ['panel', 'Panel', 'Next to the conversation'],
    ['wide', 'Wide', 'Up to the agent list'],
    ['screen', 'Full screen', 'Full screen (F) · Esc to come back'],
  ] as const

  type Item = { kind: 'text'; at?: number; text: string } | { kind: 'thinking'; at?: number; text?: string; ms?: number } | { kind: 'tool'; at?: number; call: ToolCall }

  let filter = $state<'all' | 'tools' | 'thinking'>('all')
  let follow = $state(true)
  let promptOpen = $state(false)
  let copied = $state(false)
  let scroller: HTMLElement

  const look = $derived(lookOf(sub, asking(agent)))
  const ask = $derived(agent.asks.find((a) => subagentOf(agent, a.tool_use_id) === sub))
  const index = $derived(agent.subagents.indexOf(sub))
  const previous = $derived(agent.subagents[index - 1])
  const next = $derived(agent.subagents[index + 1])
  const items = $derived(
    sub.log.flatMap((b): Item[] =>
      b.type === 'tools'
        ? b.calls.map((call) => ({ kind: 'tool', at: b.at, call }))
        : b.type === 'text'
          ? [{ kind: 'text', at: b.at, text: b.text }]
          : [{ kind: 'thinking', at: b.at, text: b.text, ms: b.ms }],
    ),
  )
  const shown = $derived(items.filter((i) => filter === 'all' || (filter === 'tools' ? i.kind === 'tool' : i.kind === 'thinking')))

  const two = (n: number) => String(n).padStart(2, '0')
  const clockTime = (at?: number) => {
    if (!at) return ''
    const d = new Date(at)
    return `${two(d.getHours())}:${two(d.getMinutes())}:${two(d.getSeconds())}`
  }
  const span = (ms: number) => {
    const s = Math.max(0, Math.round(ms / 1000))
    return `${Math.floor(s / 60)}:${two(s % 60)}`
  }
  const lasted = $derived(sub.status === 'running' ? clock.now - sub.started : sub.ms || (sub.ended ?? sub.started) - sub.started)
  const model = $derived(
    (sub.model ?? '')
      .replace(/^claude-/, '')
      .replace(/-(\d+)-(\d+).*$/, ' $1.$2')
      .replace(/^\w/, (c) => c.toUpperCase()),
  )

  $effect(() => {
    if (sub.log.length || !agent.sessionId) return
    invoke<string | null>('subagent_log', { session: agent.sessionId, task: sub.task })
      .then((text) => text && replayLog(agent, sub, text))
      .catch(() => {})
  })

  $effect(() => {
    void shown.length
    void ask
    if (follow) tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight }))
  })

  function keys(event: KeyboardEvent) {
    const target = event.target as HTMLElement
    if (event.ctrlKey || event.metaKey || event.altKey || target.closest('input, textarea, [contenteditable], dialog')) return
    if (event.key === 'j' && next) onview(next.id)
    else if (event.key === 'k' && previous) onview(previous.id)
    else if (event.key === 'f') onsize(size === 'screen' ? 'panel' : 'screen')
  }

  function leave(event: KeyboardEvent) {
    if (event.key !== 'Escape' || size !== 'screen') return
    event.stopImmediatePropagation()
    event.preventDefault()
    onsize('panel')
  }

  const result = (call: ToolCall) => {
    const text = call.result?.text.trim() ?? ''
    const lines = text ? text.split('\n').length : 0
    return call.result?.error ? 'Error' : !call.result ? '…' : !lines ? 'No output' : lines === 1 && text.length < 60 ? text : `${lines} line${lines > 1 ? 's' : ''}`
  }

  async function copy() {
    const lines = [`#${sub.num} ${sub.name}`, `Prompt: ${sub.prompt}`]
    for (const i of items) {
      if (i.kind === 'text') lines.push(`${clockTime(i.at)} ${i.text}`)
      else if (i.kind === 'thinking') lines.push(`${clockTime(i.at)} (thinking) ${i.text ?? ''}`)
      else lines.push(`${clockTime(i.at)} ${i.call.name} ${argument(agent, i.call)}\n${i.call.result?.text ?? ''}`)
    }
    if (sub.result) lines.push(`Result: ${sub.result}`)
    await navigator.clipboard.writeText(lines.join('\n'))
    copied = true
    setTimeout(() => (copied = false), 1500)
  }
</script>

<svelte:window onkeydown={keys} onkeydowncapture={leave} />

<div class="nav">
  <button class="back" onclick={() => onview(null)}>‹ All subagents</button>
  <span class="grow"></span>
  <div class="sizes" role="group" aria-label="Size of the view">
    {#each SIZES as [id, label, title] (id)}
      <button aria-pressed={size === id} {title} onclick={() => onsize(id)}>{label}</button>
    {/each}
  </div>
  <div class="steps" role="group" aria-label="Move between subagents">
    <button disabled={!previous} title={previous ? `Previous subagent · #${previous.num} ${previous.name} (K)` : ''} onclick={() => previous && onview(previous.id)}>‹ {previous ? `#${previous.num}` : ''}</button>
    <button disabled={!next} title={next ? `Next subagent · #${next.num} ${next.name} (J)` : ''} onclick={() => next && onview(next.id)}>{next ? `#${next.num}` : ''} ›</button>
  </div>
</div>

<div class="head">
  <h3><span class="num">#{sub.num}</span> {sub.name}</h3>
  <span class="state {look}">
    <span class="dot"></span>{LABEL[look][0].toUpperCase() + LABEL[look].slice(1)}{look === 'running' || look === 'waiting' ? ` · for ${span(lasted)}` : ''}
  </span>
  <span class="meta">{[sub.type, model, sub.started && `started at ${clockTime(sub.started)}`, span(lasted), `${tokens(sub.tokens)} tokens`].filter(Boolean).join(' · ')}</span>
  <div class="bar">
    <div class="filters" role="group" aria-label="Filter the log">
      <button aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>All · {items.length}</button>
      <button aria-pressed={filter === 'tools'} onclick={() => (filter = 'tools')}>Tools · {items.filter((i) => i.kind === 'tool').length}</button>
      <button aria-pressed={filter === 'thinking'} onclick={() => (filter = 'thinking')}>Thinking · {items.filter((i) => i.kind === 'thinking').length}</button>
    </div>
    <span class="grow"></span>
    <label class="follow"><input type="checkbox" bind:checked={follow} /> Follow live</label>
    <button class="copy" onclick={copy}>{copied ? 'Copied' : 'Copy log'}</button>
  </div>
</div>

<div class="log" bind:this={scroller}>
  <div class="item">
    <span class="at">{clockTime(sub.started)}</span>
    <div class="what">
      <button class="prompt-head" aria-expanded={promptOpen} onclick={() => (promptOpen = !promptOpen)}>
        Prompt received from {agent.name} <span class="toggle">{promptOpen ? 'Fold' : 'Unfold ›'}</span>
      </button>
      <p class="prompt" class:open={promptOpen}>{sub.prompt}</p>
    </div>
  </div>

  {#each shown as item, i (i)}
    <div class="item">
      <span class="at">{clockTime(item.at)}</span>
      <div class="what">
        {#if item.kind === 'text'}
          <Markdown text={item.text} />
        {:else if item.kind === 'thinking'}
          <p class="thinking">{item.text || (item.ms ? `Thought for ${Math.round(item.ms / 1000)} s` : 'Thinking…')}</p>
        {:else}
          {@const output = item.call.result?.text.trim() ?? ''}
          <details class="tool" class:failed={item.call.result?.error}>
            <summary>
              <span class="name">{item.call.name}</span>
              <span class="arg">{argument(agent, item.call)}</span>
              <span class="out">{result(item.call)}{output ? ' ⌄' : ''}</span>
            </summary>
            {#if output}<pre>{output.split('\n').slice(0, 60).join('\n')}{output.split('\n').length > 60 ? `\n… ${output.split('\n').length - 60} more lines` : ''}</pre>{/if}
          </details>
        {/if}
      </div>
    </div>
  {/each}

  {#if ask}
    <div class="item">
      <span class="at"></span>
      <div class="what"><PermissionCard {agent} {ask} /></div>
    </div>
  {/if}

  <div class="result">
    <strong>Result</strong>
    {#if sub.result}
      <Markdown text={sub.result} />
    {:else if sub.status === 'stopped'}
      <p>Stopped before handing back a result.</p>
    {:else}
      <p>Not handed back yet. It goes back to {agent.name} at the end.</p>
    {/if}
  </div>
</div>

<div class="foot">
  <span>You don't write to a subagent. To steer it, write to {agent.name}.</span>
  <button class="btn btn-secondary" onclick={() => onquote(`#${sub.num} ${sub.name}`)}>Quote #{sub.num} in the message</button>
</div>

<style>
  .nav {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }

  .back {
    height: 28px;
    padding: 0 8px;
    border-radius: 4px;
    color: var(--text-2);
    font-weight: 600;
  }

  .back:hover,
  .steps button:not(:disabled):hover {
    background: var(--surface-3);
  }

  .grow {
    flex: 1;
  }

  .steps,
  .sizes {
    display: flex;
    gap: 4px;
  }

  .sizes {
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
    gap: 0;
  }

  .sizes button {
    height: 24px;
    padding: 0 9px;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .sizes button[aria-pressed='true'] {
    background: var(--surface-5);
    color: var(--text-strong);
    font-weight: 600;
  }

  .steps button {
    min-width: 52px;
    height: 28px;
    padding: 0 8px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12px var(--mono);
  }

  .steps button:disabled {
    opacity: 0.4;
  }

  .head {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  h3 {
    margin: 0;
    font-size: 15px;
  }

  .num {
    font: 500 13px var(--mono);
    color: var(--text-muted);
  }

  .state {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--look);
  }

  .running {
    --look: var(--sub-running);
  }

  .waiting {
    --look: var(--sub-waiting);
    color: var(--accent-soft);
  }

  .error {
    --look: var(--sub-error);
    color: var(--err-text);
  }

  .done {
    --look: var(--sub-done);
  }

  .stopped {
    --look: var(--sub-stopped);
  }

  .meta {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-top: 4px;
  }

  .filters {
    display: flex;
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .filters button {
    height: 24px;
    padding: 0 9px;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .filters button[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .follow {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .copy {
    height: 26px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12px;
  }

  .log {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
  }

  .item {
    display: grid;
    grid-template-columns: 62px 1fr;
    gap: 10px;
    font-size: 13px;
    line-height: 1.55;
  }

  .at {
    padding-top: 2px;
    font: 11px var(--mono);
    color: var(--text-faint);
  }

  .what {
    min-width: 0;
  }

  .prompt-head {
    display: flex;
    gap: 8px;
    font-weight: 600;
    color: var(--text-2);
  }

  .toggle {
    font-weight: 500;
    color: var(--accent-link);
  }

  .prompt {
    margin: 4px 0 0;
    padding: 8px 10px;
    border-radius: 4px;
    background: var(--surface-1);
    color: var(--text-muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .prompt.open {
    white-space: pre-wrap;
  }

  .thinking {
    margin: 0;
    font-style: italic;
    color: var(--text-dim);
  }

  .tool {
    border-radius: 4px;
    background: var(--surface-1);
  }

  .tool summary {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 10px;
    cursor: pointer;
    list-style: none;
  }

  .tool summary::-webkit-details-marker {
    display: none;
  }

  .name {
    flex: none;
    font-weight: 600;
  }

  .arg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 12px var(--mono);
    color: var(--text-2);
  }

  .out {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .failed .out {
    color: var(--err-text);
  }

  pre {
    margin: 0;
    padding: 8px 10px;
    border-top: 1px solid var(--border);
    overflow-x: auto;
    font: 11.5px/1.5 var(--mono);
    color: var(--text-2);
  }

  .result {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-left: 72px;
    padding: 10px 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font-size: 13px;
  }

  .result p {
    margin: 0;
    color: var(--text-dim);
  }

  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-dim);
  }

  .foot span {
    flex: 1;
  }

  .foot .btn {
    height: 30px;
    padding: 0 12px;
  }
</style>
