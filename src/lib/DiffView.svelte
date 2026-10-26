<script lang="ts">
  import type { Snippet } from 'svelte'
  import { everything, fileLines, hidden, pairs, reveal, rows, type Hunk } from './diff'
  import { t } from './i18n/index.svelte'

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
    commentRevealed = true,
    content,
    whole = false,
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
    commentRevealed?: boolean
    content?: () => Promise<string | null>
    whole?: boolean
  } = $props()

  const STEP = 20

  let file = $state<string[] | null>(null)
  let opened = $derived(hunks)
  let max = $derived(limit)

  $effect(() => {
    file = null
    if (!content) return
    let live = true
    content()
      .then((text) => live && (file = text === null ? null : fileLines(text)))
      .catch(() => {})
    return () => (live = false)
  })

  const view = $derived(whole && file ? everything(opened, file) : opened)
  const gaps = $derived(file ? hidden(view, file.length) : [])
  const original = $derived(new Set(rows(hunks).flatMap((r) => (r.kind === 'add' || r.kind === 'same' ? [r.new] : []))))
  const unfold = (gap: number, way: 'up' | 'down', count = STEP) => file && (opened = reveal(view, file, gap, way, count))

  const at = (side: Line['side'], number?: number) => open?.side === side && open.number === number
  const noteAt = (side: Line['side'], number?: number) => marked.find((m) => m.side === side && m.number === number)?.note

  const unified = $derived(split ? [] : rows(view))
  const paired = $derived(split ? pairs(view) : [])
  const total = $derived(split ? paired.length : unified.length)
</script>

{#snippet plus(line: Line)}
  {@const note = noteAt(line.side, line.number)}
  {#if note !== undefined}
    <button class="plus bubble effect-snake" aria-label={t('review.editCommentOnLine', { line: String(line.number) })} title={note} onclick={() => oncomment?.(line)}>
      <svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>
    </button>
  {:else if oncomment && (commentRemoved || line.side === 'new') && (commentRevealed || line.side === 'old' || original.has(line.number))}
    <button class="plus" aria-label={t('review.commentLine', { line: String(line.number) })} title={t('review.commentThisLine')} onclick={() => oncomment(line)}>+</button>
  {/if}
{/snippet}

{#snippet head(gap: number, text = '')}
  {@const count = gaps[gap] ?? 0}
  <div class="head">
    {#if count}
      <span class="unfold">
        {#if count <= STEP}
          <button aria-label={t('review.showHidden', { count })} title={t('review.showHidden', { count })} onclick={() => unfold(gap, 'up', count)}>
            <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 9l4-4 4 4M8 15l4 4 4-4" /></svg>
          </button>
        {:else}
          {#if gap > 0}
            <button aria-label={t('review.showBelow', { count: STEP })} title={t('review.showBelow', { count: STEP })} onclick={() => unfold(gap, 'down')}>
              <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 4h8M8 11l4 4 4-4M12 7v8" /></svg>
            </button>
          {/if}
          {#if gap < view.length}
            <button aria-label={t('review.showAbove', { count: STEP })} title={t('review.showAbove', { count: STEP })} onclick={() => unfold(gap, 'up')}>
              <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 20h8M8 13l4-4 4 4M12 9v8" /></svg>
            </button>
          {/if}
        {/if}
      </span>
    {/if}
    <span>{text || (count ? `${count} more line${count > 1 ? 's' : ''}` : '')}</span>
  </div>
{/snippet}

<div class="diff" class:split>
  {#if split}
    {#each paired.slice(0, max) as row, i (i)}
      {#if row.kind === 'head'}
        {@render head(row.hunk, row.text)}
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
    {#each unified.slice(0, max) as row, i (i)}
      {#if row.kind === 'head'}
        {@render head(row.hunk, row.text)}
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
  {#if total > max}
    <button class="head more" onclick={() => (max += limit)}>{t('review.showMoreLines', { shown: Math.min(limit, total - max), total: total - max })}</button>
  {:else if gaps[view.length]}
    {@render head(view.length)}
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
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 24px;
    padding: 0 12px;
    color: var(--text-dim);
    background: var(--surface-1);
  }

  .unfold {
    display: flex;
    gap: 2px;
    margin-left: -8px;
  }

  .unfold button {
    width: 24px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .unfold button:hover,
  .more:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .unfold svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
  }

  .more {
    width: 100%;
    font: inherit;
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
