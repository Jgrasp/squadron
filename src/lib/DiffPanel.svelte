<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { parsePatch, structuredPatch } from 'diff'
  import { changedFiles, type Changes, type Hunk } from './diff'
  import DiffView, { type Line } from './DiffView.svelte'
  import { layout, MIN_DIFF, resizeDiff, setSplit, toggleDiffFull } from './layout.svelte'
  import { calls, relative, type Agent, type Comment } from './stream'

  let {
    agent,
    changes,
    file = $bindable(),
    comments = $bindable(),
    onclose,
  }: { agent: Agent; changes: Changes | null; file: string; comments: Comment[]; onclose: () => void } = $props()

  const STATUS: Record<string, string> = { A: 'NEW', D: 'DELETED' }

  const edited = $derived(changedFiles(calls(agent)).map((f) => ({ ...f, path: relative(agent, f.path) })))
  const files = $derived(
    changes
      ? changes.files
          .map((f) => ({ path: f.path, add: f.add ?? 0, del: f.del ?? 0, edits: edited.find((e) => e.path === f.path)?.calls.length ?? 0 }))
          .sort((a, b) => Number(b.edits > 0) - Number(a.edits > 0))
      : edited.map((f) => ({ path: f.path, add: f.add, del: f.del, edits: f.calls.length })),
  )
  const mine = $derived(files.filter((f) => f.edits).length)
  const selected = $derived(files.find((f) => f.path === file) ?? files[0])

  let view = $state<{ path: string; hunks: Hunk[]; status: string; binary: boolean } | null>(null)
  let error = $state('')
  let loads = 0

  $effect(() => {
    const path = selected?.path
    if (!path) return
    const git = changes && { from: changes.from, now: changes.now, status: changes.files.find((f) => f.path === path)?.status ?? '' }
    const baseline = edited.find((f) => f.path === path)?.baseline ?? null
    void selected.edits
    const run = ++loads
    error = ''
    const loading = git
      ? invoke<string>('git_patch', { project: agent.project, from: git.from, now: git.now, path }).then((text) => ({
          path,
          hunks: (parsePatch(text)[0]?.hunks ?? []) as Hunk[],
          status: STATUS[git.status] ?? 'MODIFIED',
          binary: /^Binary files /m.test(text),
        }))
      : invoke<string | null>('read_project_file', { project: agent.project, path }).then((current) => ({
          path,
          hunks: structuredPatch(path, path, baseline ?? '', current ?? '', '', '', { context: 3 }).hunks as Hunk[],
          status: current === null ? 'DELETED' : baseline === null ? 'NEW' : 'MODIFIED',
          binary: false,
        }))
    loading
      .then((result) => {
        if (run === loads) view = result
      })
      .catch((e) => {
        if (run === loads) error = String(e)
      })
  })

  const shown = $derived(view?.path === selected?.path ? view : null)
  const width = $derived(layout.diffWidths[layout.split ? 'split' : 'unified'])

  let panel: HTMLElement
  const widest = () => (panel.parentElement?.clientWidth ?? window.innerWidth) - 360

  function drag(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const right = panel.getBoundingClientRect().right
    handle.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => (e.buttons ? resizeDiff(right - e.clientX, widest()) : up())
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }

  function nudge(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') resizeDiff(panel.offsetWidth + 16, widest())
    else if (event.key === 'ArrowRight') resizeDiff(panel.offsetWidth - 16, widest())
  }
  const name = (path: string) => path.split('/').pop()

  let commenting = $state<(Line & { path: string }) | null>(null)
  let note = $state('')

  const indexOf = (path: string, line: Pick<Line, 'side' | 'number'>) =>
    comments.findIndex((c) => c.path === path && c.line === line.number && c.removed === (line.side === 'old'))
  const existing = $derived(commenting ? indexOf(commenting.path, commenting) : -1)
  const marked = $derived(
    comments.filter((c) => c.path === selected?.path).map((c) => ({ side: c.removed ? ('old' as const) : ('new' as const), number: c.line, note: c.note })),
  )

  function open(line: Line) {
    commenting = { ...line, path: selected.path }
    note = comments[indexOf(selected.path, line)]?.note ?? ''
  }

  function comment(event?: Event) {
    event?.preventDefault()
    if (!commenting || !note.trim()) return
    const { path, side, number, text } = commenting
    if (existing >= 0) comments[existing].note = note.trim()
    else comments.push({ path, line: number, removed: side === 'old', code: text, note: note.trim() })
    commenting = null
  }

  function remove() {
    if (existing >= 0) comments.splice(existing, 1)
    commenting = null
  }
</script>

