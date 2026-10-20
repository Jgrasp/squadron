<script lang="ts">
  import { pairs, rows, type Hunk } from './diff'

  export type Line = { side: 'old' | 'new'; number: number; text: string }

  let {
    hunks,
    split = false,
    limit = 1500,
    oncomment,
  }: { hunks: Hunk[]; split?: boolean; limit?: number; oncomment?: (line: Line) => void } = $props()

  const unified = $derived(split ? [] : rows(hunks))
  const paired = $derived(split ? pairs(hunks) : [])
  const total = $derived(split ? paired.length : unified.length)
</script>

{#snippet plus(line: Line)}
  {#if oncomment}
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
              <div class="half {cell.kind}">
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
      {/if}
    {/each}
  {:else}
    {#each unified.slice(0, limit) as row, i (i)}
      {#if row.kind === 'head'}
        <div class="head">{row.text}</div>
      {:else}
        <div class="line {row.kind}">
          {@render plus(row.kind === 'del' ? { side: 'old', number: row.old!, text: row.text } : { side: 'new', number: row.new!, text: row.text })}
          <span class="num">{row.old ?? ''}</span>
          <span class="num">{row.new ?? ''}</span>
          <span class="sign">{row.kind === 'add' ? '+' : row.kind === 'del' ? '−' : ''}</span>
          <span class="text">{row.text}</span>
        </div>
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
    grid-template-columns: 40px 40px 16px 1fr;
  }

  .pair {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }

  .half {
    grid-template-columns: 44px 1fr;
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

  .plus {
    position: absolute;
    top: 1px;
    left: 4px;
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
  .plus:focus-visible {
    opacity: 1;
  }
</style>
