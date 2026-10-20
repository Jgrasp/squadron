<script lang="ts">
  import { clock } from './agents.svelte'
  import ErrorCard from './ErrorCard.svelte'
  import { cost, duration, time, tokens } from './format'
  import Markdown from './Markdown.svelte'
  import type { Agent, Block, ToolCall } from './stream'
  import ToolRow from './ToolRow.svelte'

  let { agent, ondiff }: { agent: Agent; ondiff: (call: ToolCall) => void } = $props()

  const kinds: [string[], string, string][] = [
    [['Read', 'Grep', 'Glob', 'LS'], 'lecture', 'lectures'],
    [['Edit', 'MultiEdit', 'Write', 'NotebookEdit'], 'modification', 'modifications'],
    [['Bash'], 'commande', 'commandes'],
    [['WebFetch', 'WebSearch'], 'recherche web', 'recherches web'],
  ]

  function summary(calls: ToolCall[]) {
    const parts: string[] = []
    let rest = calls.length
    for (const [names, one, many] of kinds) {
      const n = calls.filter((c) => names.includes(c.name)).length
      if (n) parts.push(`${n} ${n > 1 ? many : one}`)
      rest -= n
    }
    if (rest) parts.push(`${rest} autre${rest > 1 ? 's' : ''}`)
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
      } else work.calls.push(...block.calls)
    }
    return out
  }

  const lastAgent = $derived(agent.entries.findLastIndex((e) => e.kind === 'agent'))
  const working = $derived(agent.status === 'working')
</script>

{#each agent.entries as entry, i (i)}
  {#if entry.kind === 'user'}
    <div class="user" data-user={i}>{entry.text}</div>
  {:else if entry.kind === 'agent'}
    <div class="agent">
      {#each segments(entry.blocks) as segment, s (s)}
        {#if segment.type === 'text'}
          <Markdown text={segment.text} streaming={working && i === agent.entries.length - 1 && segment === entry.blocks.at(-1)} />
        {:else}
          {#if segment.thinking}
            <div class="thought">
              {segment.ms === undefined ? 'Réfléchit…' : segment.ms < 1000 ? 'A réfléchi un instant' : `A réfléchi ${duration(segment.ms)}`}
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
      Tour {entry.index} terminé · {cost(entry.cost)} · {tokens(entry.input)} entrée · {tokens(entry.output)} sortie · {duration(entry.ms)}
    </span>
  {:else if entry.kind === 'interrupted'}
    <div class="interrupted">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="5" width="14" height="14" rx="2" /></svg>
      <strong>Interrompu par toi · {time(entry.at)}</strong>
      <span>La réponse s’arrête ici. Rien n’a été annulé dans les fichiers.</span>
    </div>
  {:else if entry.kind === 'reloaded'}
    <div class="separator" role="separator">
      <span></span>
      Session rechargée · {time(entry.at)} · conversation conservée
      <span></span>
    </div>
  {:else if entry.kind === 'error'}
    <ErrorCard {agent} {entry} />
  {/if}
{/each}

{#if working}
  <span class="turn live">
    <svg class="spin" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"
      ><circle cx="12" cy="12" r="9" stroke-dasharray="40 20" /></svg
    >
    Tour en cours · {tokens(agent.live.input)} entrée · {tokens(agent.live.outputDone + agent.live.output)} sortie · {duration(
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
    background: var(--text);
    color: var(--ink);
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
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
</style>
