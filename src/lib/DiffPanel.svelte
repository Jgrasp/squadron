<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import { parsePatch, structuredPatch } from 'diff'
  import { changedFiles, type Changes, type Hunk, type Proposal } from './diff'
  import DiffView, { type Line } from './DiffView.svelte'
  import ChangedFiles from './ChangedFiles.svelte'
  import { layout, setFileTree, setSplit } from './layout.svelte'
  import OpenWith from './OpenWith.svelte'
  import Markdown from './Markdown.svelte'
  import { ask } from '@tauri-apps/plugin-dialog'
  import { hostName, post, reviews, type Pull, type Remark } from './pulls.svelte'
  import SidePanel from './SidePanel.svelte'
  import { calls, relative, workdir, type Agent, type Comment } from './stream'

  let {
    agent,
    changes,
    proposal = null,
    file = $bindable(),
    comments = $bindable(),
    target = $bindable(),
    pull = null,
    mode = $bindable(),
    onhand,
    onreview,
    onclose,
  }: {
    agent: Agent
    changes: Changes | null
    proposal?: Proposal | null
    file: string
    comments: Comment[]
    target: Comment | null
    pull?: Pull | null
    mode: 'local' | 'pull'
    onhand: (quote: string) => void
    onreview: (summary: string, list: Comment[]) => void
    onclose: () => void
  } = $props()

  const reviewing = $derived(mode === 'pull' && !!pull)
  const keyOf = (p: Pull) => `${p.number}:${p.head}`
  let pulled = $state<{ key: string; changes: Changes; ahead: number } | null>(null)
  let pullError = $state('')
  let attempt = $state(0)

  $effect(() => {
    void attempt
    if (!reviewing || !pull) return
    const key = keyOf(pull)
    if (untrack(() => pulled?.key) === key) return
    pullError = ''
    const { base, branch, head } = pull
    invoke<{ changes: Changes; ahead: number }>('pull_changes', { dir: workdir(agent), base, branch, head })
      .then((result) => (pulled = { key, ...result }))
      .catch((e) => (pullError = String(e)))
  })

  let remarks = $state<Remark[]>([])

  $effect(() => {
    if (!reviewing || !pull) return
    const { number } = pull
    invoke<Remark[]>('pull_remarks', { dir: workdir(agent), number })
      .then((found) => (remarks = found))
      .catch(() => (remarks = []))
  })

  type Thread = { root: Remark; all: Remark[] }
  const threads = $derived.by(() => {
    const byId = new Map(remarks.map((r) => [r.id, r]))
    const rootOf = (r: Remark) => {
      for (let i = 0; r.reply_to !== null && byId.has(r.reply_to) && i < 100; i++) r = byId.get(r.reply_to)!
      return r
    }
    const list = new Map<number, Thread>()
    for (const r of remarks) {
      const root = rootOf(r)
      if (!list.has(root.id)) list.set(root.id, { root, all: [] })
      list.get(root.id)!.all.push(r)
    }
    return [...list.values()]
  })
  const general = $derived(threads.filter((t) => !t.root.path || t.root.line === null))
  const threadsAt = (path: string, line: Line) =>
    threads.filter((t) => t.root.path === path && t.root.line === line.number && t.root.removed === (line.side === 'old'))
  const talks = (path: string) => threads.filter((t) => t.root.path === path && t.root.line !== null).length
  const when = (at: string) => new Date(at).toLocaleString('en-GB', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
  let handed = $state<number[]>([])

  function hand(thread: Thread, line?: Line) {
    const said = thread.all.map((r) => `${r.author}: ${r.body}`).join('\n')
    if (line && thread.root.path) comments.push({ path: thread.root.path, line: line.number, removed: line.side === 'old', code: line.text, note: `Review on the pull request\n${said}` })
    else {
      const where = thread.root.path ? ` about ${thread.root.path}` : ''
      onhand(`Review on pull request #${pull!.number}${where}:\n${said.split('\n').map((l) => `> ${l}`).join('\n')}`)
    }
    handed.push(thread.root.id)
  }

  const source = $derived(reviewing ? (pull && pulled?.key === keyOf(pull) ? pulled.changes : null) : changes)

  const STATUS: Record<string, string> = { A: 'NEW', D: 'DELETED' }

  const edited = $derived(changedFiles(calls(agent)).map((f) => ({ ...f, path: relative(agent, f.path) })))
  const found = $derived(
    source
      ? source.files
          .map((f) => ({ path: f.path, add: f.add ?? 0, del: f.del ?? 0, edits: edited.find((e) => e.path === f.path)?.calls.length ?? 0 }))
          .sort((a, b) => Number(b.edits > 0) - Number(a.edits > 0))
      : reviewing
        ? []
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
    const git = source && { from: source.from, now: source.now, status: source.files.find((f) => f.path === path)?.status ?? '' }
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

  const review = $derived(reviewing ? (reviews[agent.id] ?? []) : [])
  const here = (c: Comment) => (c.pull ?? null) === (reviewing ? pull!.number : null)
  const at = (path: string, line: Pick<Line, 'side' | 'number'>) => (c: Comment) =>
    c.path === path && c.line === line.number && c.removed === (line.side === 'old')
  function find(path: string, line: Pick<Line, 'side' | 'number'>) {
    for (const list of [review, comments]) {
      const index = list.findIndex((c) => here(c) && at(path, line)(c))
      if (index >= 0) return { list, index, inReview: list === review }
    }
    return null
  }
  const existing = $derived(commenting ? find(commenting.path, commenting) : null)
  const marked = $derived(
    [...review, ...comments.filter(here)]
      .filter((c) => c.path === selected?.path)
      .map((c) => ({ side: c.removed ? ('old' as const) : ('new' as const), number: c.line, note: c.note })),
  )

  function open(line: Line) {
    commenting = { ...line, path: selected.path }
    const found = find(selected.path, line)
    note = found ? found.list[found.index].note : ''
  }

  $effect(() => {
    if (!target || shown?.path !== target.path) return
    commenting = { side: target.removed ? 'old' : 'new', number: target.line, text: target.code, path: target.path }
    note = target.note
    target = null
  })

  const reveal = (el: HTMLElement) => el.scrollIntoView({ block: 'center' })

  function draft(): Comment {
    const { path, side, number, text } = commenting!
    return { path, line: number, removed: side === 'old', code: text, note: note.trim(), ...(reviewing ? { pull: pull!.number } : {}) }
  }

  function comment(event?: Event) {
    event?.preventDefault()
    if (!commenting || !note.trim()) return
    if (existing) existing.list[existing.index].note = note.trim()
    else if (reviewing) reviews[agent.id] = [...(reviews[agent.id] ?? []), draft()]
    else comments.push(draft())
    commenting = null
  }

  function forAgent() {
    if (!commenting || !note.trim()) return
    comments.push(draft())
    commenting = null
  }

  let posting = $state(false)
  let postError = $state('')
  let finishing = $state(false)
  let summary = $state('')

  async function postNow() {
    if (!commenting || !note.trim() || !pull) return
    const { path, side, number, text } = commenting
    posting = true
    postError = ''
    const { error } = await post(workdir(agent), pull, [{ path, line: number, removed: side === 'old', code: text, note: note.trim() }])
    posting = false
    if (error) return (postError = error)
    if (existing) existing.list.splice(existing.index, 1)
    commenting = null
  }

  async function submit() {
    if (!pull || !review.length) return
    const sending = [...review]
    posting = true
    postError = ''
    const { done, error } = await post(workdir(agent), pull, sending, summary)
    posting = false
    reviews[agent.id] = review.filter((c) => !sending.slice(0, done).includes(c))
    if (error) return (postError = done ? `${done} posted, then: ${error}` : error)
    summary = ''
    finishing = false
  }

  function handOver() {
    const list = $state.snapshot(review)
    reviews[agent.id] = []
    finishing = false
    onreview(summary.trim(), list)
    summary = ''
  }

  async function discard() {
    const sure = await ask(`Discard the review and its ${review.length} comment${review.length > 1 ? 's' : ''}?`, {
      title: 'Discard the review',
      kind: 'warning',
      okLabel: 'Discard',
      cancelLabel: 'Keep it',
    })
    if (!sure) return
    reviews[agent.id] = []
    finishing = false
  }

  function remove() {
    if (existing) existing.list.splice(existing.index, 1)
    commenting = null
  }
</script>

<SidePanel kind="diff" size={layout.split ? 'split' : 'unified'} title="Diff" {onclose}>
  {#snippet sub()}
    {#if reviewing && pull}
      PR #{pull.number} · {pull.branch} → {pull.base}{source ? ` · ${files.length} file${files.length > 1 ? 's' : ''}` : ''}
    {:else if changes}
      {files.length} uncommitted file{files.length > 1 ? 's' : ''} · {mine} edited by {agent.name}
    {:else}
      Changes by {agent.name} · {files.length} file{files.length > 1 ? 's' : ''}
    {/if}
  {/snippet}
  {#snippet actions()}
    <button
      class="tree-toggle"
      aria-pressed={layout.fileTree}
      aria-label="File tree"
      title="{layout.fileTree ? 'Hide' : 'Show'} the file tree"
      onclick={() => setFileTree(!layout.fileTree)}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5h6M4 12h6M4 19h6M14 5h6M16 12h4M16 19h4M14 5v14" /></svg>
    </button>
    {#if pull}
      <div class="modes" role="group" aria-label="What to compare">
        <button aria-pressed={!reviewing} onclick={() => (mode = 'local')}>Changes</button>
        <button aria-pressed={reviewing} onclick={() => (mode = 'pull')}>PR #{pull.number}</button>
      </div>
    {/if}
    <div class="modes" role="group" aria-label="Diff layout">
      <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>Unified</button>
      <button aria-pressed={layout.split} onclick={() => setSplit(true)}>Split</button>
    </div>
  {/snippet}
  {#if reviewing && pull && review.length}
    <div class="review-bar">
      <span class="pulse" aria-hidden="true"></span>
      <span class="what"><strong>Review in progress</strong> · {review.length} comment{review.length > 1 ? 's' : ''}</span>
      {#if postError && !commenting}<span class="bad" role="alert">{postError}</span>{/if}
      <button class="btn btn-secondary" onclick={discard}>Discard</button>
      <button class="btn btn-primary" aria-expanded={finishing} onclick={() => (finishing = !finishing)}>Finish review</button>
    </div>
    {#if finishing}
      <div class="finish">
        <label for="review-summary" class="sr-only">Overall comment</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea id="review-summary" class="field" rows="3" bind:value={summary} placeholder="Overall comment, optional" autofocus></textarea>
        <div class="actions">
          <span class="hint">Send the {review.length} comment{review.length > 1 ? 's' : ''} to {agent.name}, or post the review on {hostName(pull)}</span>
          <button class="btn btn-secondary" disabled={posting} onclick={submit}>{posting ? 'Posting…' : `Post on ${hostName(pull)}`}</button>
          <button class="btn btn-primary" disabled={posting} onclick={handOver}>Send to {agent.name}</button>
        </div>
      </div>
    {/if}
  {/if}
  {#if reviewing && pulled && pulled.ahead > 0 && source}
    <p class="ahead" role="note">
      {pulled.ahead} commit{pulled.ahead > 1 ? 's' : ''} of the branch {pulled.ahead > 1 ? 'are' : 'is'} not pushed to the pull request yet: lines can differ from {hostName(pull!)}.
    </p>
  {/if}
  {#if reviewing && !source}
    <div class="fetching" role="status">
      {#if pullError}
        <svg class="failed" width="34" height="34" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" /><path d="M12 7v6M12 16.5v.5" /></svg>
        <strong>Pull request #{pull!.number} could not be read</strong>
        <span class="why">{pullError}</span>
        <button class="btn btn-secondary" onclick={() => attempt++}>Try again</button>
      {:else}
        <svg class="ring" width="34" height="34" viewBox="0 0 34 34" aria-hidden="true"><circle class="track" cx="17" cy="17" r="14" /><circle class="arc" cx="17" cy="17" r="14" /></svg>
        <strong>Fetching pull request #{pull!.number}</strong>
        <span class="branches">{pull!.branch} → {pull!.base}</span>
        <span class="why">Getting both branches from {hostName(pull!)}, then building the diff on this computer</span>
      {/if}
    </div>
  {:else if files.length}
    {#if !layout.fileTree}
    <div class="tabs" role="tablist" aria-label="Changed files">
      {#each files as f (f.path)}
        <button
          role="tab"
          aria-selected={f === selected}
          class:other={!f.edits && !reviewing}
          title={f.edits ? f.path : `${f.path} · not edited by ${agent.name}`}
          onclick={() => (file = f.path)}
        >
          {name(f.path)} <span class="add">+{f.add}</span>{#if f.del}<span class="del">−{f.del}</span>{/if}
          {#if reviewing && talks(f.path)}<span class="talks" title="{talks(f.path)} review thread{talks(f.path) > 1 ? 's' : ''}">{talks(f.path)}</span>{/if}
        </button>
      {/each}
    </div>

    {/if}

    <div class="columns">
    {#if layout.fileTree}
      <ChangedFiles
        files={files.map((f) => ({ ...f, other: !f.edits && !reviewing, talks: reviewing ? talks(f.path) : 0 }))}
        selected={selected?.path}
        onpick={(path) => (file = path)}
      />
    {/if}
    <div class="body">
      {#if reviewing && general.length}
        <details class="general" open>
          <summary>{general.length} comment{general.length > 1 ? 's' : ''} on the pull request</summary>
          {#each general as t (t.root.id)}{@render thread(t)}{/each}
        </details>
      {/if}
      {#if selected}
        <div class="file">
          <div class="file-head">
            {#if shown}
              <span class="badge" class:new={shown.status === 'NEW'} class:gone={shown.status === 'DELETED'} class:proposed={proposing}>{shown.status}</span>
            {/if}
            <span class="path">{selected.path}</span>
            <span class="by" hidden={reviewing}
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
              {below}
            />
          {:else}
            <p class="empty">The file is back to its original state.</p>
          {/if}
        </div>
      {/if}
    </div>
    </div>

    <div class="foot">
      <span>
        {reviewing && pull
          ? `What the pull request changes, compared to ${pull.base}`
          : changes
            ? 'What is not committed yet, compared to the last commit'
            : 'Diff since the agent first touched the file'}
      </span>
      {#if reviewing && pull}
        <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: pull.url })}>Open on {hostName(pull)}</button>
      {:else}
        <OpenWith project={workdir(agent)} path={selected.path} up />
      {/if}
    </div>
  {:else}
    <p class="empty pad">{changes ? 'Nothing changed in the folder.' : `${agent.name} has not changed any file yet.`}</p>
  {/if}
</SidePanel>

{#snippet below(line: Line)}
  {#if reviewing && selected}
    {#each threadsAt(selected.path, line) as t (t.root.id)}{@render thread(t, line)}{/each}
  {/if}
{/snippet}

{#snippet thread(t: Thread, line?: Line)}
  <div class="thread" class:inline-thread={!!line}>
    {#if !line && t.root.path}<span class="where">{t.root.path} · outdated</span>{/if}
    {#each t.all as r (r.id)}
      <div class="remark" class:reply={r !== t.root}>
        <span class="who"><strong>{r.author}</strong> · {when(r.at)}</span>
        <Markdown text={r.body} />
      </div>
    {/each}
    <div class="thread-actions">
      {#if handed.includes(t.root.id)}
        <span class="handed">In your next message to {agent.name}</span>
      {:else}
        <button class="btn btn-secondary" onclick={() => hand(t, line)}>Hand to {agent.name}</button>
      {/if}
    </div>
  </div>
{/snippet}

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
      {#if existing}
        <button type="button" class="btn btn-danger" onclick={remove}>Delete</button>
        <span class="hint">{existing.inReview ? 'In the review' : 'In your next message'} · <kbd>Ctrl ↵</kbd></span>
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={!note.trim()}>Save</button>
      {:else if reviewing && pull && review.length}
        <span class="hint">Review in progress · {review.length} comment{review.length > 1 ? 's' : ''} · <kbd>Ctrl ↵</kbd></span>
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={!note.trim()}>Add to review</button>
      {:else if reviewing && pull}
        {#if postError}<span class="hint bad" role="alert">{postError}</span>{:else}<span class="hint"><kbd>Ctrl ↵</kbd> starts a review</span>{/if}
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>Cancel</button>
        <button type="button" class="btn btn-ghost" title="Posts this comment on {hostName(pull)} right away" disabled={!note.trim() || posting} onclick={postNow}>
          {posting ? 'Posting…' : `Post on ${hostName(pull)}`}
        </button>
        <button type="button" class="btn btn-secondary" title="Adds it to your next message to {agent.name}" disabled={!note.trim()} onclick={forAgent}>Add comment</button>
        <button class="btn btn-primary" title="Gathers this comment and the next ones; send them to {agent.name} or post them on {hostName(pull)} at the end" disabled={!note.trim()}>
          Start a review
        </button>
      {:else}
        <span class="hint">Added to your next message · <kbd>Ctrl ↵</kbd></span>
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={!note.trim()}>Add comment</button>
      {/if}
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

  .talks {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--wait-bg);
    color: var(--accent-soft);
    font: 600 11px/18px var(--sans);
    text-align: center;
  }

  .general {
    margin-bottom: 14px;
    border-radius: 6px;
    box-shadow: 0 0 0 1px var(--border);
    background: var(--surface-1);
  }

  .general summary {
    padding: 10px 12px;
    cursor: pointer;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
  }

  .thread {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
    font: 13px/1.5 var(--sans);
    color: var(--text);
  }

  .inline-thread {
    margin: 6px 12px 10px 48px;
    border: none;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 3px 0 0 var(--accent-soft), inset 0 0 0 1px var(--border-strong);
  }

  .where {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .remark.reply {
    padding-left: 14px;
    border-left: 2px solid var(--border-strong);
  }

  .who {
    display: block;
    margin-bottom: 2px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .who strong {
    color: var(--text-2);
  }

  .thread-actions {
    display: flex;
    justify-content: flex-end;
  }

  .thread-actions .btn {
    height: 30px;
    padding: 0 12px;
  }

  .handed {
    font-size: 12px;
    color: var(--ok-text);
  }

  .review-bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--wait-bg);
    font-size: 13px;
  }

  .review-bar .what {
    flex: 1;
    color: var(--accent-soft);
  }

  .review-bar .what strong {
    color: var(--text-strong);
  }

  .review-bar .btn {
    height: 30px;
    padding: 0 12px;
  }

  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(224, 138, 91, 0.25);
  }

  .finish {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
  }

  .finish textarea {
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }

  .finish .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .finish .btn {
    height: 32px;
    padding: 0 12px;
  }

  .fetching {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }

  .fetching strong {
    margin-top: 8px;
    font-size: 15px;
    color: var(--text-strong);
  }

  .branches {
    font: 12.5px var(--mono);
    color: var(--text-2);
  }

  .why {
    max-width: 420px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .fetching .btn {
    margin-top: 8px;
  }

  .ring {
    fill: none;
    stroke-width: 3;
    animation: spin 1.1s linear infinite;
  }

  .ring .track {
    stroke: var(--surface-4);
  }

  .ring .arc {
    stroke: var(--accent);
    stroke-linecap: round;
    stroke-dasharray: 22 66;
  }

  .failed {
    fill: none;
    stroke: var(--err);
    stroke-width: 2;
    stroke-linecap: round;
  }

  @media (prefers-reduced-motion: reduce) {
    .ring {
      animation-duration: 3s;
    }
  }

  .tree-toggle {
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-muted);
  }

  .tree-toggle svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .tree-toggle[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .ahead {
    flex: none;
    margin: 0;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--wait-bg);
    color: var(--accent-soft);
    font-size: 12.5px;
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
    min-width: 0;
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

  .bad,
  .foot .bad {
    color: var(--err-text);
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