<section class="panel" class:full={layout.diffFull} style:width={layout.diffFull ? null : `${width}px`} aria-label="Diff" bind:this={panel}>
  {#if !layout.diffFull}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="Resize the diff panel"
      aria-valuenow={width}
      aria-valuemin={MIN_DIFF}
      tabindex="0"
      title="Drag to resize, double-click for full screen"
      onpointerdown={drag}
      onkeydown={nudge}
      ondblclick={toggleDiffFull}
    ></div>
  {/if}
  <div class="top">
    <div class="title">
      <h2>Diff</h2>
      <span>
        {#if changes}
          {files.length} file{files.length > 1 ? 's' : ''} changed in the folder · {mine} edited by {agent.name}
        {:else}
          Changes by {agent.name} · {files.length} file{files.length > 1 ? 's' : ''}
        {/if}
      </span>
    </div>
    <div class="modes" role="group" aria-label="Diff layout">
      <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>Unified</button>
      <button aria-pressed={layout.split} onclick={() => setSplit(true)}>Split</button>
    </div>
    <button
      class="icon"
      aria-label={layout.diffFull ? 'Exit full screen' : 'Full screen'}
      aria-pressed={layout.diffFull}
      title={layout.diffFull ? 'Exit full screen' : 'Full screen'}
      onclick={toggleDiffFull}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true">
        {#if layout.diffFull}
          <path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7" />
        {:else}
          <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" />
        {/if}
      </svg>
    </button>
    <button class="icon" aria-label="Close panel" onclick={onclose}>×</button>
  </div>

  {#if files.length}
    <div class="tabs" role="tablist" aria-label="Changed files">
      {#each files as f (f.path)}
        <button
          role="tab"
          aria-selected={f === selected}
          class:other={!f.edits}
          title={f.edits ? f.path : `${f.path} · not edited by ${agent.name}`}
          onclick={() => (file = f.path)}
        >
          {name(f.path)} <span class="add">+{f.add}</span>{#if f.del}<span class="del">−{f.del}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="body">
      {#if selected}
        <div class="file">
          <div class="file-head">
            {#if shown}
              <span class="badge" class:new={shown.status === 'NEW'} class:gone={shown.status === 'DELETED'}>{shown.status}</span>
            {/if}
            <span class="path">{selected.path}</span>
            <span class="by">{selected.edits ? `${selected.edits} edit${selected.edits > 1 ? 's' : ''} by ${agent.name}` : `Not edited by ${agent.name}`}</span>
          </div>
          {#if error}
            <p class="empty">{error}</p>
          {:else if !shown}
            <p class="empty">Reading the file…</p>
          {:else if shown.binary}
            <p class="empty">Binary file, no preview.</p>
          {:else if shown.hunks.length}
            <DiffView
              hunks={shown.hunks}
              split={layout.split}
              oncomment={open}
              {marked}
              open={commenting?.path === selected.path ? commenting : null}
              {editor}
            />
          {:else}
            <p class="empty">The file is back to its original state.</p>
          {/if}
        </div>
      {/if}
    </div>

    <div class="foot">
      <span>
        {changes
          ? agent.base
            ? `Everything that changed in the folder since ${agent.name} started`
            : 'Uncommitted changes of the folder'
          : 'Diff since the agent first touched the file'}
      </span>
      <button class="btn btn-secondary" onclick={() => invoke('open_in_editor', { project: agent.project, path: selected.path })}>
        Open in editor
      </button>
    </div>
  {:else}
    <p class="empty pad">{changes ? 'Nothing changed in the folder.' : `${agent.name} has not changed any file yet.`}</p>
  {/if}
</section>

{#snippet editor()}
  <form class="inline" onsubmit={comment}>
    <label for="line-comment" class="sr-only">Comment on line {commenting?.number}</label>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea
      id="line-comment"
      class="field"
      rows="3"
      bind:value={note}
      placeholder="What should {agent.name} change here?"
      autofocus
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation()
          commenting = null
        } else if (e.ctrlKey && e.key === 'Enter') comment(e)
      }}
    ></textarea>
    <div class="actions">
      {#if existing >= 0}
        <button type="button" class="btn btn-danger" onclick={remove}>Delete</button>
      {/if}
      <span class="hint">{existing >= 0 ? 'In your next message' : 'Added to your next message'} · <kbd>Ctrl ↵</kbd></span>
      <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>Cancel</button>
      <button class="btn btn-primary" disabled={!note.trim()}>{existing >= 0 ? 'Save' : 'Add comment'}</button>
    </div>
  </form>
{/snippet}

<style>
  .panel {
    position: relative;
    min-width: 420px;
    flex: 0 1 auto;
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

  .panel.full {
    position: absolute;
    inset: 0;
    z-index: 20;
    min-width: 0;
    border-left: none;
  }

  .resizer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    z-index: 2;
    width: 6px;
    cursor: col-resize;
  }

  .resizer:hover,
  .resizer:focus-visible,
  .resizer:active {
    background: var(--border-strong);
    outline: none;
  }

  .modes {
    display: flex;
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .modes button {
    height: 26px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .modes button[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }

  .icon:hover,
  .icon[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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

  .tabs button.other {
    color: var(--text-dim);
    box-shadow: inset 0 0 0 1px var(--border);
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

  .inline {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 6px 12px 10px 48px;
    padding: 10px;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 8px 24px rgba(0, 0, 0, 0.35);
    font: 13.5px/1.4 var(--sans);
    color: var(--text);
  }

  .inline textarea {
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }

  .inline .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .inline .btn {
    height: 32px;
    padding: 0 12px;
  }

  .btn-danger {
    background: #3a2420;
    color: #f8d5cc;
  }

  .hint {
    flex: 1;
    font-size: 12px;
    color: var(--text-dim);
  }

  kbd {
    font-family: var(--mono);
  }
</style>
