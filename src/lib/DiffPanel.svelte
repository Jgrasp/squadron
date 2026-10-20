<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { structuredPatch } from 'diff'
  import { changedFiles } from './diff'
  import DiffView from './DiffView.svelte'
  import { calls, relative, type Agent } from './stream'

  let { agent, file = $bindable(), onclose }: { agent: Agent; file: string; onclose: () => void } = $props()

  const files = $derived(changedFiles(calls(agent)))
  const selected = $derived(files.find((f) => f.path === file) ?? files[0])
  let current = $state<string | null | undefined>(undefined)
  let error = $state('')

  $effect(() => {
    const target = selected
    if (!target) return
    void target.calls.length
    current = undefined
    invoke<string | null>('read_project_file', { project: agent.project, path: target.path })
      .then((text) => {
        if (target === selected) current = text
      })
      .catch((e) => (error = String(e)))
  })

  const patch = $derived(
    selected && current !== undefined
      ? structuredPatch(selected.path, selected.path, selected.baseline ?? '', current ?? '', '', '', { context: 3 })
      : null,
  )
  const name = (path: string) => path.split('/').pop()
  const status = $derived(!selected ? '' : current === null ? 'DELETED' : selected.baseline === null ? 'NEW' : 'MODIFIED')
</script>

<section class="panel" aria-label="Diff">
  <div class="top">
    <div class="title">
      <h2>Diff</h2>
      <span>Changes by {agent.name} · {files.length} file{files.length > 1 ? 's' : ''}</span>
    </div>
    <button class="close" aria-label="Close panel" onclick={onclose}>×</button>
  </div>

  {#if files.length}
    <div class="tabs" role="tablist" aria-label="Changed files">
      {#each files as f (f.path)}
        <button role="tab" aria-selected={f === selected} title={relative(agent, f.path)} onclick={() => (file = f.path)}>
          {name(f.path)} <span class="add">+{f.add}</span>{#if f.del}<span class="del">−{f.del}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="body">
      {#if selected}
        <div class="file">
          <div class="file-head">
            <span class="badge" class:new={status === 'NEW'} class:gone={status === 'DELETED'}>{status}</span>
            <span class="path">{relative(agent, selected.path)}</span>
            <span class="by">{selected.calls.length} edit{selected.calls.length > 1 ? 's' : ''}</span>
          </div>
          {#if error}
            <p class="empty">{error}</p>
          {:else if !patch}
            <p class="empty">Reading the file…</p>
          {:else if patch.hunks.length}
            <DiffView hunks={patch.hunks} />
          {:else}
            <p class="empty">The file is back to its original state.</p>
          {/if}
        </div>
      {/if}
    </div>

    <div class="foot">
      <span>Diff since the agent first touched the file</span>
      <button class="btn btn-secondary" onclick={() => invoke('open_in_editor', { project: agent.project, path: selected.path })}>
        Open in editor
      </button>
    </div>
  {:else}
    <p class="empty pad">{agent.name} has not changed any file yet.</p>
  {/if}
</section>

<style>
  .panel {
    width: 680px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .top {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px 0 18px;
    border-bottom: 1px solid var(--border);
  }

  .title {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 700;
  }

  .title span {
    font-size: 12px;
    color: var(--text-dim);
  }

  .close {
    width: 32px;
    height: 32px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }

  .tabs {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12px var(--mono);
  }

  .tabs button[aria-selected='true'] {
    background: var(--surface-4);
    box-shadow: inset 0 0 0 1px var(--text-faint);
    color: var(--text-strong);
  }

  .add {
    color: var(--ok);
  }

  .del {
    color: var(--err);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 14px 16px;
  }

  .file {
    border-radius: 6px;
    box-shadow: 0 0 0 1px var(--border);
    overflow: hidden;
  }

  .file-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--surface-1);
    font-size: 12.5px;
  }

  .badge {
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 700;
  }

  .badge.new {
    background: #1f3330;
    color: var(--ok-text);
  }

  .badge.gone {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .path {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    font-family: var(--mono);
  }

  .by {
    color: var(--text-dim);
  }

  .empty {
    margin: 0;
    padding: 14px;
    color: var(--text-dim);
  }

  .pad {
    padding: 24px 18px;
  }

  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    font-size: 12.5px;
  }

  .foot span {
    flex: 1;
    color: var(--text-dim);
  }

  .foot .btn {
    height: 32px;
    padding: 0 12px;
  }
</style>
