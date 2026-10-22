<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { parsePatch, structuredPatch } from 'diff'
  import { changedFiles, type Changes, type Hunk, type Proposal } from './diff'
  import DiffView, { type Line } from './DiffView.svelte'
  import { layout, setSplit } from './layout.svelte'
  import OpenWith from './OpenWith.svelte'
  import SidePanel from './SidePanel.svelte'
  import { calls, relative, workdir, type Agent, type Comment } from './stream'

  let {
    agent,
    changes,
    proposal = null,
    file = $bindable(),
    comments = $bindable(),
    target = $bindable(),
    onclose,
  }: {
    agent: Agent
    changes: Changes | null
    proposal?: Proposal | null
    file: string
    comments: Comment[]
    target: Comment | null
    onclose: () => void
  } = $props()

  const STATUS: Record<string, string> = { A: 'NEW', D: 'DELETED' }

  const edited = $derived(changedFiles(calls(agent)).map((f) => ({ ...f, path: relative(agent, f.path) })))
  const found = $derived(
    changes
      ? changes.files
          .map((f) => ({ path: f.path, add: f.add ?? 0, del: f.del ?? 0, edits: edited.find((e) => e.path === f.path)?.calls.length ?? 0 }))
          .sort((a, b) => Number(b.edits > 0) - Number(a.edits > 0))
      : edited.map((f) => ({ path: f.path, add: f.add, del: f.del, edits: f.calls.length })),
  )
  const files = $derived(
    proposal && !found.some((f) => f.path === proposal.path) ? [{ path: proposal.path, add: 0, del: 0, edits: 0 }, ...found] : found,
  )
  const mine = $derived(files.filter((f) => f.edits).length)
  const selected = $derived(files.find((f) => f.path === file) ?? files[0])
  const proposing = $derived(!!proposal && selected?.path === proposal.path)

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
      ? invoke<string>('git_patch', { project: workdir(agent), from: git.from, now: git.now, path }).then((text) => ({
          path,
          hunks: (parsePatch(text)[0]?.hunks ?? []) as Hunk[],
          status: STATUS[git.status] ?? 'MODIFIED',
          binary: /^Binary files /m.test(text),
        }))
      : invoke<string | null>('read_project_file', { project: workdir(agent), path }).then((current) => ({
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

  const shown = $derived(
    proposing && proposal
      ? { path: proposal.path, hunks: proposal.hunks, status: 'PROPOSED', binary: false }
      : view?.path === selected?.path
        ? view
        : null,
  )
  const base = (path: string) => path.split('/').pop()!
  const name = (path: string) => (files.filter((f) => base(f.path) === base(path)).length > 1 ? path.split('/').slice(-2).join('/') : base(path))

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

  $effect(() => {
    if (!target || shown?.path !== target.path) return
    commenting = { side: target.removed ? 'old' : 'new', number: target.line, text: target.code, path: target.path }
    note = target.note
    target = null
  })

  const reveal = (el: HTMLElement) => el.scrollIntoView({ block: 'center' })

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

<SidePanel kind="diff" size={layout.split ? 'split' : 'unified'} title="Diff" {onclose}>
  {#snippet sub()}
    {#if changes}
      {files.length} uncommitted file{files.length > 1 ? 's' : ''} · {mine} edited by {agent.name}
    {:else}
      Changes by {agent.name} · {files.length} file{files.length > 1 ? 's' : ''}
    {/if}
  {/snippet}
  {#snippet actions()}
    <div class="modes" role="group" aria-label="Diff layout">
      <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>Unified</button>
      <button aria-pressed={layout.split} onclick={() => setSplit(true)}>Split</button>
    </div>
  {/snippet}
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
              <span class="badge" class:new={shown.status === 'NEW'} class:gone={shown.status === 'DELETED'} class:proposed={proposing}>{shown.status}</span>
            {/if}
            <span class="path">{selected.path}</span>
            <span class="by"
              >{proposing
                ? `Proposed by ${agent.name} · not applied yet`
                : selected.edits
                  ? `${selected.edits} edit${selected.edits > 1 ? 's' : ''} by ${agent.name}`
                  : `Not edited by ${agent.name}`}</span
            >
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
              oncomment={proposing ? undefined : open}
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
        {changes ? 'What is not committed yet, compared to the last commit' : 'Diff since the agent first touched the file'}
      </span>
      <OpenWith project={workdir(agent)} path={selected.path} up />
    </div>
  {:else}
    <p class="empty pad">{changes ? 'Nothing changed in the folder.' : `${agent.name} has not changed any file yet.`}</p>
  {/if}
</SidePanel>

{#snippet editor()}
  <form class="inline" onsubmit={comment} use:reveal>
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

  .tabs {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    max-height: 112px;
    overflow-y: auto;
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

  .badge.proposed {
    background: var(--wait-bg);
    color: var(--accent-soft);
  }

  .badge.gone {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .path {
    flex: 1 1 auto;
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
