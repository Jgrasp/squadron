<script lang="ts">
  import type { Snippet } from 'svelte'
  import { pairs, rows, type Hunk } from './diff'

  export type Line = { side: 'old' | 'new'; number: number; text: string }

  let {
    hunks,
    split = false,
    limit = 1500,
    oncomment,
    open,
    marked = [],
    editor,
    below,
    commentRemoved = true,
  }: {
    hunks: Hunk[]
    split?: boolean
    limit?: number
    oncomment?: (line: Line) => void
    open?: Pick<Line, 'side' | 'number'> | null
    marked?: (Pick<Line, 'side' | 'number'> & { note: string })[]
    editor?: Snippet
    below?: Snippet<[Line]>
    commentRemoved?: boolean
  } = $props()

  const at = (side: Line['side'], number?: number) => open?.side === side && open.number === number
  const noteAt = (side: Line['side'], number?: number) => marked.find((m) => m.side === side && m.number === number)?.note

  const unified = $derived(split ? [] : rows(hunks))
  const paired = $derived(split ? pairs(hunks) : [])
  const total = $derived(split ? paired.length : unified.length)
</script>

{#snippet plus(line: Line)}
  {@const note = noteAt(line.side, line.number)}
  {#if note !== undefined}
    <button class="plus bubble effect-snake" aria-label="Edit the comment on line {line.number}" title={note} onclick={() => oncomment?.(line)}>
      <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>
    </button>
  {:else if oncomment && (commentRemoved || line.side === 'new')}
    <button class="plus" aria-label="Comment line {line.number}" title="Comment this line" onclick={() => oncomment(line)}>+</button>
  {/if}
{/snippet}

<div class="diff" class:split>
  {#if split}
    {#each paired.slice(0, limit) as row, i (i)}
      {#if row.kind === 'head'}
        <div class="head">{row.text}</div>
      {:else}
        <div class="pair">
          {#each [row.left, row.right] as cell, side (side)}
            {#if cell}
              <div
                class="half {cell.kind}"
                class:picked={side ? at('new', cell.number) : cell.kind === 'del' && at('old', cell.number)}
                class:commented={side ? noteAt('new', cell.number) !== undefined : cell.kind === 'del' && noteAt('old', cell.number) !== undefined}
              >
                {#if side === 1 || cell.kind === 'del'}
                  {@render plus({ side: side ? 'new' : 'old', number: cell.number, text: cell.text })}
                {/if}
                <span class="num">{cell.number}</span>
                <span class="text">{cell.text}</span>
              </div>
            {:else}
              <div class="half blank"></div>
            {/if}
          {/each}
        </div>
        {#if at('old', row.left?.kind === 'del' ? row.left.number : undefined) || at('new', row.right?.number)}
          {@render editor?.()}
        {/if}
        {#if row.left?.kind === 'del'}{@render below?.({ side: 'old', number: row.left.number, text: row.left.text })}{/if}
        {#if row.right}{@render below?.({ side: 'new', number: row.right.number, text: row.right.text })}{/if}
      {/if}
    {/each}
  {:else}
    {#each unified.slice(0, limit) as row, i (i)}
      {#if row.kind === 'head'}
        <div class="head">{row.text}</div>
      {:else}
        <div
          class="line {row.kind}"
          class:picked={row.kind === 'del' ? at('old', row.old) : at('new', row.new)}
          class:commented={(row.kind === 'del' ? noteAt('old', row.old) : noteAt('new', row.new)) !== undefined}
        >
          {@render plus(row.kind === 'del' ? { side: 'old', number: row.old!, text: row.text } : { side: 'new', number: row.new!, text: row.text })}
          <span class="num">{row.old ?? ''}</span>
          <span class="num">{row.new ?? ''}</span>
          <span class="sign">{row.kind === 'add' ? '+' : row.kind === 'del' ? '−' : ''}</span>
          <span class="text">{row.text}</span>
        </div>
        {#if row.kind === 'del' ? at('old', row.old) : at('new', row.new)}
          {@render editor?.()}
        {/if}
        {@render below?.(row.kind === 'del' ? { side: 'old', number: row.old!, text: row.text } : { side: 'new', number: row.new!, text: row.text })}
      {/if}
    {/each}
  {/if}
  {#if total > limit}
    <div class="head">… {total - limit} more lines</div>
  {/if}
</div>

<style>
  .diff {
    padding: 4px 0;
    background: var(--bg-app);
    font: 12px/20px var(--mono);
    color: var(--text-3);
  }

  .head {
    padding: 0 12px;
    color: var(--text-dim);
  }

  .line,
  .half {
    position: relative;
    display: grid;
  }

  .line {
    grid-template-columns: 56px 40px 16px 1fr;
  }

  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }

  .half {
    grid-template-columns: 64px 1fr;
  }

  .half + .half {
    border-left: 1px solid var(--border);
  }

  .half.blank {
    background: repeating-linear-gradient(135deg, transparent 0 6px, rgba(255, 255, 255, 0.02) 6px 12px);
  }

  .num {
    padding-right: 4px;
    text-align: right;
    color: #918579;
    user-select: none;
  }

  .half .num {
    padding-right: 10px;
  }

  .sign {
    text-align: center;
    user-select: none;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    padding-right: 12px;
  }

  .add {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .add .num {
    color: var(--ok-line);
  }

  .del {
    background: var(--del-bg);
    color: var(--err-text);
  }

  .del .num {
    color: var(--del-line);
  }

  .commented {
    background-image: linear-gradient(rgba(224, 138, 91, 0.1), rgba(224, 138, 91, 0.1));
    box-shadow: inset 3px 0 0 var(--accent-soft);
  }

  .picked {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .plus {
    position: absolute;
    top: 1px;
    left: 9px;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font: 700 14px/1 var(--sans);
    opacity: 0;
  }

  .line:hover .plus,
  .half:hover .plus,
  .plus:focus-visible,
  .bubble {
    opacity: 1;
  }

  .bubble {
    top: 0;
    width: 20px;
    height: 20px;
    background: var(--wait-bg);
    color: var(--accent-soft);
  }

  .bubble svg {
    fill: currentColor;
  }
</style>
