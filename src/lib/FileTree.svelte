<script lang="ts" module>
  export type Change = 'added' | 'modified' | 'deleted'
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileTree from './FileTree.svelte'

  type Entry = { name: string; dir: boolean; gone?: boolean }

  const ORDER: Change[] = ['added', 'modified', 'deleted']

  let {
    project,
    path = '',
    depth = 0,
    gone = false,
    version,
    expanded,
    active,
    status,
    folders,
    authors,
    onopen,
    ontoggle,
  }: {
    project: string
    path?: string
    depth?: number
    gone?: boolean
    version: string
    expanded: string[]
    active: string
    status: Map<string, Change>
    folders: Map<string, Set<Change>>
    authors: Map<string, string[]>
    onopen: (path: string) => void
    ontoggle: (path: string) => void
  } = $props()

  let entries = $state<Entry[] | null>(null)
  let error = $state('')

  $effect(() => {
    void version
    if (gone) {
      entries = []
      return
    }
    invoke<Entry[]>('list_dir', { project, path })
      .then((list) => ((entries = list), (error = '')))
      .catch((e) => (error = String(e)))
  })

  const indent = (extra: number) => `${8 + depth * 12 + extra}px`

  const shown = $derived.by(() => {
    if (!entries) return null
    const names = new Set(entries.map((e) => e.name))
    const prefix = path ? `${path}/` : ''
    const ghosts = new Map<string, boolean>()
    for (const [file, change] of status) {
      if (change !== 'deleted' || !file.startsWith(prefix)) continue
      const rest = file.slice(prefix.length).split('/')
      if (!names.has(rest[0])) ghosts.set(rest[0], ghosts.get(rest[0]) || rest.length > 1)
    }
    return [...entries, ...[...ghosts].map(([name, dir]) => ({ name, dir, gone: true }))].sort(
      (a, b) => Number(b.dir) - Number(a.dir) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
    )
  })

  function title(full: string) {
    const change = status.get(full)
    const by = authors.get(full)
    return [change && change[0].toUpperCase() + change.slice(1), by && `by ${by.join(', ')}`].filter(Boolean).join(' ') || undefined
  }
</script>

{#if error}
  <li class="row muted" style:padding-left={indent(12)}>{error}</li>
{:else if shown}
  {#each shown as entry (entry.name)}
    {@const full = path ? `${path}/${entry.name}` : entry.name}
    {#if entry.dir}
      {@const open = expanded.includes(full)}
      <li>
        <button class="row" class:deleted={entry.gone} style:padding-left={indent(0)} aria-expanded={open} onclick={() => ontoggle(full)}>
          <span class="caret" aria-hidden="true">{open ? '▾' : '▸'}</span>
          <span class="name">{entry.name}</span>
          {#each ORDER.filter((c) => folders.get(full)?.has(c)) as change (change)}
            <span class="square {change}" title="Contains {change} files"></span>
          {/each}
        </button>
        {#if open}
          <ul>
            <FileTree
              {project}
              path={full}
              depth={depth + 1}
              gone={gone || !!entry.gone}
              {version}
              {expanded}
              {active}
              {status}
              {folders}
              {authors}
              {onopen}
              {ontoggle}
            />
          </ul>
        {/if}
      </li>
    {:else}
      <li>
        <button
          class="row {status.get(full) ?? ''}"
          class:active={active === full}
          style:padding-left={indent(14)}
          title={title(full)}
          disabled={entry.gone}
          onclick={() => onopen(full)}
        >
          <span class="name">{entry.name}</span>
        </button>
      </li>
    {/if}
  {/each}
{/if}

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding-right: 8px;
    border-radius: 3px;
    color: var(--text-muted);
    text-align: left;
    white-space: nowrap;
  }

  .row:hover {
    background: var(--surface-1);
    color: var(--text);
  }

  .row.active {
    background: var(--menu);
    color: var(--text-strong);
  }

  .row.added,
  .square.added {
    --tint: #9ad6cb;
  }

  .row.modified,
  .square.modified {
    --tint: #f0c987;
  }

  .row.deleted,
  .square.deleted {
    --tint: #f59e8c;
  }

  .row.added,
  .row.modified,
  .row.deleted {
    background: color-mix(in srgb, var(--tint) 10%, transparent);
    color: var(--tint);
  }

  .row.added:hover,
  .row.modified:hover,
  .row.deleted:not(:disabled):hover {
    background: color-mix(in srgb, var(--tint) 22%, transparent);
    color: var(--tint);
  }

  .row.added.active,
  .row.modified.active {
    background: color-mix(in srgb, var(--tint) 30%, transparent);
  }

  .row.deleted .name {
    text-decoration: line-through;
  }

  .row:disabled {
    cursor: default;
  }

  .square {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 2px;
    background: var(--tint);
  }

  .caret {
    width: 8px;
    flex: none;
    color: var(--text-dim);
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }


  .muted {
    color: var(--text-faint);
    font-size: 12px;
  }
</style>
