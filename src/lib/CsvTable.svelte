<script lang="ts">
  import { tick } from 'svelte'
  import { compareCells, delimiterOf, numericColumns, parseCsv } from './csv'
  import { t } from './i18n/index.svelte'

  let { text, name, fill = false }: { text: string; name: string; fill?: boolean } = $props()

  const PAGE = 200

  const rows = $derived(parseCsv(text, delimiterOf(text, name)))
  const columns = $derived(rows.reduce((most, row) => Math.max(most, row.length), 0))
  const header = $derived(rows[0] ?? [])
  const body = $derived(rows.slice(1).map((cells, index) => ({ cells, index: index + 1, search: cells.join('\u0000').toLowerCase() })))
  const numeric = $derived(numericColumns(rows.slice(1), columns))

  let filter = $state('')
  let sort = $state<{ column: number; dir: 1 | -1 }>()
  let limit = $state(PAGE)
  let widths = $state<number[]>()
  let table = $state<HTMLTableElement>()
  let scroller = $state<HTMLElement>()

  const shown = $derived.by(() => {
    const words = filter.trim().toLowerCase().split(/\s+/).filter(Boolean)
    const kept = words.length ? body.filter((row) => words.every((word) => row.search.includes(word))) : body
    if (!sort) return kept
    const { column, dir } = sort
    return kept.toSorted((a, b) => {
      const x = a.cells[column]?.trim() ?? ''
      const y = b.cells[column]?.trim() ?? ''
      return !x || !y ? compareCells(x, y) : dir * compareCells(x, y, numeric[column])
    })
  })

  $effect(() => {
    void filter, sort
    limit = PAGE
    scroller?.scrollTo({ top: 0 })
  })

  function more() {
    if (!scroller || limit >= shown.length) return
    if (scroller.scrollTop + scroller.clientHeight >= scroller.scrollHeight - 400) limit += PAGE
  }

  function order(column: number) {
    sort = sort?.column !== column ? { column, dir: 1 } : sort.dir === 1 ? { column, dir: -1 } : undefined
  }

  async function resize(event: PointerEvent, column: number) {
    event.preventDefault()
    const grip = event.currentTarget as HTMLElement
    if (!widths) {
      widths = [...table!.querySelectorAll('thead th')].slice(1).map((th) => th.getBoundingClientRect().width)
      await tick()
    }
    const start = event.clientX
    const from = widths[column]
    grip.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => (widths![column] = Math.max(56, from + e.clientX - start))
    const stop = () => (grip.removeEventListener('pointermove', move), grip.removeEventListener('pointerup', stop))
    grip.addEventListener('pointermove', move)
    grip.addEventListener('pointerup', stop)
  }
</script>

<div class="sheet" class:fill>
  {#if rows.length}
    <div class="bar">
      <input class="filter" type="search" placeholder={t('conversation.csvFilter')} aria-label={t('conversation.csvFilter')} bind:value={filter} />
      <span class="count">
        {shown.length === body.length ? t('conversation.csvRows', { count: body.length }) : t('conversation.csvMatches', { count: shown.length, total: body.length })}
        · {t('conversation.csvColumns', { count: columns })}
      </span>
    </div>
  {/if}
  <div class="scroll" bind:this={scroller} onscroll={more}>
    {#if !rows.length}
      <p class="note">{t('conversation.csvEmpty')}</p>
    {:else}
      <table bind:this={table} class:fixed={!!widths} style:width={widths && `${widths.reduce((sum, width) => sum + width, 52)}px`}>
        {#if widths}
          <colgroup>
            <col class="index" />
            {#each widths as width, column (column)}
              <col style:width="{width}px" />
            {/each}
          </colgroup>
        {/if}
        <thead>
          <tr>
            <th class="index" aria-hidden="true"></th>
            {#each { length: columns }, column (column)}
              <th class:number={numeric[column]} aria-sort={sort?.column === column ? (sort.dir === 1 ? 'ascending' : 'descending') : undefined}>
                <button type="button" title={header[column]} onclick={() => order(column)}>
                  <span>{header[column] ?? ''}</span>
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
                </button>
                <span class="grip" aria-hidden="true" onpointerdown={(e) => resize(e, column)}></span>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each shown.slice(0, limit) as row (row.index)}
            <tr>
              <td class="index">{row.index}</td>
              {#each { length: columns }, column (column)}
                {@const cell = row.cells[column] ?? ''}
                <td class:number={numeric[column]} title={cell.length > 24 ? cell : undefined}>{cell}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
      {#if !shown.length}
        <p class="note">{t('conversation.csvNoMatch')}</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .sheet {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .fill {
    height: 100%;
  }

  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px 8px 15px;
    border-bottom: 1px solid var(--border);
  }

  .filter {
    width: 220px;
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text);
    font: 12.5px var(--sans);
    outline: none;
  }

  .filter:focus {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .count {
    margin-left: auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .scroll {
    max-height: 440px;
    overflow: auto;
    overscroll-behavior: contain;
  }

  .scroll::-webkit-scrollbar-corner {
    background: transparent;
  }

  .fill .scroll {
    flex: 1;
    max-height: none;
  }

  table {
    border-collapse: separate;
    border-spacing: 0;
    font-size: 12.5px;
    line-height: 1.4;
  }

  table.fixed {
    table-layout: fixed;
  }

  col.index {
    width: 52px;
  }

  th,
  td {
    max-width: 360px;
    padding: 6px 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    border-bottom: 1px solid var(--border);
    border-right: 1px solid var(--border);
  }

  .fixed :is(th, td) {
    max-width: none;
  }

  td {
    color: var(--text);
  }

  .number {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0;
    background: var(--surface-2);
  }

  th button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    font: 600 12px var(--sans);
    color: var(--text-muted);
    text-align: inherit;
  }

  th.number button {
    flex-direction: row-reverse;
  }

  th button span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  th button:hover,
  th[aria-sort] button {
    color: var(--text-strong);
  }

  th svg {
    flex: none;
    width: 11px;
    height: 11px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0;
  }

  th button:hover svg {
    opacity: 0.5;
  }

  th[aria-sort] svg {
    opacity: 1;
  }

  th[aria-sort='ascending'] svg {
    transform: rotate(180deg);
  }

  .grip {
    position: absolute;
    inset: 0 -4px 0 auto;
    z-index: 1;
    width: 8px;
    cursor: col-resize;
  }

  .index {
    position: sticky;
    left: 0;
    z-index: 1;
    width: 52px;
    min-width: 52px;
    padding: 6px 10px;
    text-align: right;
    font: 11.5px var(--mono);
    color: var(--text-faint);
    background: var(--bg-main);
  }

  th.index {
    z-index: 2;
    background: var(--surface-2);
  }

  tbody tr:hover td {
    background: var(--surface-1);
  }

  .note {
    margin: 0;
    padding: 12px 15px;
    font-size: 12.5px;
    color: var(--text-dim);
  }
</style>
