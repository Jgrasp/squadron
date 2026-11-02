<script lang="ts" module>
  export type Person = { name: string; avatar: string }
  export type Note = { key: string; line: number; removed: boolean; mine: boolean; author: Person; others: Person[]; text: string; current: boolean }
  export type TreeFile = { path: string; add: number; del: number; other?: boolean; notes?: Note[]; viewed?: boolean }

  type Node = { name: string; path: string; dirs: Node[]; files: TreeFile[] }

  export function tree(files: TreeFile[]): Node {
    const root: Node = { name: '', path: '', dirs: [], files: [] }
    for (const file of files) {
      const parts = file.path.split('/')
      let node = root
      for (let i = 0; i < parts.length - 1; i++) {
        const path = parts.slice(0, i + 1).join('/')
        let next = node.dirs.find((d) => d.path === path)
        if (!next) node.dirs.push((next = { name: parts[i], path, dirs: [], files: [] }))
        node = next
      }
      node.files.push(file)
    }
    const tidy = (node: Node): Node => {
      while (node.path && node.dirs.length === 1 && !node.files.length) {
        const [only] = node.dirs
        node = { ...only, name: `${node.name}/${only.name}` }
      }
      return {
        ...node,
        dirs: node.dirs.map(tidy).sort((a, b) => a.name.localeCompare(b.name)),
        files: [...node.files].sort((a, b) => a.path.localeCompare(b.path)),
      }
    }
    return tidy(root)
  }
</script>

<script lang="ts">
  import Avatar from './Avatar.svelte'
  import { t } from './i18n/index.svelte'

  let {
    files,
    selected,
    viewed = null,
    onpick,
    onnote,
  }: { files: TreeFile[]; selected: string | undefined; viewed?: number | null; onpick: (path: string) => void; onnote?: (key: string) => void } = $props()

  let filter = $state('')
  let closed = $state<string[]>([])

  const root = $derived(tree(files.filter((f) => f.path.toLowerCase().includes(filter.trim().toLowerCase()))))
  let list = $state<HTMLElement>()
  const current = $derived(files.flatMap((f) => f.notes ?? []).find((n) => n.current)?.key)

  $effect(() => {
    if (current) list?.querySelector('.note[aria-current="true"]')?.scrollIntoView({ block: 'nearest' })
  })

  const toggle = (path: string) => (closed = closed.includes(path) ? closed.filter((p) => p !== path) : [...closed, path])
</script>

