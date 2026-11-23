<script lang="ts" module>
  import type { Person } from './ChangedFiles.svelte'

  const selves: Record<string, Promise<Person | null>> = {}
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick, untrack, type Snippet } from 'svelte'
  import { slide } from 'svelte/transition'
  import { parsePatch, structuredPatch } from 'diff'
  import { changedFiles, rows, type Changes, type Hunk, type Proposal } from './diff'
  import DiffView, { type Line } from './DiffView.svelte'
  import ChangedFiles, { type Note } from './ChangedFiles.svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import { layout, setFileTree, setSplit, setThreads, setThreadsWidth } from './layout.svelte'
  import { settle } from './unread.svelte'
  import OpenWith from './OpenWith.svelte'
  import Markdown from './Markdown.svelte'
  import { ask } from './asking.svelte'
  import { show } from './agents.svelte'
  import { settings } from './settings.svelte'
  import { behind, closeRound, hostName, inRound, judgementOf, markReviewed, openRound, post, reviewChanges, reviews, setViewed, viewedFiles, type Pull, type Remark } from './pulls.svelte'
  import SidePanel from './SidePanel.svelte'
  import PullThreads, { stateOf, type ThreadState } from './PullThreads.svelte'
  import { calls, relative, workdir, type Agent, type Comment } from './stream'
  import Trans from './Trans.svelte'
  import { can, offers } from './forges.svelte'
  import { date, errorText, t, type Key } from './i18n/index.svelte'

  let {
    agent,
    dir,
    changes,
    proposal = null,
    file = $bindable(),
    comments = $bindable(),
    target = $bindable(),
    pull = null,
    mode = $bindable(),
    onhand,
    onreview,
    onposted,
    reviewer = null,
    handTo = null,
    onagent,
    since = null,
    own = false,
    banner,
    back,
    onclose,
  }: {
    agent: Agent | null
    dir?: string
    changes: Changes | null
    proposal?: Proposal | null
    file: string
    comments: Comment[]
    target: Comment | null
    pull?: Pull | null
    mode: 'local' | 'pull' | 'branch'
    onhand: (quote: string) => void
    onreview: (summary: string, list: Comment[]) => void
    onposted?: () => void
    reviewer?: Agent | null
    handTo?: Agent | 'new' | null
    onagent?: () => Promise<unknown>
    since?: string | null
    own?: boolean
    banner?: Snippet
    back?: () => void
    onclose: () => void
  } = $props()

  const folder = $derived(dir ?? workdir(agent!))
  const reviewing = $derived(mode === 'pull' && !!pull)
  const reviewKey = $derived(
    agent?.review && agent.review.number === pull?.number ? `${agent.review.project}#${agent.review.number}` : (agent?.id ?? `${folder}#${pull?.number}`),
  )
  let onlyNew = $state(untrack(() => !!since))
  let sinceError = $state('')
  const keyOf = (p: Pull) => `${p.number}:${p.head}:${onlyNew ? since : ''}`
  let pulled = $state<{ key: string; changes: Changes; ahead: number; added: number } | null>(null)
  let pullError = $state('')
  let attempt = $state(0)

  $effect(() => {
    void attempt
    if (!reviewing || !pull) return
    const key = keyOf(pull)
    if (untrack(() => pulled?.key) === key) return
    pullError = ''
    const { base, branch, head } = pull
    const from = onlyNew ? since : null
    const stale = () => !pull || keyOf(pull) !== key
    invoke<{ changes: Changes; ahead: number; added: number }>('pull_changes', { dir: folder, base, branch, head, since: from })
      .then((result) => {
        if (!stale()) pulled = { key, ...result }
      })
      .catch((e) => {
        if (stale()) return
        if (!from) return (pullError = String(e))
        sinceError = String(e)
        onlyNew = false
      })
  })

  const branching = $derived(mode === 'branch' && !reviewing && !!changes)
  let branched = $state<{ changes: Changes; base: string; commits: number } | null>(null)
  let branchError = $state('')

  $effect(() => {
    if (!branching) return
    void changes?.now
    invoke<{ changes: Changes; base: string; commits: number }>('branch_changes', { project: folder })
      .then((found) => ((branched = found), (branchError = '')))
      .catch((e) => (branchError = String(e)))
  })

  let remarks = $state<Remark[]>([])

  let fresh = $state(0)

  $effect(() => {
    void fresh
    if (!reviewing || !pull) return
    const { number } = pull
    const key = reviewKey
    invoke<Remark[]>('pull_remarks', { dir: folder, number })
      .then((found) => ((remarks = found), settle(key, found)))
      .catch(() => (remarks = []))
  })

  type Thread = { root: Remark; all: Remark[] }
  const threads = $derived.by(() => {
    const byId = new Map(remarks.map((r) => [r.id, r]))
    const rootOf = (r: Remark) => {
      for (let i = 0; r.reply_to !== null && byId.has(r.reply_to) && i < 100; i++) r = byId.get(r.reply_to)!
      return r
    }
    const list = new Map<string, Thread>()
    for (const r of remarks) {
      const root = rootOf(r)
      if (!list.has(root.id)) list.set(root.id, { root, all: [] })
      list.get(root.id)!.all.push(r)
    }
    return [...list.values()]
  })
  const general = $derived(threads.filter((t) => !t.root.path || t.root.line === null))
  const STATE_KEYS: Record<ThreadState, Key> = {
    open: 'review.threadOpen',
    answered: 'review.threadAnswered',
    fixed: 'review.threadFixed',
    resolved: 'review.threadResolved',
    outdated: 'review.threadOutdated',
  }
  const pending = $derived(threads.filter((th) => ['open', 'answered', 'fixed'].includes(stateOf(reviewKey, th))))
  const opened = $derived(threads.filter((th) => stateOf(reviewKey, th) === 'open').length)
  const resolvables = $derived(pull && can(pull.host, 'resolve') ? pending.filter((th) => th.root.resolved === false) : [])
  let toResolve = $state<string[]>([])

  const rereader = $derived(reviewer ?? (agent?.review ? agent : null))
  const rereadable = $derived(!!pull && !!rereader?.review && rereader.status !== 'working' && behind(rereader, pull))
  let rereading = $state(false)

  async function reReview() {
    if (!pull || !rereader) return
    rereading = true
    postError = null
    try {
      await reviewChanges(rereader, pull)
    } catch (e) {
      postError = { text: String(e), done: 0 }
    } finally {
      rereading = false
    }
  }
  $effect(() => {
    if (finishing) untrack(() => (toResolve = resolvables.filter((th) => stateOf(reviewKey, th) === 'fixed').map((th) => th.root.id)))
  })

  let picked = $state('')
  function goToThread(th: Thread) {
    const there = spots.find((s) => s.key === `thread:${th.root.id}`)
    picked = there ? '' : th.root.id
    if (there) return goToSpot(there)
    const el = body?.querySelector(`[data-thread="${th.root.id}"]`)
    if (!el) return th.root.path && scrollToFile(th.root.path)
    el.closest('details')?.setAttribute('open', '')
    within(el, 'start')
  }

  function dragThreads(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const from = event.clientX
    const width = layout.threadsWidth
    handle.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => (e.buttons ? setThreadsWidth(width + from - e.clientX) : up())
    const up = () => (handle.removeEventListener('pointermove', move), handle.removeEventListener('pointerup', up))
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }

  const goToDraft = (c: Comment) => goTo(c)
  const threadsAt = (path: string, line: Line) =>
    threads.filter((t) => t.root.path === path && t.root.line === line.number && t.root.removed === (line.side === 'old'))
  const talks = (path: string) => threads.filter((t) => t.root.path === path && t.root.line !== null).length
  const when = (at: string) => date(new Date(at), { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
  let handed = $state<string[]>([])
  let replying = $state<string | null>(null)
  let answer = $state('')
  let answering = $state(false)
  let replyError = $state('')

  async function reply(thread: Thread, event: Event) {
    event.preventDefault()
    if (!pull || !answer.trim()) return
    answering = true
    replyError = ''
    try {
      await invoke('pull_reply', { dir: folder, number: pull.number, to: thread.root.id, kind: thread.root.kind, body: answer })
      replying = null
      answer = ''
      fresh++
    } catch (e) {
      replyError = String(e)
    } finally {
      answering = false
    }
  }

  function hand(thread: Thread, line?: Line) {
    const said = thread.all.map((r) => `${r.author}: ${r.body}`).join('\n')
    if (line && thread.root.path) comments.push({ path: thread.root.path, line: line.number, removed: line.side === 'old', code: line.text, note: `Review on the pull request\n${said}` })
    else {
      const where = thread.root.path ? ` about ${thread.root.path}` : ''
      onhand(`Review on pull request #${pull!.number}${where}:\n${said.split('\n').map((l) => `> ${l}`).join('\n')}`)
    }
    handed.push(thread.root.id)
  }

  const source = $derived(reviewing ? (pull && pulled?.key === keyOf(pull) ? pulled.changes : null) : branching ? (branched?.changes ?? null) : changes)

  const STATUS: Record<string, string> = { A: 'NEW', D: 'DELETED' }
  const BADGES: Record<string, Key> = { NEW: 'review.statusNew', DELETED: 'review.statusDeleted', MODIFIED: 'review.statusModified', PROPOSED: 'review.statusProposed' }
  const badge = (status: string) => (BADGES[status] ? t(BADGES[status]) : status)

  const edited = $derived(agent ? changedFiles(calls(agent)).map((f) => ({ ...f, path: relative(agent, f.path) })) : [])
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
    if (!path || reviewing) return
    const git = source && { from: source.from, now: source.now, status: source.files.find((f) => f.path === path)?.status ?? '' }
    const baseline = edited.find((f) => f.path === path)?.baseline ?? null
    void selected.edits
    const run = ++loads
    error = ''
    const loading = git
      ? invoke<string>('git_patch', { project: folder, from: git.from, now: git.now, path }).then((text) => ({
          path,
          hunks: (parsePatch(text)[0]?.hunks ?? []) as Hunk[],
          status: STATUS[git.status] ?? 'MODIFIED',
          binary: /^Binary files /m.test(text),
        }))
      : invoke<string | null>('read_project_file', { project: folder, path }).then((current) => ({
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

  const review = $derived(reviewing ? (reviews[reviewKey] ?? []) : [])
  const here = (c: Comment) => !c.page && (c.pull ?? null) === (reviewing ? pull!.number : null)
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

  let me = $state<Person | null>(null)

  $effect(() => {
    if (!reviewing && !comments.some(here)) return
    const dir = folder
    ;(selves[dir] ??= invoke<Person>('pull_me', { dir }).catch(() => null)).then((found) => dir === folder && (me = found))
  })

  type Spot = Omit<Note, 'current'> & { path: string }
  const draftKey = (c: Comment) => `draft:${c.path}:${c.removed ? 'old' : 'new'}:${c.line}`
  const spots = $derived.by(() => {
    if (!reviewing) return []
    const order = new Map(files.map((f, i) => [f.path, i]))
    const list: Spot[] = threads
      .filter((th) => th.root.path && th.root.line !== null && order.has(th.root.path))
      .map((th) => ({
        key: `thread:${th.root.id}`,
        path: th.root.path!,
        line: th.root.line!,
        removed: th.root.removed,
        mine: false,
        author: { name: th.root.author, avatar: th.root.avatar },
        others: [...new Map(th.all.filter((r) => r.author !== th.root.author).map((r) => [r.author, { name: r.author, avatar: r.avatar }])).values()],
        text: th.root.body,
      }))
    for (const c of [...review, ...comments.filter(here)])
      if (c.path && order.has(c.path) && !list.some((s) => s.key === draftKey(c)))
        list.push({ key: draftKey(c), path: c.path, line: c.line, removed: c.removed, mine: true, author: { name: '', avatar: '' }, others: [], text: c.note })
    return list.sort((a, b) => order.get(a.path)! - order.get(b.path)! || a.line - b.line || Number(b.mine) - Number(a.mine))
  })
  let current = $state('')
  const place = $derived(spots.findIndex((s) => s.key === current))
  const lit = $derived(spots[place] ?? null)
  const litIn = (path: string) => (lit?.path === path ? { side: lit.removed ? ('old' as const) : ('new' as const), number: lit.line } : null)
  let aim = $state<Spot | null>(null)

  function goToSpot(s: Spot) {
    picked = ''
    current = s.key
    spot = s.path
    steered = Date.now()
    setOpen(s.path, true)
    loadPatch(s.path)
    aim = s
  }

  function step(by: number) {
    if (!spots.length) return
    goToSpot(spots[place < 0 ? (by > 0 ? 0 : spots.length - 1) : (place + by + spots.length) % spots.length])
  }

  $effect(() => {
    if (!aim) return
    const patch = patchOf(aim.path)
    if (!patch || patch === 'loading') return
    const { path, removed, line } = aim
    untrack(() => {
      aim = null
      tick().then(() => {
        steered = Date.now()
        const section = body?.querySelector<HTMLElement>(`[data-file="${CSS.escape(path)}"]`)
        const row = section?.querySelector<HTMLElement>(`[data-line="${removed ? 'old' : 'new'}:${line}"]`)
        within(row ?? section, row ? 'near' : 'start')
      })
    })
  })

  function nextKey(e: KeyboardEvent) {
    if (!reviewing || !spots.length || e.ctrlKey || e.altKey || e.metaKey || e.key.toLowerCase() !== 'n' || !body?.offsetParent) return
    if ((e.target as HTMLElement | null)?.closest?.('input, textarea, select, [contenteditable="true"]')) return
    e.preventDefault()
    step(e.shiftKey ? -1 : 1)
  }

  function open(line: Line) {
    commenting = { ...line, path: selected.path }
    const found = find(selected.path, line)
    note = found ? found.list[found.index].note : ''
  }

  type Patch = { hunks: Hunk[]; status: string; binary: boolean; blob: string }
  let patches = $state<Record<string, Patch | { error: string } | 'loading'>>({})
  const patchOf = (path: string) => patches[`${source?.from}:${source?.now}:${path}`]

  function loadPatch(path: string) {
    if (!source || patchOf(path)) return
    const key = `${source.from}:${source.now}:${path}`
    const status = source.files.find((f) => f.path === path)?.status ?? ''
    patches[key] = 'loading'
    invoke<string>('git_patch', { project: folder, from: source.from, now: source.now, path })
      .then(
        (text) =>
          (patches[key] = {
            hunks: (parsePatch(text)[0]?.hunks ?? []) as Hunk[],
            status: STATUS[status] ?? 'MODIFIED',
            binary: /^Binary files /m.test(text),
            blob: /^index \w+\.\.(\w+)/m.exec(text)?.[1] ?? '',
          }),
      )
      .catch((e) => (patches[key] = { error: String(e) }))
  }

  function approach(el: HTMLElement, path: string) {
    const watch = new IntersectionObserver((seen) => seen.some((s) => s.isIntersecting) && loadPatch(path), {
      root: el.closest('.body'),
      rootMargin: '800px 0px',
    })
    watch.observe(el)
    return { destroy: () => watch.disconnect() }
  }

  let body = $state<HTMLElement>()
  let spot = $state('')
  let flipped = $state<string[]>([])
  const seen = $derived(reviewing && pull ? (viewedFiles[pull.url] ?? {}) : {})
  const viewedCount = $derived(files.filter((f) => viewed(f.path) === 'yes').length)

  function viewed(path: string) {
    if (!(path in seen)) return 'no'
    const patch = patchOf(path)
    if (!patch || patch === 'loading' || 'error' in patch) return 'yes'
    return patch.blob === seen[path] ? 'yes' : 'changed'
  }

  const isOpen = (path: string) => (viewed(path) === 'yes') === flipped.includes(path)
  const setOpen = (path: string, open: boolean) => {
    if (isOpen(path) !== open) flipped = flipped.includes(path) ? flipped.filter((p) => p !== path) : [...flipped, path]
  }

  function toggleViewed(path: string, event: Event) {
    const patch = patchOf(path)
    if (!pull || !patch || patch === 'loading' || 'error' in patch) return
    const on = viewed(path) !== 'yes'
    setViewed(pull, path, on ? patch.blob : null)
    flipped = flipped.filter((p) => p !== path)
    const section = (event.currentTarget as HTMLElement).closest<HTMLElement>('[data-file]')
    if (on && body && section && section.getBoundingClientRect().top < body.getBoundingClientRect().top) tick().then(() => within(section, 'start'))
  }

  $effect(() => {
    if (!reviewing || !source) return
    for (const path of Object.keys(seen)) if (files.some((f) => f.path === path)) untrack(() => loadPatch(path))
  })
  let wholes = $state<string[]>([])
  const toggleWhole = (path: string) => (wholes = wholes.includes(path) ? wholes.filter((p) => p !== path) : [...wholes, path])

  let steered = 0

  function spy() {
    if (!body || !reviewing || Date.now() - steered < 400) return
    const top = body.getBoundingClientRect().top + 60
    let found = ''
    for (const el of body.querySelectorAll<HTMLElement>('[data-file]')) {
      if (el.getBoundingClientRect().top > top) break
      found = el.dataset.file ?? ''
    }
    spot = found || files[0]?.path || ''
  }

  $effect(() => {
    void files
    void reviewing
    tick().then(spy)
  })

  function scrollToFile(path: string) {
    spot = path
    steered = Date.now()
    setOpen(path, true)
    tick().then(() => within(body?.querySelector<HTMLElement>(`[data-file="${CSS.escape(path)}"]`), 'start'))
  }

  function within(el: Element | null | undefined, block: 'start' | 'center' | 'near') {
    const box = el?.closest<HTMLElement>('.body')
    if (!el || !box) return
    const r = el.getBoundingClientRect()
    const b = box.getBoundingClientRect()
    box.scrollTop += r.top - b.top - (block === 'center' ? Math.max(0, (b.height - r.height) / 2) : block === 'near' ? b.height / 4 : 0)
  }

  const notesFor = (path: string) => [...review, ...comments.filter(here)].filter((c) => c.path === path)
  const markedFor = (path: string) => notesFor(path).map((c) => ({ side: c.removed ? ('old' as const) : ('new' as const), number: c.line, note: c.note }))
  function outsideFor(path: string, hunks: Hunk[]) {
    const lines = new Set(rows(hunks).flatMap((r) => (r.kind === 'head' ? [] : r.kind === 'del' ? [`old:${r.old}`] : [`new:${r.new}`])))
    return notesFor(path).filter((c) => !lines.has(`${c.removed ? 'old' : 'new'}:${c.line}`))
  }

  function openAt(path: string, line: Line) {
    commenting = { ...line, path }
    const found = find(path, line)
    note = found ? found.list[found.index].note : ''
  }

  $effect(() => {
    if (!target) return
    if (reviewing) {
      const patch = patchOf(target.path)
      const path = target.path
      if (!patch || patch === 'loading') return untrack(() => (loadPatch(path), scrollToFile(path)))
    } else if (shown?.path !== target.path) return
    commenting = { side: target.removed ? 'old' : 'new', number: target.line, text: target.code, path: target.path }
    note = target.note
    target = null
  })

  const still = matchMedia('(prefers-reduced-motion: reduce)').matches

  function reveal(el: HTMLElement) {
    el.querySelector('textarea')?.focus({ preventScroll: true })
    const box = el.closest('.body')
    if (!box) return
    const top = el.getBoundingClientRect().top
    const view = box.getBoundingClientRect()
    if (top < view.top || top > view.bottom) within(el, 'center')
  }

  function glide(event: Event) {
    const el = event.currentTarget as HTMLElement
    const box = el.closest('.body')
    if (!box) return
    const r = el.getBoundingClientRect()
    const view = box.getBoundingClientRect()
    const by = Math.min(r.bottom + 12 - view.bottom, r.top - view.top - 48)
    if (by > 0) box.scrollBy({ top: by, behavior: still ? 'instant' : 'smooth' })
  }

  function draft(): Comment {
    const { path, side, number, text } = commenting!
    return { path, line: number, removed: side === 'old', code: text, note: note.trim(), ...(reviewing ? { pull: pull!.number, ...inRound(reviewKey, pull!.head) } : {}) }
  }

  function comment(event?: Event) {
    event?.preventDefault()
    if (!commenting || !note.trim()) return
    if (existing) existing.list[existing.index].note = note.trim()
    else if (reviewing) reviews[reviewKey] = [...(reviews[reviewKey] ?? []), draft()]
    else comments.push(draft())
    commenting = null
  }

  function forAgent() {
    if (!commenting || !note.trim()) return
    comments.push(draft())
    commenting = null
  }

  let posting = $state(false)
  let postError = $state<{ text: string; done: number } | null>(null)
  const problem = $derived(
    postError ? (postError.done ? t('review.postedThenError', { count: postError.done, error: errorText(postError.text) }) : errorText(postError.text)) : '',
  )
  let finishing = $state(false)
  let summary = $state('')

  async function postNow() {
    if (!commenting || !note.trim() || !pull) return
    const { path, side, number, text } = commenting
    posting = true
    postError = null
    const { error } = await post(folder, pull, [{ path, line: number, removed: side === 'old', code: text, note: note.trim() }])
    posting = false
    if (error) return (postError = { text: error, done: 0 })
    if (existing) existing.list.splice(existing.index, 1)
    commenting = null
    fresh++
  }

  let generalNote = $state('')
  const mineGeneral = $derived(reviewing ? [...review, ...comments.filter(here)].filter((c) => !c.path) : [])
  const generalDraft = (): Comment => ({ path: '', line: 0, removed: false, code: '', note: generalNote.trim(), pull: pull!.number, ...inRound(reviewKey, pull!.head) })

  function addGeneral(where: 'review' | 'agent') {
    if (!generalNote.trim() || !pull) return
    if (where === 'review') reviews[reviewKey] = [...(reviews[reviewKey] ?? []), generalDraft()]
    else comments.push(generalDraft())
    generalNote = ''
  }

  async function postGeneral() {
    if (!generalNote.trim() || !pull) return
    posting = true
    postError = null
    const { error } = await post(folder, pull, [generalDraft()])
    posting = false
    if (error) return (postError = { text: error, done: 0 })
    generalNote = ''
    fresh++
  }

  let listing = $state(false)
  let addMenu = $state<'line' | 'general' | null>(null)
  const shownLines = $derived(
    shown ? new Set(rows(shown.hunks).flatMap((r) => (r.kind === 'head' ? [] : r.kind === 'del' ? [`old:${r.old}`] : [`new:${r.new}`]))) : null,
  )
  const outside = $derived(
    reviewing && selected && shown && shownLines
      ? [...review, ...comments.filter(here)].filter((c) => c.path === selected.path && !shownLines.has(`${c.removed ? 'old' : 'new'}:${c.line}`))
      : [],
  )

  function goTo(c: Comment) {
    listing = false
    if (!c.path) return within(document.querySelector('.general'), 'start')
    const there = spots.find((s) => s.key === draftKey(c))
    if (there) return goToSpot(there)
    if (reviewing) scrollToFile(c.path)
    else file = c.path
    target = c
  }

  function drop(c: Comment) {
    if (review.includes(c)) reviews[reviewKey] = review.filter((x) => x !== c)
    else comments.splice(comments.indexOf(c), 1)
  }

  let starting = $state(false)

  async function askAgent() {
    starting = true
    postError = null
    await onagent?.().catch((e) => (postError = { text: String(e), done: 0 }))
    starting = false
  }

  let verdict = $state<'' | 'approve' | 'request_changes'>('')
  const VERDICTS: [typeof verdict, Key][] = [
    ['', 'review.verdictComment'],
    ['approve', 'review.verdictApprove'],
    ['request_changes', 'review.verdictRequestChanges'],
  ]
  const closed = $derived(!!pull && pull.state !== 'open')
  const ready = $derived(!!verdict || !!summary.trim() || review.length > 0)

  async function submit() {
    if (!pull || !ready) return
    const sending = [...review]
    const head = openRound(reviewKey)?.head || pull.head
    const resolving = resolvables.filter((th) => toResolve.includes(th.root.id))
    posting = true
    postError = null
    const { done, said, error, sent } = await post(folder, pull, sending, summary, verdict, head)
    posting = false
    reviews[reviewKey] = review.filter((c) => !sent.includes(c))
    if (said) summary = ''
    if (error) return (postError = { text: error, done })
    closeRound(reviewKey, head, verdict, sending.length)
    const failed: string[] = []
    for (const th of resolving)
      await invoke('pull_resolve', { dir: folder, number: pull.number, to: th.root.id, kind: th.root.kind, resolved: true }).catch((e) => failed.push(String(e)))
    if (resolving.length) fresh++
    if (failed.length) postError = { text: t('review.resolveFailed', { count: failed.length, error: errorText(failed[0]) }), done: 0 }
    summary = ''
    verdict = ''
    finishing = false
    if (!agent) markReviewed(folder, pull, head)
    else if (agent.review) markReviewed(agent.review.project, pull, head)
    onposted?.()
  }

  const handing = $derived(reviewing ? [...review, ...comments.filter(here)] : [])

  function handOver() {
    const list = $state.snapshot(handing)
    reviews[reviewKey] = []
    comments = comments.filter((c) => !here(c))
    finishing = false
    onreview(summary.trim(), list)
    summary = ''
  }

  async function discard() {
    const sure = await ask(t('review.discardQuestion', { count: review.length }), {
      title: t('review.discardTitle'),
      kind: 'warning',
      okLabel: t('review.discard'),
      cancelLabel: t('review.keepIt'),
    })
    if (!sure) return
    reviews[reviewKey] = []
    finishing = false
  }

  function remove() {
    if (existing) existing.list.splice(existing.index, 1)
    commenting = null
  }
</script>

<svelte:window onkeydown={nextKey} onpointerdown={(e) => addMenu && !(e.target as HTMLElement).closest('[data-add-menu]') && (addMenu = null)} />

<SidePanel kind="diff" title={t('review.diff')} {back} {onclose}>
  {#snippet sub()}
    {#if reviewing && pull}
      {t('review.pullTag', { number: String(pull.number) })} · {pull.branch} → {pull.base}{source ? ` · ${t('review.fileCount', { count: files.length })}` : ''}{onlyNew && pulled
        ? ` · ${t('review.newCommitsSince', { count: pulled.added })}`
        : ''}
    {:else if branching}
      {branched ? t('review.branchSince', { base: branched.base, commits: branched.commits, count: files.length }) : branchError}
    {:else if changes}
      {t('review.uncommittedFiles', { count: files.length })}{agent ? ` · ${t('review.editedBy', { count: mine, agent: agent.name })}` : ''}
    {:else if agent}
      {t('review.changesBy', { agent: agent.name })} · {t('review.fileCount', { count: files.length })}
    {:else}
      {t('review.noGitRepository')}
    {/if}
  {/snippet}
  {#snippet actions()}
    <button
      class="tree-toggle"
      aria-pressed={layout.fileTree}
      aria-label={t('review.fileTree')}
      title={layout.fileTree ? t('review.hideFileTree') : t('review.showFileTree')}
      onclick={() => setFileTree(!layout.fileTree)}
    >
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 5h6M4 12h6M4 19h6M14 5h6M16 12h4M16 19h4M14 5v14" /></svg>
    </button>
    {#if reviewing}
      <div class="modes">
        <button aria-pressed={layout.threads} title={layout.threads ? t('review.hideThreads') : t('review.showThreads')} onclick={() => setThreads(!layout.threads)}>{t('review.viewThreads')}{#if pending.length}<span class="talks">{pending.length}</span>{/if}</button>
      </div>
    {/if}
    {#if reviewing && since}
      <div class="modes" role="group" aria-label={t('review.whatToReview')}>
        <button aria-pressed={onlyNew} onclick={() => ((onlyNew = true), (sinceError = ''))}>{t('review.sinceYourReview')}</button>
        <button aria-pressed={!onlyNew} onclick={() => (onlyNew = false)}>{t('review.allChanges')}</button>
      </div>
    {/if}
    {#if agent && (changes || pull)}
      <div class="modes" role="group" aria-label={t('review.whatToCompare')}>
        <button aria-pressed={mode === 'local'} onclick={() => (mode = 'local')}>{t('review.uncommitted')}</button>
        {#if changes}<button aria-pressed={mode === 'branch'} title={t('review.branchTitle')} onclick={() => (mode = 'branch')}>{t('review.branch')}</button>{/if}
        {#if pull}<button aria-pressed={mode === 'pull'} onclick={() => (mode = 'pull')}>{t('review.pullTag', { number: String(pull.number) })}</button>{/if}
      </div>
    {/if}
    <div class="modes" role="group" aria-label={t('review.diffLayout')}>
      <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>{t('review.unified')}</button>
      <button aria-pressed={layout.split} onclick={() => setSplit(true)}>{t('review.split')}</button>
    </div>
  {/snippet}
  {@render banner?.()}
  {#if reviewing && pull}
    <div class="pr-head">
      <strong class="pr-title">{pull.title}</strong>
      {#if pull.description.trim()}
        <details class="pr-description">
          <summary>{t('common.description')}</summary>
          <div class="pr-body"><Markdown text={pull.description} /></div>
        </details>
      {/if}
    </div>
  {/if}
  {#if reviewing && pull}
    <div class="review-bar" class:idle={!review.length}>
      <span class="pulse" aria-hidden="true"></span>
      <span class="what">
        {#if review.length}
          <strong>{openRound(reviewKey) ? t('review.roundN', { n: openRound(reviewKey)!.n }) : t('review.reviewInProgress')}</strong> ·
          <button class="count-toggle" aria-expanded={listing} onclick={() => (listing = !listing)}
            >{t('review.commentCount', { count: review.length })} <span aria-hidden="true">{listing ? '▴' : '▾'}</span></button
          >
        {:else}<strong>{t('review.yourReview')}</strong> · {t('review.commentThenFinish')}{/if}
      </span>
      {#if spots.length}
        <span class="nav" role="group" aria-label={t('review.commentNav')}>
          <button aria-label={t('review.previousComment')} title={t('review.previousComment')} onclick={() => step(-1)}>‹</button>
          <span class="place">{place < 0 ? t('review.commentCount', { count: spots.length }) : t('review.commentPlace', { index: place + 1, total: spots.length })}</span>
          <button aria-label={t('review.nextComment')} title={t('review.nextComment')} onclick={() => step(1)}>›</button>
          <kbd>N</kbd>
        </span>
      {/if}
      {#if postError && !commenting}<span class="bad" role="alert">{problem}</span>{/if}
      {#if reviewer}
        <button class="agent-link" title={t('review.openConversationOf', { name: reviewer.name })} onclick={() => show(reviewer)}>
          {t(
            reviewer.status === 'working'
              ? 'review.reviewerWorking'
              : reviewer.status === 'waiting'
                ? 'review.reviewerWaiting'
                : reviewer.status === 'error'
                  ? 'review.reviewerError'
                  : 'review.reviewerDone',
            { name: reviewer.name },
          )}
        </button>
      {:else if onagent}
        <button
          class="btn btn-secondary"
          disabled={starting || !settings.pilot}
          title={settings.pilot
            ? t('review.askAgentTitle')
            : t('review.askAgentNeedsPilot')}
          onclick={askAgent}>{starting ? t('review.starting') : t('review.askAgent')}</button
        >
      {/if}
      {#if rereadable}
        <button class="btn btn-secondary rereview" disabled={rereading} title={t('review.reviewNewChangesTitle', { name: rereader!.name })} onclick={reReview}
          ><Icon name="persona" size={14} />{rereading ? t('review.starting') : t('review.reReviewWithAgent')}</button
        >
      {/if}
      {#if review.length}<button class="btn btn-secondary" onclick={discard}>{t('review.discard')}</button>{/if}
      <button class="btn btn-primary" aria-expanded={finishing} onclick={() => (finishing = !finishing)}>{t('review.finishReview')}</button>
    </div>
    {#if listing && review.length}
      <ul class="drafts-list">
        {#each review as c, i (i)}
          <li>
            <button class="go" onclick={() => goTo(c)}>
              <span class="where">{c.path ? `${c.path}:${c.line}${c.removed ? ` · ${t('review.removedLine')}` : ''}` : t('review.pullRequest')}</span>
              <span class="note">{c.note}</span>
            </button>
            <button class="drop" aria-label={t('review.deleteCommentOnLineOf', { path: c.path, line: String(c.line) })} title={t('common.delete')} onclick={() => drop(c)}>×</button>
          </li>
        {/each}
      </ul>
    {/if}
    {#if finishing}
      <div class="finish">
        {#if resolvables.length}
          <div class="resolve-list" role="group" aria-label={t('review.resolveOn', { host: hostName(pull) })}>
            <span class="resolve-head">{t('review.resolveOn', { host: hostName(pull) })} <span>{t('review.resolveHelp')}</span></span>
            {#each resolvables as th (th.root.id)}
              {@const state = stateOf(reviewKey, th)}
              <label>
                <input
                  type="checkbox"
                  checked={toResolve.includes(th.root.id)}
                  onchange={(e) => (toResolve = e.currentTarget.checked ? [...toResolve, th.root.id] : toResolve.filter((id) => id !== th.root.id))}
                />
                <span class="state {state}">{t(STATE_KEYS[state])}</span>
                <span class="where">{th.root.path ? `${th.root.path}:${th.root.line}` : t('review.pullRequest')}</span>
                <span class="why">{judgementOf(reviewKey, th.root.id)?.note ?? th.root.body.split('\n')[0]}</span>
              </label>
            {/each}
          </div>
        {/if}
        <label for="review-summary" class="sr-only">{t('review.overallComment')}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea
          id="review-summary"
          class="field"
          rows="3"
          bind:value={summary}
          placeholder={verdict === 'request_changes' ? t('review.whatShouldChange') : t('review.overallCommentOptional')}
          autofocus
        ></textarea>
        <div class="actions">
          {#if (agent && !agent.review) || (own && handTo)}
            {@const to = agent ?? (handTo === 'new' ? null : handTo)}
            {@const where = to ? t('review.theConversation') : t('review.newAgent')}
            <span class="hint">{t('review.sendOrPost', { count: handing.length, agent: where, host: hostName(pull) })}</span>
            {#if can(pull.host, 'post')}
              <button class="btn btn-secondary" disabled={posting || !ready} onclick={submit}>{posting ? t('review.posting') : t('review.postOn', { host: hostName(pull) })}</button>
            {/if}
            <button class="btn btn-primary" title={to?.name} disabled={posting || !(ready || handing.length)} onclick={handOver}>{t('review.sendTo', { agent: where })}</button>
          {:else if can(pull.host, 'post')}
            <div class="modes verdicts" role="radiogroup" aria-label={t('review.verdict')}>
              {#each (own || closed ? VERDICTS.slice(0, 1) : VERDICTS).filter(([id]) => offers(pull.host, id || 'comment')) as [id, label] (id)}
                <button role="radio" aria-checked={verdict === id} onclick={() => (verdict = id)}>{t(label)}</button>
              {/each}
            </div>
            <span class="hint">
              {#if verdict === 'request_changes' && !summary.trim()}
                {pull.host === 'github' ? t('review.emptyGetsDefault', { host: hostName(pull) }) : opened ? t('review.threadsSayWhat', { count: opened }) : ''}
              {/if}
            </span>
            <button class="btn btn-primary" disabled={posting || !ready} onclick={submit}>{posting ? t('review.posting') : t('review.submitOn', { host: hostName(pull) })}</button>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
  {#if reviewing && sinceError}
    <p class="ahead" role="note">{errorText(sinceError)}</p>
  {/if}
  {#if reviewing && agent && pulled && pulled.ahead > 0 && source}
    <p class="ahead" role="note">
      {t('review.notPushed', { count: pulled.ahead, host: hostName(pull!) })}
    </p>
  {/if}
  {#if reviewing && !source}
    <div class="fetching" role="status">
      {#if pullError}
        <svg class="failed" width="34" height="34" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" /><path d="M12 7v6M12 16.5v.5" /></svg>
        <strong>{t('review.couldNotRead', { number: String(pull!.number) })}</strong>
        <span class="why">{errorText(pullError)}</span>
        <button class="btn btn-secondary" onclick={() => attempt++}>{t('common.tryAgain')}</button>
      {:else}
        <svg class="ring" width="34" height="34" viewBox="0 0 34 34" aria-hidden="true"><circle class="track" cx="17" cy="17" r="14" /><circle class="arc" cx="17" cy="17" r="14" /></svg>
        <strong>{t('review.fetching', { number: String(pull!.number) })}</strong>
        <span class="branches">{pull!.branch} → {pull!.base}</span>
        <span class="why">{t('review.fetchingWhy', { host: hostName(pull!) })}</span>
      {/if}
    </div>
  {:else if files.length}
    {#if !layout.fileTree}
    <div class="tabs" role="tablist" aria-label={t('review.changedFiles')}>
      {#each files as f (f.path)}
        <button
          role="tab"
          aria-selected={reviewing ? f.path === spot : f === selected}
          class:other={!f.edits && !reviewing && !!agent}
          class:viewed={reviewing && viewed(f.path) === 'yes'}
          title={f.edits || !agent ? f.path : t('review.notEditedByShort', { path: f.path, agent: agent.name })}
          onclick={() => (reviewing ? scrollToFile(f.path) : (file = f.path))}
        >
          {name(f.path)} <span class="add">+{f.add}</span>{#if f.del}<span class="del">−{f.del}</span>{/if}
          {#if reviewing && talks(f.path)}<span class="talks" title={t('review.reviewThreads', { count: talks(f.path) })}>{talks(f.path)}</span>{/if}
        </button>
      {/each}
    </div>

    {/if}

    <div class="columns">
    {#if layout.fileTree}
      <ChangedFiles
        files={files.map((f) => ({
          ...f,
          other: !f.edits && !reviewing && !!agent,
          notes: spots.filter((s) => s.path === f.path).map((s) => ({ ...s, current: s.key === current })),
          viewed: reviewing && viewed(f.path) === 'yes',
        }))}
        viewed={reviewing ? viewedCount : null}
        selected={reviewing ? spot : selected?.path}
        {me}
        onpick={(path) => (reviewing ? scrollToFile(path) : (file = path))}
        onnote={(key) => {
          const there = spots.find((s) => s.key === key)
          if (there) goToSpot(there)
        }}
      />
    {/if}
    <div class="body" class:stack={reviewing} bind:this={body} onscroll={spy}>
      {#if reviewing && pull}
        {@const count = general.length + mineGeneral.length}
        <details class="general" open={count > 0 || undefined}>
          <summary>{count ? t('review.commentsOnPull', { count }) : t('review.commentOnWholePull')}</summary>
          {#each general as th (th.root.id)}{@render thread(th)}{/each}
          {#each mineGeneral as c, i (i)}
            <div class="thread mine">
              <span class="who"><strong>{t('common.you')}</strong> · {review.includes(c) ? t('review.inReviewNotPosted') : t('review.inNextMessageTo', { agent: agent?.name ?? '' })}</span>
              <Markdown text={c.note} />
              <div class="thread-actions"><button class="drop" aria-label={t('review.deleteThisComment')} title={t('common.delete')} onclick={() => drop(c)}>×</button></div>
            </div>
          {/each}
          <form class="pr-comment" onsubmit={(e) => (e.preventDefault(), addGeneral('review'))}>
            <label for="pr-comment" class="sr-only">{t('review.commentOnWholePull')}</label>
            <textarea
              id="pr-comment"
              class="field"
              rows="2"
              bind:value={generalNote}
              placeholder={t('review.commentOnWholePull')}
              onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && (e.preventDefault(), addGeneral('review'))}
            ></textarea>
            <div class="actions">
              <span class="hint"><Trans k="review.ctrlEnterAdds">{#snippet keys()}<kbd>Ctrl ↵</kbd>{/snippet}</Trans></span>
              {@render addTo('general', pull, !generalNote.trim(), postGeneral, () => addGeneral('agent'))}
            </div>
          </form>
        </details>
      {/if}
      {#if reviewing}
        {#each files as f (f.path)}
          {@const patch = patchOf(f.path)}
          {@const open = isOpen(f.path)}
          {@const seenAs = viewed(f.path)}
          <section class="file" data-file={f.path} use:approach={f.path}>
            <div class="file-head">
              <button
                class="fold-file"
                aria-expanded={open}
                aria-label={open ? t('review.foldFile', { path: f.path }) : t('review.unfoldFile', { path: f.path })}
                onclick={() => setOpen(f.path, !open)}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
              </button>
              {#if patch && typeof patch === 'object' && 'status' in patch}
                <span class="badge" class:new={patch.status === 'NEW'} class:gone={patch.status === 'DELETED'}>{badge(patch.status)}</span>
              {/if}
              <span class="path">{f.path}</span>
              <span class="stat"><span class="add">+{f.add}</span>{#if f.del}&nbsp;<span class="del">−{f.del}</span>{/if}</span>
              {#if open && patch && typeof patch === 'object' && 'status' in patch && patch.status === 'MODIFIED' && !patch.binary && patch.hunks.length}
                <button class="whole" aria-pressed={wholes.includes(f.path)} onclick={() => toggleWhole(f.path)}>{t('review.wholeFile')}</button>
              {/if}
              <button
                class="mark-viewed"
                class:changed={seenAs === 'changed'}
                role="checkbox"
                aria-checked={seenAs === 'yes'}
                disabled={!patch || patch === 'loading' || 'error' in patch}
                title={seenAs === 'changed' ? t('review.changedSinceViewed') : undefined}
                onclick={(e) => toggleViewed(f.path, e)}
              >
                <span class="box" aria-hidden="true"><svg width="10" height="10" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg></span>
                {seenAs === 'changed' ? t('review.changedViewed') : t('review.viewed')}
              </button>
            </div>
            {#if open}
              {#if !patch || patch === 'loading'}
                <p class="empty">{t('review.readingFile')}</p>
              {:else if 'error' in patch}
                <p class="empty">{errorText(patch.error)}</p>
              {:else}
                {@const away = outsideFor(f.path, patch.hunks)}
                {#if away.length}
                  <div class="outside">
                    <span class="outside-head">{t('review.commentsOutside')}</span>
                    {#each away as c, i (i)}
                      <div class="outside-item">
                        <span class="mono">{t(c.removed ? 'review.removedLineNumber' : 'review.lineNumber', { line: String(c.line) })}</span>
                        <div class="outside-note"><Markdown text={c.note} /></div>
                        <button class="drop" aria-label={t('review.deleteCommentOnLine', { line: String(c.line) })} title={t('common.delete')} onclick={() => drop(c)}>×</button>
                      </div>
                    {/each}
                  </div>
                {/if}
                {#if patch.binary}
                  <p class="empty">{t('review.binaryFile')}</p>
                {:else if patch.hunks.length}
                  {#snippet lineThreads(line: Line)}
                    {@render mineAt(f.path, line)}
                    {#each threadsAt(f.path, line) as th (th.root.id)}{@render thread(th, line)}{/each}
                  {/snippet}
                  <DiffView
                    hunks={patch.hunks}
                    split={layout.split}
                    content={() => invoke<string | null>('git_file', { project: folder, rev: source!.now, path: f.path })}
                    whole={wholes.includes(f.path)}
                    oncomment={(line) => openAt(f.path, line)}
                    commentRemoved={!onlyNew}
                    commentRevealed={false}
                    marked={markedFor(f.path)}
                    lit={litIn(f.path)}
                    open={commenting?.path === f.path ? commenting : null}
                    {editor}
                    below={lineThreads}
                  />
                {:else}
                  <p class="empty">{t('review.noChange')}</p>
                {/if}
              {/if}
            {/if}
          </section>
        {/each}
      {:else if selected}
        <div class="file">
          <div class="file-head">
            {#if shown}
              <span class="badge" class:new={shown.status === 'NEW'} class:gone={shown.status === 'DELETED'} class:proposed={proposing}>{badge(shown.status)}</span>
            {/if}
            <span class="path">{selected.path}</span>
            <span class="by" hidden={reviewing || !agent}
              >{proposing
                ? t('review.proposedBy', { agent: agent?.name ?? '' })
                : selected.edits
                  ? t('review.editsBy', { count: selected.edits, agent: agent?.name ?? '' })
                  : t('review.notEditedBy', { agent: agent?.name ?? '' })}</span
            >
            {#if shown && !proposing && shown.status === 'MODIFIED' && !shown.binary && shown.hunks.length}
              <button class="whole" aria-pressed={wholes.includes(selected.path)} onclick={() => toggleWhole(selected.path)}>{t('review.wholeFile')}</button>
            {/if}
          </div>
          {#if outside.length}
            <div class="outside">
              <span class="outside-head">{t('review.commentsOutside')}</span>
              {#each outside as c, i (i)}
                <div class="outside-item">
                  <span class="mono">{t(c.removed ? 'review.removedLineNumber' : 'review.lineNumber', { line: String(c.line) })}</span>
                  <div class="outside-note"><Markdown text={c.note} /></div>
                  <button class="drop" aria-label={t('review.deleteCommentOnLine', { line: String(c.line) })} title={t('common.delete')} onclick={() => drop(c)}>×</button>
                </div>
              {/each}
            </div>
          {/if}
          {#if error}
            <p class="empty">{errorText(error)}</p>
          {:else if !shown}
            <p class="empty">{t('review.readingFile')}</p>
          {:else if shown.binary}
            <p class="empty">{t('review.binaryFile')}</p>
          {:else if shown.hunks.length}
            <DiffView
              hunks={shown.hunks}
              split={layout.split}
              content={proposing
                ? undefined
                : source
                  ? () => invoke<string | null>('git_file', { project: folder, rev: source.now, path: selected.path })
                  : () => invoke<string | null>('read_project_file', { project: folder, path: selected.path })}
              whole={!proposing && wholes.includes(selected.path)}
              oncomment={proposing || (!agent && !reviewing) ? undefined : open}
              {marked}
              open={commenting?.path === selected.path ? commenting : null}
              {editor}
              {below}
            />
          {:else}
            <p class="empty">{t('review.backToOriginal')}</p>
          {/if}
        </div>
      {/if}
    </div>
    {#if reviewing && pull && layout.threads}
      <aside class="threads-column" style:width="min({layout.threadsWidth}px, 40%)" aria-label={t('review.viewThreads')}>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
        <div
          class="threads-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label={t('review.resizeThreads')}
          aria-valuenow={layout.threadsWidth}
          aria-valuemin={280}
          aria-valuemax={640}
          tabindex="0"
          title={t('review.dragToResizeThreads')}
          onpointerdown={dragThreads}
          onkeydown={(e) => (e.key === 'ArrowLeft' ? setThreadsWidth(layout.threadsWidth + 20) : e.key === 'ArrowRight' && setThreadsWidth(layout.threadsWidth - 20))}
          ondblclick={() => setThreadsWidth(320)}
        ></div>
        <div class="threads-head">
          <strong>{t('review.viewThreads')}</strong>
          {#if pending.length}<span class="talks">{pending.length}</span>{/if}
          <button class="fold-threads" aria-label={t('review.hideThreads')} title={t('review.hideThreads')} onclick={() => setThreads(false)}>
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        </div>
        <PullThreads {pull} {folder} {reviewKey} {threads} self={me?.name ?? ''} drafts={review} onfresh={() => fresh++} ongo={goToThread} ondraft={goToDraft} ondrop={drop} onedit={(c, text) => (c.note = text)} />
      </aside>
    {/if}
    </div>

    <div class="foot">
      <span>
        {reviewing && pull
          ? onlyNew
            ? t('review.footSince')
            : t('review.footPull', { base: pull.base })
          : branching && branched
            ? t('review.footBranch', { base: branched.base })
            : changes
            ? t('review.footUncommitted')
            : t('review.footAgent')}
      </span>
      {#if reviewing && pull}
        <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: pull.url })}>{t('review.openOn', { host: hostName(pull) })}</button>
      {:else}
        <OpenWith project={folder} path={selected.path} up />
      {/if}
    </div>
  {:else}
    <p class="empty pad">{changes ? t('review.nothingChanged') : agent ? t('review.agentChangedNothing', { agent: agent.name }) : t('review.noRepositoryHere')}</p>
  {/if}
</SidePanel>

{#snippet below(line: Line)}
  {#if selected}
    {@render mineAt(selected.path, line)}
    {#if reviewing}
      {#each threadsAt(selected.path, line) as th (th.root.id)}{@render thread(th, line)}{/each}
    {/if}
  {/if}
{/snippet}

{#snippet mineAt(path: string, line: Line)}
  {#if !(commenting?.path === path && commenting.side === line.side && commenting.number === line.number)}
    {#each notesFor(path).filter(at(path, line)) as c, i (i)}
      <div class="thread inline-thread mine" class:hit={lit?.key === draftKey(c)}>
        <div class="remark">
          {#if me}
            <span class="self"><Avatar name={me.name} src={me.avatar} size={20} /></span>
          {:else}
            <span class="me" aria-hidden="true"><svg width="11" height="11" viewBox="0 0 24 24"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg></span>
          {/if}
          <div class="said">
            <span class="who"><strong>{t('common.you')}</strong> · {review.includes(c) ? t('review.inReviewNotPosted') : t('review.inNextMessageTo', { agent: agent?.name ?? '' })}</span>
            <Markdown text={c.note} />
          </div>
        </div>
        <div class="thread-actions">
          <button class="btn btn-secondary" onclick={() => openAt(path, line)}>{t('common.edit')}</button>
          <button class="btn btn-secondary" onclick={() => drop(c)}>{t('common.delete')}</button>
        </div>
      </div>
    {/each}
  {/if}
{/snippet}

{#snippet thread(th: Thread, line?: Line)}
  <div class="thread" class:inline-thread={!!line} class:hit={lit?.key === `thread:${th.root.id}` || picked === th.root.id} data-thread={th.root.id}>
    {#if !line && th.root.path}<span class="where">{t('review.outdated', { path: th.root.path })}</span>{/if}
    {#each th.all as r (r.id)}
      <div class="remark" class:reply={r !== th.root}>
        <Avatar name={r.author} src={r.avatar} size={20} />
        <div class="said">
          <span class="who"><strong>{r.author}</strong> · {when(r.at)}</span>
          <Markdown text={r.body} />
        </div>
      </div>
    {/each}
    {#if replying === th.root.id}
      <form class="reply-form" onsubmit={(e) => reply(th, e)} use:reveal in:slide={{ duration: still ? 0 : 260 }} onintroend={glide}>
        <label for="reply-{th.root.id}" class="sr-only">{t('review.replyTo', { author: th.root.author })}</label>
        <textarea
          id="reply-{th.root.id}"
          class="field"
          rows="2"
          bind:value={answer}
          placeholder={t('review.replyTo', { author: th.root.author })}
          onkeydown={(e) => {
            if (e.key === 'Escape') (e.stopPropagation(), (replying = null))
            else if (e.ctrlKey && e.key === 'Enter') reply(th, e)
          }}
        ></textarea>
        <div class="thread-actions">
          {#if replyError}<span class="bad" role="alert">{errorText(replyError)}</span>{/if}
          <button type="button" class="btn btn-secondary" onclick={() => (replying = null)}>{t('common.cancel')}</button>
          <button class="btn btn-primary" disabled={!answer.trim() || answering}>{answering ? t('review.replying') : t('review.replyOn', { host: hostName(pull!) })}</button>
        </div>
      </form>
    {:else}
      <div class="thread-actions">
        {#if pull && can(pull.host, 'reply')}
          <button class="btn btn-secondary" onclick={() => ((replying = th.root.id), (answer = ''), (replyError = ''))}>{t('review.reply')}</button>
        {/if}
        {#if agent}
          {#if handed.includes(th.root.id)}
            <span class="handed">{t('review.inYourNextMessageTo', { agent: agent.name })}</span>
          {:else}
            <button class="btn btn-secondary" onclick={() => hand(th, line)}>{t('review.handTo', { agent: agent.name })}</button>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
{/snippet}

{#snippet addTo(where: 'line' | 'general', pull: Pull, empty: boolean, postIt: () => unknown, toAgent: () => unknown)}
  {@const posts = can(pull.host, 'post')}
  <span class="add" data-add-menu>
    <button class="btn btn-primary" class:joined={posts || !!agent} disabled={empty}>{t('review.addToReview')}</button>
    {#if posts || agent}
      <button
        type="button"
        class="btn btn-primary more"
        aria-label={t('review.moreActions')}
        title={t('review.moreActions')}
        aria-haspopup="menu"
        aria-expanded={addMenu === where}
        onclick={() => (addMenu = addMenu === where ? null : where)}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
    {/if}
    {#if addMenu === where}
      <div class="choices" role="menu" aria-label={t('review.moreActions')}>
        {#if posts}
          <button type="button" role="menuitem" title={t('review.postNowTitle', { host: hostName(pull) })} disabled={empty || posting} onclick={() => ((addMenu = null), postIt())}>
            {posting ? t('review.posting') : t('review.postNowOn', { host: hostName(pull) })}
          </button>
        {/if}
        {#if agent}
          <button type="button" role="menuitem" title={t('review.addToNextMessageTitle', { agent: agent.name })} disabled={empty} onclick={() => ((addMenu = null), toAgent())}>{t('review.addComment')}</button>
        {/if}
      </div>
    {/if}
  </span>
{/snippet}

{#snippet editor()}
  <form class="inline" onsubmit={comment} use:reveal in:slide={{ duration: still ? 0 : 260 }} onintroend={glide}>
    <label for="line-comment" class="sr-only">{t('review.commentOnLine', { line: String(commenting?.number ?? '') })}</label>
    <textarea
      id="line-comment"
      class="field"
      rows="3"
      bind:value={note}
      placeholder={agent ? t('review.whatShouldAgentChange', { agent: agent.name }) : t('review.yourComment')}
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation()
          commenting = null
        } else if (e.ctrlKey && e.key === 'Enter') comment(e)
      }}
    ></textarea>
    <div class="actions">
      {#if existing}
        <button type="button" class="btn btn-danger" onclick={remove}>{t('common.delete')}</button>
        <span class="hint">{existing.inReview ? t('review.inTheReview') : t('review.inYourNextMessage')} · <kbd>Ctrl ↵</kbd></span>
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>{t('common.cancel')}</button>
        <button class="btn btn-primary" disabled={!note.trim()}>{t('common.save')}</button>
      {:else if reviewing && pull}
        {#if postError}<span class="hint bad" role="alert">{problem}</span>{:else}<span class="hint"><Trans k="review.ctrlEnterAdds">{#snippet keys()}<kbd>Ctrl ↵</kbd>{/snippet}</Trans></span>{/if}
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>{t('common.cancel')}</button>
        {@render addTo('line', pull, !note.trim(), postNow, forAgent)}
      {:else}
        <span class="hint">{t('review.addedToNextMessage')} · <kbd>Ctrl ↵</kbd></span>
        <button type="button" class="btn btn-secondary" onclick={() => (commenting = null)}>{t('common.cancel')}</button>
        <button class="btn btn-primary" disabled={!note.trim()}>{t('review.addComment')}</button>
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

  .remark {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 0 8px;
  }

  .remark :global(.avatar) {
    margin-top: 1px;
  }

  .remark.reply {
    margin-left: 28px;
    padding-left: 14px;
    border-left: 2px solid var(--border-strong);
  }

  .me {
    width: 20px;
    height: 20px;
    margin-top: 1px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
  }

  .me svg {
    fill: currentColor;
  }

  .self {
    display: inline-flex;
    margin-top: 1px;
  }

  .self :global(.avatar) {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .inline-thread.mine {
    box-shadow: inset 3px 0 0 var(--accent), inset 0 0 0 1px var(--border-strong);
  }

  .inline-thread.hit {
    box-shadow: inset 3px 0 0 var(--accent), inset 0 0 0 1px var(--border-strong), 0 0 0 2px var(--accent-soft);
  }

  .thread.hit:not(.inline-thread) {
    box-shadow: inset 0 0 0 2px var(--accent-soft);
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
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .reply-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reply-form textarea {
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }

  .thread-actions .btn {
    height: 30px;
    padding: 0 12px;
  }

  .handed {
    font-size: 12px;
    color: var(--ok-text);
  }

  .pr-head {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
  }

  .pr-title {
    font-size: 15px;
    line-height: 1.4;
    color: var(--text-strong);
  }

  .pr-description > summary {
    width: fit-content;
    cursor: pointer;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .pr-description > summary:hover {
    color: var(--text-2);
  }

  .pr-body {
    max-height: 40vh;
    overflow-y: auto;
    margin-top: 8px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 13.5px;
  }

  .review-bar {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--wait-bg);
    font-size: 13px;
  }

  .count-toggle {
    color: var(--accent-soft);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .count-toggle:hover {
    color: var(--text-strong);
  }

  .pr-comment {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
  }

  .pr-comment textarea {
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }

  .pr-comment .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .pr-comment .btn {
    height: 32px;
    padding: 0 12px;
  }

  .add {
    position: relative;
    display: inline-flex;
  }

  .add .joined {
    border-radius: 4px 0 0 4px;
  }

  .add .btn.more {
    width: 30px;
    margin-left: 1px;
    padding: 0;
    border-radius: 0 4px 4px 0;
  }

  .add .more svg {
    width: 12px;
    height: 12px;
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .add .choices {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 30;
    min-width: 220px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .add .choices button {
    height: auto;
    padding: 8px 10px;
    color: var(--text);
    font-size: 13px;
    border-radius: 4px;
    text-align: left;
    white-space: nowrap;
  }

  .add .choices button:hover:not(:disabled) {
    background: var(--menu-hover);
  }

  .thread.mine {
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .drafts-list {
    flex: none;
    max-height: 30vh;
    overflow-y: auto;
    margin: 0;
    padding: 6px 10px;
    list-style: none;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
  }

  .drafts-list li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .drafts-list .go {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 8px;
    border-radius: 4px;
    text-align: left;
  }

  .drafts-list .go:hover {
    background: var(--surface-3);
  }

  .drafts-list .where {
    flex: none;
    font: 12px var(--mono);
    color: var(--accent-soft);
  }

  .drafts-list .note {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    color: var(--text-2);
  }

  .drop {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 16px;
  }

  .drop:hover {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .outside {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--wait-bg);
  }

  .outside-head {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-soft);
  }

  .outside-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-size: 13px;
  }

  .outside-item .mono {
    flex: none;
    padding-top: 2px;
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .outside-note {
    flex: 1;
    min-width: 0;
  }

  .agent-link {
    height: 30px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .agent-link:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .review-bar.idle {
    background: var(--surface-1);
  }

  .review-bar.idle .pulse {
    background: var(--text-faint);
    box-shadow: none;
  }

  .review-bar.idle .what {
    color: var(--text-dim);
  }

  .verdicts button[aria-checked='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .review-bar .what {
    flex: 1 1 220px;
    color: var(--accent-soft);
  }

  .review-bar .what strong {
    color: var(--text-strong);
  }

  .review-bar .btn {
    height: 30px;
    padding: 0 12px;
  }

  .nav {
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 30px;
    padding: 0 4px;
    border-radius: 4px;
    background: var(--wait-btn);
    color: var(--wait-btn-text);
    font-size: 12.5px;
  }

  .nav button {
    width: 24px;
    height: 24px;
    border-radius: 3px;
    font-size: 15px;
  }

  .nav button:hover {
    background: var(--option-on);
    color: var(--text-strong);
  }

  .place {
    padding: 0 4px;
    font-variant-numeric: tabular-nums;
  }

  .nav kbd {
    margin-left: 4px;
    padding: 0 5px;
    border-radius: 3px;
    background: var(--option);
    font: 11px var(--mono);
    color: var(--wait-muted);
  }

  .review-bar.idle .nav {
    background: var(--surface-3);
    color: var(--text-2);
  }

  .review-bar.idle .nav button:hover {
    background: var(--surface-5);
  }

  .review-bar.idle .nav kbd {
    background: var(--surface-4);
    color: var(--text-dim);
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

  .threads-column {
    position: relative;
    flex: none;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .threads-resizer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    z-index: 2;
    width: 6px;
    cursor: col-resize;
  }

  .threads-resizer:hover,
  .threads-resizer:focus-visible,
  .threads-resizer:active {
    background: var(--border-strong);
    outline: none;
  }

  .threads-head {
    flex: none;
    height: 40px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 14px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-strong);
  }

  .fold-threads {
    width: 28px;
    height: 28px;
    margin-left: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .fold-threads:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .fold-threads svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .body.stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .stack .file {
    overflow: visible;
  }

  .stack .file-head {
    position: sticky;
    top: -14px;
    z-index: 3;
    border-radius: 6px 6px 0 0;
    box-shadow: 0 1px 0 var(--border);
  }

  .fold-file {
    flex: none;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .fold-file:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .fold-file svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    transition: transform 0.12s;
  }

  .fold-file[aria-expanded='true'] svg {
    transform: rotate(90deg);
  }

  .stat {
    flex: none;
    font: 12px var(--mono);
  }

  .whole {
    flex: none;
    height: 22px;
    padding: 0 8px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .whole:hover {
    color: var(--text-strong);
  }

  .whole[aria-pressed='true'] {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .mark-viewed {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px 0 6px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .mark-viewed:hover:not(:disabled) {
    color: var(--text-strong);
  }

  .mark-viewed:disabled {
    opacity: 0.5;
  }

  .mark-viewed .box {
    width: 12px;
    height: 12px;
    display: grid;
    place-items: center;
    border-radius: 2px;
    box-shadow: inset 0 0 0 1px var(--text-faint);
  }

  .mark-viewed svg {
    fill: none;
    stroke: var(--accent-ink);
    stroke-width: 3.5;
    visibility: hidden;
  }

  .mark-viewed[aria-checked='true'] {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .mark-viewed[aria-checked='true'] .box {
    background: var(--accent);
    box-shadow: none;
  }

  .mark-viewed[aria-checked='true'] svg {
    visibility: visible;
  }

  .mark-viewed.changed {
    color: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--wait-border);
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

  .tabs button.viewed {
    color: var(--text-faint);
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

  .resolve-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .resolve-head {
    margin-bottom: 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .resolve-head span {
    margin-left: 6px;
    font-size: 12px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-faint);
  }

  .resolve-list label {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .resolve-list input {
    margin: 0;
    accent-color: var(--accent);
  }

  .resolve-list .state {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--err-bg);
    font: 600 11px var(--sans);
    color: var(--err-text);
  }

  .resolve-list .state.answered {
    background: color-mix(in srgb, var(--remote-dot) 16%, transparent);
    color: var(--remote-text);
  }

  .resolve-list .state.fixed {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .resolve-list .where {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 12px var(--mono);
    color: var(--text-3);
  }

  .resolve-list .why {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
    font-size: 12px;
    color: var(--text-dim);
  }

  .rereview {
    gap: 7px;
  }

  .rereview :global(svg) {
    color: var(--persona);
  }
</style>
