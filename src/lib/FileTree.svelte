<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import FileTree from './FileTree.svelte'

  type Entry = { name: string; dir: boolean }

  let {
    project,
    path = '',
    depth = 0,
    version,
    expanded,
    active,
    marks,
    folders,
    onopen,
    ontoggle,
  }: {
    project: string
    path?: string
    depth?: number
    version: string
    expanded: string[]
    active: string
    marks: Map<string, string[]>
    folders: Set<string>
    onopen: (path: string) => void
    ontoggle: (path: string) => void
  } = $props()

  let entries = $state<Entry[] | null>(null)
  let error = $state('')

  $effect(() => {
    void version
    invoke<Entry[]>('list_dir', { project, path })
      .then((list) => ((entries = list), (error = '')))
      .catch((e) => (error = String(e)))
  })

  const indent = (extra: number) => `${8 + depth * 12 + extra}px`
</script>

{#if error}
  <li class="row muted" style:padding-left={indent(12)}>{error}</li>
{:else if entries}
  {#each entries as entry (entry.name)}
    {@const full = path ? `${path}/${entry.name}` : entry.name}
    {#if entry.dir}
      {@const open = expanded.includes(full)}
      <li>
        <button class="row" style:padding-left={indent(0)} aria-expanded={open} onclick={() => ontoggle(full)}>
          <span class="caret" aria-hidden="true">{open ? '▾' : '▸'}</span>
          <span class="name">{entry.name}</span>
          {#if folders.has(full)}<span class="dot" title="Contains files changed by agents"></span>{/if}
        </button>
        {#if open}
          <ul>
            <FileTree {project} path={full} depth={depth + 1} {version} {expanded} {active} {marks} {folders} {onopen} {ontoggle} />
          </ul>
        {/if}
      </li>
    {:else}
      {@const by = marks.get(full)}
      <li>
        <button class="row" class:active={active === full} class:touched={!!by} style:padding-left={indent(14)} onclick={() => onopen(full)}>
          <span class="name">{entry.name}</span>
          {#if by}<span class="by" title="Changed by {by.join(', ')}">{by.join(' ')}</span>{/if}
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

  .row.touched {
    color: var(--text-2);
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

  .by {
    flex: none;
    font: 11px var(--mono);
    color: var(--accent);
  }

  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 3px;
    background: var(--accent);
  }

  .muted {
    color: var(--text-faint);
    font-size: 12px;
  }
</style>