<nav class="tree" aria-label={t('review.changedFiles')}>
  <input class="filter" type="search" placeholder={t('review.filterFiles', { count: files.length })} bind:value={filter} />
  {#if viewed !== null}
    <div class="progress">
      <span>{t('review.viewedOf', { count: viewed, total: files.length })}</span>
      <span class="bar" aria-hidden="true"><span style:width="{(viewed / Math.max(1, files.length)) * 100}%"></span></span>
    </div>
  {/if}
  <div class="list" bind:this={list}>
    {@render branch(root, 0)}
  </div>
</nav>

{#snippet branch(node: Node, depth: number)}
  {#each node.dirs as dir (dir.path)}
    {@const open = !closed.includes(dir.path) || !!filter.trim()}
    <button class="dir" style:--depth={depth} aria-expanded={open} title={dir.path} onclick={() => toggle(dir.path)}>
      <svg class="chevron" width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
      <span class="name">{dir.name}</span>
    </button>
    {#if open}{@render branch(dir, depth + 1)}{/if}
  {/each}
  {#each node.files as file (file.path)}
    <button class="file" class:other={file.other} class:viewed={file.viewed} style:--depth={depth} aria-current={file.path === selected ? 'true' : undefined} title={file.path} onclick={() => onpick(file.path)}>
      <span class="name">{file.path.split('/').pop()}</span>
      {#if file.viewed}
        <svg class="seen" width="11" height="11" viewBox="0 0 24 24" aria-label={t('review.viewed')}><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
      {/if}
      <span class="add">+{file.add}</span>{#if file.del}<span class="del">−{file.del}</span>{/if}
    </button>
    {#if file.notes?.length}
      <div class="notes" style:--depth={depth}>
        {#each file.notes as note (note.key)}
          <button class="note" class:mine={note.mine} aria-current={note.current ? 'true' : undefined} title={note.text} onclick={() => onnote?.(note.key)}>
            <span class="at">{note.removed ? '−' : ''}{note.line}</span>
            <span class="by">
              {#if note.mine}
                <span class="me" aria-hidden="true"><svg width="9" height="9" viewBox="0 0 24 24"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg></span>
                <strong>{t('common.you')}</strong>
              {:else}
                <Avatar name={note.author.name} src={note.author.avatar} size={16} />
                <strong>{note.author.name}</strong>
                {#if note.others.length}
                  <span class="others">
                    {#each note.others.slice(0, 3) as person (person.name)}<Avatar name={person.name} src={person.avatar} size={16} />{/each}
                    {#if note.others.length > 3}<span class="more">+{note.others.length - 3}</span>{/if}
                  </span>
                {/if}
              {/if}
            </span>
            <span class="text">{note.text}</span>
          </button>
        {/each}
      </div>
    {/if}
  {/each}
{/snippet}

<style>
  .tree {
    flex: none;
    width: 230px;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .filter {
    flex: none;
    height: 32px;
    margin: 10px;
    padding: 0 10px;
    border: none;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text);
    font: 12.5px var(--sans);
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 6px 10px;
  }

  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px 0 calc(8px + var(--depth) * 14px);
    border-radius: 4px;
    text-align: left;
    font: 12px var(--mono);
    color: var(--text-2);
  }

  button:hover {
    background: var(--surface-2);
  }

  .dir {
    color: var(--text-muted);
  }

  .chevron {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    transition: transform 0.12s;
  }

  .dir[aria-expanded='true'] .chevron {
    transform: rotate(90deg);
  }

  .file {
    padding-left: calc(24px + var(--depth) * 14px);
  }

  .file[aria-current='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .file.other {
    color: var(--text-dim);
  }

  .file.viewed {
    color: var(--text-faint);
  }

  .seen {
    flex: none;
    fill: none;
    stroke: var(--ok);
    stroke-width: 3;
  }

  .progress {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: -2px 10px 10px;
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .bar {
    height: 3px;
    border-radius: 2px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--ok);
    transition: width 0.2s;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .add {
    color: var(--ok);
  }

  .del {
    color: var(--err);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 2px 0 6px calc(10px + var(--depth) * 14px);
    padding-left: 4px;
    border-left: 1px solid var(--border-strong);
  }

  .note {
    --row: var(--bg-sidebar);
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 2px 6px;
    height: auto;
    padding: 5px 6px 5px 2px;
    background: var(--row);
    font: 12px var(--sans);
    white-space: nowrap;
  }

  .note:hover {
    --row: var(--surface-2);
    background: var(--row);
  }

  .note[aria-current='true'] {
    --row: var(--surface-4);
    background: var(--row);
  }

  .note[aria-current='true'] :global(.avatar) {
    background: var(--surface-5);
  }

  .at {
    align-self: center;
    font: 11.5px var(--mono);
    color: var(--text-dim);
    text-align: right;
  }

  .mine .at {
    color: var(--accent-soft);
  }

  .by {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .by strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
    color: var(--text-2);
  }

  .note[aria-current='true'] strong {
    color: var(--text-strong);
  }

  .me {
    flex: none;
    width: 16px;
    height: 16px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .me svg {
    fill: currentColor;
  }

  .others {
    flex: none;
    display: inline-flex;
    align-items: center;
    margin-left: 2px;
  }

  .others :global(.avatar) {
    box-shadow: 0 0 0 2px var(--row);
  }

  .others :global(.avatar + .avatar) {
    margin-left: -4px;
  }

  .more {
    margin-left: 4px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .note .text {
    grid-column: 2;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .note[aria-current='true'] .text {
    color: var(--text-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron,
    .bar span {
      transition: none;
    }
  }
</style>
