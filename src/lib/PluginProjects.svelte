<script lang="ts">
  import { untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import Icon from './Icon.svelte'
  import { contained, rows, within, type Row } from './groups'
  import { t } from './i18n/index.svelte'
  import { projects } from './projects.svelte'

  type GroupRow = Extract<Row, { kind: 'group' }>

  const NOTE = 'squadron.pluginFollowNote'

  let {
    title,
    intro,
    chosen,
    groups = [],
    confirm,
    onsave,
    onclose,
  }: {
    title: string
    intro: string
    chosen: string[]
    groups?: string[]
    confirm: (count: number) => string
    onsave: (paths: string[], groups: string[]) => void
    onclose: () => void
  } = $props()

  const inside = (group: string) => contained(projects.groups, projects.list, group).map((p) => p.path)

  let follow = $state(new Set(untrack(() => groups.filter((id) => projects.groups.some((g) => g.id === id)))))
  const held = $derived(new Set([...follow].flatMap(inside)))
  let picked = $state(new Set(untrack(() => chosen.filter((p) => !held.has(p)))))
  let query = $state('')
  let folded = $state<Record<string, boolean>>({})
  let note = $state(localStorage.getItem(NOTE) !== '1')

  const all = $derived(projects.list.map((p) => p.path))
  const on = (path: string) => picked.has(path) || held.has(path)
  const count = $derived(all.filter(on).length)
  const every = $derived(count > 0 && count === all.length)
  const names = $derived(projects.groups.filter((g) => follow.has(g.id)).map((g) => g.name).join(', '))
  const list = $derived(rows(projects.groups, projects.list, folded, query))
  const blocks = $derived.by(() => {
    const out: { head: GroupRow | null; rows: Row[] }[] = []
    for (const row of list) {
      if (row.depth === 0 && row.kind === 'group') out.push({ head: row, rows: [] })
      else if (row.depth === 0) {
        if (out[out.length - 1]?.head !== null) out.push({ head: null, rows: [] })
        out[out.length - 1].rows.push(row)
      } else out[out.length - 1].rows.push(row)
    }
    return out
  })

  const inherited = (id: string) => [...follow].some((f) => f !== id && within(projects.groups, id, f))

  function setGroup(id: string, checked: boolean) {
    const next = new Set([...follow].filter((f) => !within(projects.groups, f, id)))
    if (checked) next.add(id)
    const rest = new Set(picked)
    for (const p of inside(id)) rest.delete(p)
    follow = next
    picked = rest
  }

  function setProject(path: string, checked: boolean) {
    const next = new Set(picked)
    if (checked) next.add(path)
    else next.delete(path)
    picked = next
  }

  function setAll(checked: boolean) {
    follow = new Set()
    picked = new Set(checked ? all : [])
  }

  function dismiss() {
    note = false
    localStorage.setItem(NOTE, '1')
  }

  function save() {
    const paths = all.filter(on)
    onsave(paths, paths.length === all.length ? [] : [...follow])
  }
</script>

{#snippet group(row: GroupRow, depth: number)}
  {@const paths = inside(row.id)}
  {@const n = paths.filter(on).length}
  {@const followed = follow.has(row.id)}
  {@const above = inherited(row.id)}
  <div class={depth < 0 ? 'bh' : 'row g'} style:--depth={Math.max(depth, 0)}>
    <button class="fold" aria-expanded={!row.folded} onclick={() => (folded[row.id] = !row.folded)}>
      <svg class="chev" class:open={!row.folded} width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
      <span class="nm">{row.name}</span>
      <span class="n">{n ? `${n}/${paths.length}` : paths.length}</span>
    </button>
    {#if followed}<span class="follows"><Icon name="link" size={11} />{t('plugins.follows')}</span>{/if}
    {#if above}<span class="link" title={t('plugins.followHeld')}><Icon name="link" size={12} /></span>{/if}
    <input
      type="checkbox"
      aria-label={t('plugins.followGroup', { name: row.name })}
      checked={followed || above}
      indeterminate={!followed && !above && n > 0}
      disabled={above || !paths.length}
      onchange={(e) => setGroup(row.id, e.currentTarget.checked)}
    />
  </div>
{/snippet}

{#snippet line(row: Row, depth: number)}
  {#if row.kind === 'group'}
    {@render group(row, depth)}
  {:else}
    {@const path = row.project.path}
    {@const fixed = held.has(path)}
    <label class="row p" class:on={on(path)} class:held={fixed} style:--depth={depth} style:--project={row.project.color} title={fixed ? t('plugins.followHeld') : undefined}>
      <span class="dot" aria-hidden="true"></span>
      <span class="nm">{row.project.name}</span>
      {#if fixed}<span class="link"><Icon name="link" size={12} /></span>{/if}
      <input type="checkbox" checked={on(path)} disabled={fixed} onchange={(e) => setProject(path, e.currentTarget.checked)} />
    </label>
  {/if}
{/snippet}

<Dialog {title} width={520} {onclose}>
  <div class="picker">
    <p class="intro">{intro}</p>
    {#if note && projects.groups.length}
      <div class="note">
        <Icon name="link" size={15} />
        <div>
          <strong>{t('plugins.followTitle')}</strong>
          <p>{t('plugins.followHint')}</p>
        </div>
        <button class="btn btn-ghost" onclick={dismiss}>{t('plugins.followGotIt')}</button>
      </div>
    {/if}
    <label class="search">
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
      <input placeholder={t('plugins.filterProjects', { count: all.length })} aria-label={t('plugins.filterProjectsLabel')} spellcheck="false" bind:value={query} />
    </label>
    <div class="tree">
      <label class="every">
        <span class="nm">{t('plugins.allProjects')}</span>
        <span class="n">{count}/{all.length}</span>
        <input type="checkbox" checked={every} indeterminate={count > 0 && !every} onchange={(e) => setAll(e.currentTarget.checked)} />
      </label>
      {#each blocks as block (block.head ? `g:${block.head.id}` : 'loose')}
        {#if block.head}
          <section class="box" class:follow={follow.has(block.head.id)} aria-label={block.head.name}>
            {@render group(block.head, -1)}
            {#each block.rows as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
              {@render line(row, row.depth - 1)}
            {/each}
          </section>
        {:else}
          <div class="loose">
            {#if blocks.length > 1}<span class="cap">{t('shell.groupRoot')}</span>{/if}
            {#each block.rows as row (row.kind === 'group' ? `g:${row.id}` : `p:${row.project.path}`)}
              {@render line(row, 0)}
            {/each}
          </div>
        {/if}
      {:else}
        <p class="none">{t('shell.projectsNoMatch')}</p>
      {/each}
    </div>
  </div>
  <div class="foot">
    <span class="count">{t('plugins.selected', { count })}{#if follow.size && !every}<span class="sub">{` · ${t('plugins.followsFoot', { groups: names })}`}</span>{/if}</span>
    <button class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
    <button class="btn btn-primary" onclick={save}>{confirm(count)}</button>
  </div>
</Dialog>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 20px 0;
  }

  .intro {
    margin: 0 0 2px;
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

  .note {
    display: flex;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--ok);
  }

  .note > :global(svg) {
    margin-top: 2px;
  }

  .note div {
    flex: 1;
    min-width: 0;
  }

  .note strong {
    display: block;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .note p {
    margin: 2px 0 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-dim);
  }

  .note .btn-ghost {
    align-self: flex-start;
    height: 26px;
    font-size: 12px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-dim);
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
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
    gap: 8px;
    max-height: 420px;
    overflow-y: auto;
  }

  .every {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font-weight: 700;
    color: var(--text-strong);
    cursor: pointer;
  }

  .box {
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .box.follow {
    box-shadow: inset 0 0 0 1px var(--ok-line);
  }

  .bh {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding-right: 8px;
    border-radius: 4px;
  }

  .bh:hover,
  .row:hover {
    background: var(--surface-2);
  }

  .fold {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    color: inherit;
    text-align: left;
    white-space: nowrap;
  }

  .bh .nm {
    flex: 0 1 auto;
    font-weight: 700;
    color: var(--text);
  }

  .loose {
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 0 4px;
  }

  .cap {
    padding: 6px 8px 2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .row {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px 0 calc(8px + var(--depth, 0) * 16px);
    border-radius: 4px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .row:hover {
    color: var(--text);
  }

  label.row {
    cursor: pointer;
  }

  .row.g {
    padding-left: calc(var(--depth, 0) * 16px);
    font-weight: 600;
    color: var(--text-2);
  }

  .row.g .nm {
    flex: 0 1 auto;
  }

  .row.p.on {
    color: var(--text);
  }

  .row.p.held {
    color: var(--text-3);
    cursor: default;
  }

  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .n {
    font: 500 11.5px var(--mono);
    color: var(--text-faint);
  }

  .chev {
    color: var(--text-faint);
    transition: transform 0.12s;
  }

  .chev.open {
    transform: rotate(90deg);
  }

  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project, var(--text-faint));
  }

  .follows {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 20px;
    padding: 0 7px;
    border-radius: 3px;
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--ok-text);
    font-size: 11.5px;
    font-weight: 600;
  }

  .link {
    display: grid;
    color: var(--ok-line);
  }

  input[type='checkbox'] {
    flex: none;
    width: 15px;
    height: 15px;
    margin: 0;
    accent-color: var(--accent);
    cursor: pointer;
  }

  input[type='checkbox']:disabled {
    opacity: 0.55;
    cursor: default;
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
    min-width: 0;
    color: var(--text-muted);
  }

  .sub {
    color: var(--ok-text);
  }
</style>
