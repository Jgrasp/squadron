<script lang="ts" module>
  export type TreeFile = { path: string; add: number; del: number; other?: boolean; talks?: number; drafts?: number }

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
  import { t } from './i18n/index.svelte'

  let { files, selected, onpick }: { files: TreeFile[]; selected: string | undefined; onpick: (path: string) => void } = $props()

  let filter = $state('')
  let closed = $state<string[]>([])

  const root = $derived(tree(files.filter((f) => f.path.toLowerCase().includes(filter.trim().toLowerCase()))))
  const toggle = (path: string) => (closed = closed.includes(path) ? closed.filter((p) => p !== path) : [...closed, path])
</script>

<nav class="tree" aria-label={t('review.changedFiles')}>
  <input class="filter" type="search" placeholder={t('review.filterFiles', { count: files.length })} bind:value={filter} />
  <div class="list">
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
    <button class="file" class:other={file.other} style:--depth={depth} aria-current={file.path === selected ? 'true' : undefined} title={file.path} onclick={() => onpick(file.path)}>
      <span class="name">{file.path.split('/').pop()}</span>
      {#if file.drafts}
        <span class="drafts" title={t('review.draftsInReview', { count: file.drafts })}
          ><svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>{file.drafts}</span
        >
      {/if}
      {#if file.talks}<span class="talks">{file.talks}</span>{/if}
      <span class="add">+{file.add}</span>{#if file.del}<span class="del">−{file.del}</span>{/if}
    </button>
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

  .drafts {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 16px;
    padding: 0 5px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-ink);
    font: 700 10.5px/16px var(--sans);
  }

  .drafts svg {
    fill: currentColor;
  }

  .talks {
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--wait-bg);
    color: var(--accent-soft);
    font: 600 10.5px/16px var(--sans);
    text-align: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .chevron {
      transition: none;
    }
  }
</style>
