<script lang="ts">
  import { untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { contained, rows } from './groups'
  import { t } from './i18n/index.svelte'
  import { projects } from './projects.svelte'

  let {
    title,
    intro,
    chosen,
    confirm,
    onsave,
    onclose,
  }: { title: string; intro: string; chosen: string[]; confirm: (count: number) => string; onsave: (paths: string[]) => void; onclose: () => void } = $props()

  let picked = $state(new Set(untrack(() => chosen)))
  let query = $state('')
  let folded = $state<Record<string, boolean>>({})

  const all = $derived(projects.list.map((p) => p.path))
  const list = $derived(rows(projects.groups, projects.list, folded, query))
  const count = $derived(all.filter((p) => picked.has(p)).length)

  const inside = (group: string) => contained(projects.groups, projects.list, group).map((p) => p.path)
  const tally = (paths: string[]) => {
    const n = paths.filter((p) => picked.has(p)).length
    return { on: n > 0 && n === paths.length, mixed: n > 0 && n < paths.length, n }
  }
  const every = $derived(tally(all))

  function flip(paths: string[], on: boolean) {
    const next = new Set(picked)
    for (const p of paths) {
      if (on) next.add(p)
      else next.delete(p)
    }
    picked = next
  }
</script>

<Dialog {title} width={520} {onclose}>
  <div class="picker">
    <p class="intro">{intro}</p>
    <label class="search">
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
      <input placeholder={t('plugins.filterProjects', { count: all.length })} aria-label={t('plugins.filterProjectsLabel')} spellcheck="false" bind:value={query} />
    </label>
    <label class="row all">
      <span class="nm">{t('plugins.allProjects')}</span>
      <small>{every.n} / {all.length}</small>
      <input type="checkbox" checked={every.on} indeterminate={every.mixed} onchange={(e) => flip(all, e.currentTarget.checked)} />
    </label>
    <div class="tree">
      {#each list as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
        {#if row.kind === 'group'}
          {@const paths = inside(row.id)}
          {@const s = tally(paths)}
          <div class="row g" style:--depth={row.depth}>
            <button class="fold" aria-expanded={!row.folded} aria-label={row.name} onclick={() => {
                folded[row.id] = !row.folded
              }}>
              <svg class="chev" class:open={!row.folded} width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg>
              <span class="nm">{row.name}</span>
            </button>
            <small>{s.n ? `${s.n}/${paths.length}` : paths.length}</small>
            <input type="checkbox" aria-label={row.name} checked={s.on} indeterminate={s.mixed} disabled={!paths.length} onchange={(e) => flip(paths, e.currentTarget.checked)} />
          </div>
        {:else}
          <label class="row p" class:on={picked.has(row.project.path)} style:--depth={row.depth} style:--project={row.project.color}>
            <span class="dot" aria-hidden="true"></span>
            <span class="nm">{row.project.name}</span>
            <input type="checkbox" checked={picked.has(row.project.path)} onchange={(e) => flip([row.project.path], e.currentTarget.checked)} />
          </label>
        {/if}
      {:else}
        <p class="none">{t('shell.projectsNoMatch')}</p>
      {/each}
    </div>
  </div>
  <div class="foot">
    <span class="count">{t('plugins.selected', { count })}</span>
    <button class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn btn-primary" onclick={() => onsave(all.filter((p) => picked.has(p)))}>{confirm(count)}</button>
  </div>
</Dialog>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 20px 0;
  }

  .intro {
    margin: 0;
    color: var(--text-dim);
    font-size: 13px;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-dim);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
    color: var(--text);
  }

  .tree {
    display: flex;
    flex-direction: column;
    max-height: 360px;
    overflow-y: auto;
  }

  .row {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 10px 0 calc(8px + var(--depth, 0) * 16px);
    border-radius: 4px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .row::before {
    content: '';
    position: absolute;
    left: 8px;
    top: 0;
    bottom: 0;
    width: calc(var(--depth, 0) * 16px);
    background: repeating-linear-gradient(to right, transparent 0 6px, var(--border-strong) 6px 7px, transparent 7px 16px);
    pointer-events: none;
  }

  .row:hover {
    background: color-mix(in srgb, var(--text) 6%, transparent);
    color: var(--text);
  }

  label.row {
    cursor: pointer;
  }

  .row.all {
    height: 40px;
    font-weight: 600;
    color: var(--text-strong);
    border-bottom: 1px solid var(--border-strong);
    border-radius: 4px 4px 0 0;
  }

  .row.g {
    font-weight: 600;
    color: var(--text-2);
  }

  .row.p.on {
    color: var(--text);
  }

  .fold {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
  }

  .chev {
    color: var(--text-faint);
    transition: transform 0.12s;
  }

  .chev.open {
    transform: rotate(90deg);
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--project, var(--accent));
  }

  small {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  input[type='checkbox'] {
    flex: none;
    width: 15px;
    height: 15px;
    margin: 0;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .none {
    margin: 16px 8px;
    color: var(--text-dim);
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }

  .count {
    flex: 1;
    color: var(--text-muted);
  }
</style>
