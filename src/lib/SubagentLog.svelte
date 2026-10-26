<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick } from 'svelte'
  import { clock } from './agents.svelte'
  import { tokens } from './format'
  import { date, t, type Key } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'
  import PermissionCard from './PermissionCard.svelte'
  import { argument, replayLog, subagentOf, type Agent, type Subagent, type ToolCall } from './stream'
  import { asking, label, lookOf } from './subagents'

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

  const SIZES: ['panel' | 'wide' | 'screen', Key, Key][] = [
    ['panel', 'conversation.sizePanel', 'conversation.sizePanelHint'],
    ['wide', 'conversation.sizeWide', 'conversation.sizeWideHint'],
    ['screen', 'conversation.sizeScreen', 'conversation.sizeScreenHint'],
  ]

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
  const clockTime = (at?: number) => (at ? date(at, { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }) : '')
  const capital = (text: string) => text.charAt(0).toUpperCase() + text.slice(1)
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
    return call.result?.error ? t('conversation.error') : !call.result ? '…' : !lines ? t('conversation.noOutput') : lines === 1 && text.length < 60 ? text : t('conversation.lines', { count: lines })
  }

  async function copy() {
    const lines = [`#${sub.num} ${sub.name}`, t('conversation.copyPrompt', { prompt: sub.prompt })]
    for (const i of items) {
      if (i.kind === 'text') lines.push(`${clockTime(i.at)} ${i.text}`)
      else if (i.kind === 'thinking') lines.push(`${clockTime(i.at)} ${t('conversation.copyThinking', { text: i.text ?? '' })}`)
      else lines.push(`${clockTime(i.at)} ${i.call.name} ${argument(agent, i.call)}\n${i.call.result?.text ?? ''}`)
    }
    if (sub.result) lines.push(t('conversation.copyResult', { result: sub.result }))
    await navigator.clipboard.writeText(lines.join('\n'))
    copied = true
    setTimeout(() => (copied = false), 1500)
  }
</script>

<svelte:window onkeydown={keys} onkeydowncapture={leave} />

<div class="nav">
  <button class="back" onclick={() => onview(null)}>{t('conversation.allSubagents')}</button>
  <span class="grow"></span>
  <div class="sizes" role="group" aria-label={t('conversation.viewSize')}>
    {#each SIZES as [id, name, hint] (id)}
      <button aria-pressed={size === id} title={t(hint)} onclick={() => onsize(id)}>{t(name)}</button>
    {/each}
  </div>
  <div class="steps" role="group" aria-label={t('conversation.moveBetween')}>
    <button disabled={!previous} title={previous ? t('conversation.previousSubagent', { num: previous.num, name: previous.name }) : ''} onclick={() => previous && onview(previous.id)}>‹ {previous ? `#${previous.num}` : ''}</button>
    <button disabled={!next} title={next ? t('conversation.nextSubagent', { num: next.num, name: next.name }) : ''} onclick={() => next && onview(next.id)}>{next ? `#${next.num}` : ''} ›</button>
  </div>
</div>

<div class="head">
  <h3><span class="num">#{sub.num}</span> {sub.name}</h3>
  <span class="state {look}">
    <span class="dot"></span>{look === 'running' || look === 'waiting' ? t('conversation.stateFor', { state: capital(label(look)), time: span(lasted) }) : capital(label(look))}
  </span>
  <span class="meta">{[sub.type, model, sub.started && t('conversation.startedAt', { time: clockTime(sub.started) }), span(lasted), t('conversation.tokens', { n: tokens(sub.tokens) })].filter(Boolean).join(' · ')}</span>
  <div class="bar">
    <div class="filters" role="group" aria-label={t('conversation.filterLog')}>
      <button aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>{t('conversation.filterAll', { count: items.length })}</button>
      <button aria-pressed={filter === 'tools'} onclick={() => (filter = 'tools')}>{t('conversation.filterTools', { count: items.filter((i) => i.kind === 'tool').length })}</button>
      <button aria-pressed={filter === 'thinking'} onclick={() => (filter = 'thinking')}>{t('conversation.filterThinking', { count: items.filter((i) => i.kind === 'thinking').length })}</button>
    </div>
    <span class="grow"></span>
    <label class="follow"><input type="checkbox" bind:checked={follow} /> {t('conversation.followLive')}</label>
    <button class="copy" onclick={copy}>{copied ? t('common.copied') : t('conversation.copyLog')}</button>
  </div>
</div>

<div class="log" bind:this={scroller}>
  <div class="item">
    <span class="at">{clockTime(sub.started)}</span>
    <div class="what">
      <button class="prompt-head" aria-expanded={promptOpen} onclick={() => (promptOpen = !promptOpen)}>
        {t('conversation.promptFrom', { name: agent.name })} <span class="toggle">{promptOpen ? t('conversation.fold') : t('conversation.unfold')}</span>
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
          <p class="thinking">{item.text || (item.ms ? t('conversation.thoughtSeconds', { count: Math.round(item.ms / 1000) }) : t('conversation.thinking'))}</p>
        {:else}
          {@const output = item.call.result?.text.trim() ?? ''}
          <details class="tool" class:failed={item.call.result?.error}>
            <summary>
              <span class="name">{item.call.name}</span>
              <span class="arg">{argument(agent, item.call)}</span>
              <span class="out">{result(item.call)}{output ? ' ⌄' : ''}</span>
            </summary>
            {#if output}<pre>{output.split('\n').slice(0, 60).join('\n')}{output.split('\n').length > 60 ? `\n${t('conversation.moreLines', { count: output.split('\n').length - 60 })}` : ''}</pre>{/if}
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
    <strong>{t('conversation.result')}</strong>
    {#if sub.result}
      <Markdown text={sub.result} />
    {:else if sub.status === 'stopped'}
      <p>{t('conversation.stoppedNoResult')}</p>
    {:else}
      <p>{t('conversation.notHandedBack', { name: agent.name })}</p>
    {/if}
  </div>
</div>

<div class="foot">
  <span>{t('conversation.noWriting', { name: agent.name })}</span>
  <button class="btn btn-secondary" onclick={() => onquote(`#${sub.num} ${sub.name}`)}>{t('conversation.quote', { num: sub.num })}</button>
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
