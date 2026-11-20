import { invoke } from '@tauri-apps/api/core'
import { agents, create, FOR_INFO, persist, send } from './agents.svelte'
import { defaultsFor, projects } from './projects.svelte'
import { layout, requestPanel, setPanel } from './layout.svelte'
import { settings } from './settings.svelte'
import { prompt, workdir, type Agent, type Comment } from './stream'
import { pushedTo } from './pushed'
import { byHead } from './rounds'
import type { Linked } from './linked.svelte'
import { t, type Key } from './i18n/index.svelte'
import { forgeName, providedLinks } from './forges.svelte'

export type Pull = {
  host: string
  number: number
  title: string
  description: string
  url: string
  state: 'open' | 'merged' | 'declined' | 'closed' | 'superseded'
  draft: boolean
  approvals: number
  changes_requested: boolean
  comments: number
  checks: 'passing' | 'failing' | 'pending' | null
  branch: string
  base: string
  head: string
  author: string
  author_avatar: string
  updated: string
  reviewed: 'approved' | 'changes_requested' | 'commented' | null
  reviewed_head: string | null
  reviewed_at: string | null
  head_at: string | null
  reviewers: Reviewer[]
  conflicts: boolean
  mergeable: boolean
}

export type Reviewer = { name: string; avatar: string; verdict: 'approved' | 'changes_requested' | 'commented' | 'waiting' }

export function progress(pull: Pull) {
  const approved = pull.reviewers.filter((r) => r.verdict === 'approved').length
  return pull.reviewers.length
    ? t('review.approvedOf', { count: approved, total: pull.reviewers.length })
    : pull.approvals
      ? t('review.approvedCount', { count: pull.approvals })
      : t('review.noReviewer')
}

export function standing(pull: Pull): [string, string] {
  if (pull.changes_requested) return ['act-fix', t('review.changesRequested')]
  if (pull.reviewers.length && pull.reviewers.every((r) => r.verdict === 'approved')) return ['act-merge', t('review.approved')]
  if (pull.draft) return ['quiet', t('review.draft')]
  return ['wait', t('review.inReview')]
}

export type Lists = { review: Pull[]; mine: Pull[]; others: Pull[]; error: string }

export type Remark = {
  id: string
  author: string
  avatar: string
  body: string
  path: string | null
  line: number | null
  removed: boolean
  reply_to: string | null
  at: string
  kind: 'line' | 'general'
  resolved?: boolean | null
  commit?: string | null
}

export const pulls = $state<Record<string, Pull | null>>({})
export const reviews = $state<Record<string, Comment[]>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.reviews') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

$effect.root(() => {
  $effect(() => {
    const kept = Object.fromEntries(Object.entries($state.snapshot(reviews)).filter(([, list]) => list.length))
    try {
      localStorage.setItem('squadron.reviews', JSON.stringify(kept))
    } catch {}
  })
})
export function openReview(project: string, number: number) {
  projects.overview = false
  projects.board = false
  projects.pulls = true
  projects.current = project
  setPanel(project, null)
  requestPanel('pulls')
  refreshPull(project, number)
  layout.review = { project, number }
}
export function refreshPull(project: string, number: number) {
  const fetched = [lookUpLists(project, true)]
  if (`${project}#${number}` in numbered) fetched.push(lookUpNumbered(project, number, true))
  return Promise.all(fetched)
}
export const awaitsChecks = (project: string, pull: Pull) =>
  pull.state === 'open' && !pull.mergeable && pull.checks === 'pending' && !!lists[project]?.mine.some((p) => p.number === pull.number)
export const numbered = $state<Record<string, Pull | null>>({})
const asked = new Set<string>()

export async function lookUpNumbered(project: string, number: number, force = false) {
  const key = `${project}#${number}`
  if (!force && asked.has(key)) return
  asked.add(key)
  numbered[key] = await invoke<Pull>('numbered_pull', { dir: project, number }).catch(() => numbered[key] ?? null)
}
export const lists = $state<Record<string, Lists>>({})
const listed: Record<string, number> = {}

export async function lookUpLists(project: string, force = false) {
  if (!force && Date.now() - (listed[project] ?? 0) < 60_000) return
  listed[project] = Date.now()
  try {
    const found = await invoke<Omit<Lists, 'error'>>('pull_lists', { dir: project })
    forgetIgnored(project, found)
    lists[project] = { ...found, error: '' }
  } catch (e) {
    lists[project] = { review: [], mine: [], others: [], error: String(e) }
  }
}

export const reviewedHeads = $state<Record<string, string>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.reviewed') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

export type Round = { n: number; head: string; started: number; by: string; posted?: number; verdict?: string; comments?: number }

export const rounds = $state<Record<string, Round[]>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.rounds') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

$effect.root(() => {
  $effect(() => {
    const kept = Object.fromEntries(Object.entries($state.snapshot(rounds)).map(([key, list]) => [key, list.slice(-20)]))
    try {
      localStorage.setItem('squadron.rounds', JSON.stringify(kept))
    } catch {}
  })
})

export const openRound = (key: string) => {
  const last = rounds[key]?.at(-1)
  return last && !last.posted ? last : null
}

export const lastPosted = (key: string) => rounds[key]?.findLast((r) => r.posted) ?? null

export function roundFor(key: string, head: string, by = '') {
  const open = openRound(key)
  if (open && (!head || same(open.head, head))) return open
  if (open && !reviews[key]?.some((c) => c.round === open.n)) {
    Object.assign(open, { head, by, started: Date.now() })
    return open
  }
  const list = rounds[key] ?? []
  rounds[key] = [...list, { n: (list.at(-1)?.n ?? 0) + 1, head, started: Date.now(), by }]
  return rounds[key].at(-1)!
}

export function inRound(key: string, head: string, by = '') {
  const round = roundFor(key, head, by)
  return { round: round.n, head: round.head }
}

export function closeRound(key: string, head: string, verdict: string, comments: number) {
  const open = openRound(key) ?? roundFor(key, head)
  const now = Date.now()
  for (const round of rounds[key] ?? []) if (!round.posted && round !== open) round.posted = now
  Object.assign(open, { posted: now, verdict, comments })
}

export type Judgement = { thread: string; state: 'fixed' | 'answered' | 'open'; commit?: string; note: string; round: number; by: string; at: number }

export const judgements = $state<Record<string, Judgement[]>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.judged') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

$effect.root(() => {
  $effect(() => {
    const kept = Object.fromEntries(Object.entries($state.snapshot(judgements)).map(([key, list]) => [key, list.slice(-200)]))
    try {
      localStorage.setItem('squadron.judged', JSON.stringify(kept))
    } catch {}
  })
})

export function judge(key: string, judgement: Judgement) {
  const list = (judgements[key] ?? []).filter((j) => j.thread !== judgement.thread || j.round !== judgement.round)
  judgements[key] = [...list, judgement]
}

export const judgementOf = (key: string, thread: string) => judgements[key]?.findLast((j) => j.thread === thread) ?? null

export const threadRoots = (remarks: Remark[]) => {
  const ids = new Set(remarks.map((r) => r.id))
  return remarks.filter((r) => !r.reply_to || !ids.has(r.reply_to))
}

export function markReviewed(dir: string, pull: Pull, head = pull.head) {
  reviewedHeads[`${dir}#${pull.number}`] = head
  try {
    localStorage.setItem('squadron.reviewed', JSON.stringify(reviewedHeads))
  } catch {}
}

export const viewedFiles = $state<Record<string, Record<string, string>>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.viewed') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

export function setViewed(pull: Pull, path: string, blob: string | null) {
  const files = { ...viewedFiles[pull.url] }
  if (blob === null) delete files[path]
  else files[path] = blob
  if (Object.keys(files).length) viewedFiles[pull.url] = files
  else delete viewedFiles[pull.url]
  try {
    localStorage.setItem('squadron.viewed', JSON.stringify(viewedFiles))
  } catch {}
}

const same = (a: string, b: string) =>a.startsWith(b) || b.startsWith(a)

export function sinceOf(dir: string, pull: Pull) {
  return reviewedHeads[`${dir}#${pull.number}`] ?? pull.reviewed_head ?? pull.reviewed_at
}

export function moved(dir: string, pull: Pull) {
  const local = reviewedHeads[`${dir}#${pull.number}`]
  if (local) return !same(local, pull.head)
  if (pull.reviewed_head) return !same(pull.reviewed_head, pull.head)
  if (pull.reviewed_at && pull.head_at) return Date.parse(pull.head_at) > Date.parse(pull.reviewed_at)
  return false
}

export const ignoredPulls = $state<Record<string, string>>(
  (() => {
    try {
      return JSON.parse(localStorage.getItem('squadron.ignored') ?? '{}')
    } catch {
      return {}
    }
  })(),
)

const saveIgnored = () => {
  try {
    localStorage.setItem('squadron.ignored', JSON.stringify(ignoredPulls))
  } catch {}
}

export function ignore(dir: string, pull: Pull, untilCommits: boolean) {
  ignoredPulls[`${dir}#${pull.number}`] = untilCommits ? pull.head : ''
  saveIgnored()
}

export function unignore(dir: string, pull: Pull) {
  delete ignoredPulls[`${dir}#${pull.number}`]
  saveIgnored()
}

export const ignoredUntil = (dir: string, pull: Pull) => ignoredPulls[`${dir}#${pull.number}`]

export function ignored(dir: string, pull: Pull) {
  const head = ignoredUntil(dir, pull)
  return head === '' || (!!head && same(head, pull.head))
}

function forgetIgnored(dir: string, found: Omit<Lists, 'error'>) {
  const open = new Map([...found.review, ...found.mine, ...found.others].map((p) => [`${dir}#${p.number}`, p]))
  const stale = Object.keys(ignoredPulls).filter((key) => key.startsWith(`${dir}#`) && (!open.has(key) || !ignored(dir, open.get(key)!)))
  for (const key of stale) delete ignoredPulls[key]
  if (stale.length) saveIgnored()
}

export const waitsFor = (dir: string, pull: Pull) => !ignored(dir, pull) && (!pull.reviewed || (pull.reviewed !== 'approved' && moved(dir, pull)))

export const needsYou = (project: string) => {
  const found = lists[project]
  return found ? found.review.filter((p) => waitsFor(project, p)).length + found.mine.filter((p) => p.changes_requested).length : 0
}
const checked: Record<string, number> = {}

export async function lookUp(dir: string, force = false) {
  if (!force && Date.now() - (checked[dir] ?? 0) < 60_000) return
  checked[dir] = Date.now()
  try {
    pulls[dir] = await invoke<Pull | null>('pull_request', { dir })
  } catch {
    pulls[dir] = null
  }
}

export const owned = $state<Record<string, Pull>>({})
const ownChecked: Record<string, number> = {}

async function lookUpOwned(agent: Agent, force = false) {
  if (!force && Date.now() - (ownChecked[agent.id] ?? 0) < 60_000) return
  ownChecked[agent.id] = Date.now()
  const ref = pushedTo(agent, providedLinks)
  try {
    const found = !ref
      ? null
      : 'url' in ref
        ? (await invoke<Linked>('linked_pull', { url: ref.url, projects: [] })).pull
        : await invoke<Pull | null>('pull_request', { dir: workdir(agent), branch: ref.branch })
    if (found) owned[agent.id] = found
    else delete owned[agent.id]
  } catch {
    delete owned[agent.id]
  }
}

export const pullOf = (agent: Agent) => owned[agent.id] ?? pulls[workdir(agent)] ?? null

export function lookUpAgent(agent: Agent, force = false) {
  lookUp(workdir(agent), force)
  lookUpOwned(agent, force)
}

export const lookUpAll = (force = false) => {
  new Set(agents.list.map(workdir)).forEach((dir) => lookUp(dir, force))
  agents.list.forEach((agent) => lookUpOwned(agent, force))
}

const tidied = new Set<string>()

export function tidyMerged(agent: Agent) {
  const worktree = agent.worktree
  const pull = pullOf(agent)
  if (!worktree || agent.alive || agent.review || pull?.state !== 'merged' || pull.branch !== worktree.branch || tidied.has(agent.id)) return
  if (agents.list.some((a) => a.id !== agent.id && a.worktree?.path === worktree.path)) return
  tidied.add(agent.id)
  return invoke('drop_merged_worktree', { dir: worktree.path, branch: worktree.branch, head: pull.head })
    .then(() => {
      if (agent.worktree?.path !== worktree.path) return
      agent.worktree = null
      agent.pinned = false
      persist(agent)
      return invoke('set_workdir', { id: agent.id, dir: null })
    })
    .catch(() => {})
}

$effect.root(() => {
  $effect(() => {
    for (const agent of agents.list) tidyMerged(agent)
  })
})

const VERDICTS: Record<string, string> = { approve: 'approved it', request_changes: 'asked for changes', '': 'commented on it' }

function tell(pull: Pull, text: string, comments: Comment[] = []) {
  for (const agent of agents.list)
    if (!agent.review && agent.alive && !agent.asks.length && agent.status !== 'error' && pullOf(agent)?.url === pull.url)
      send(agent.id, text, comments, [], '', FOR_INFO)
}

type Posted = { done: number; said: boolean; error: string | null; sent: Comment[] }

export async function post(dir: string, pull: Pull, list: Comment[], summary = '', verdict = '', head = pull.head): Promise<Posted> {
  const groups = byHead(list, head)
  const sent: Comment[] = []
  let said = false
  try {
    for (const [i, { at, items }] of groups.entries()) {
      const last = i === groups.length - 1
      const drafts = items.map((c) => ({ path: c.path, line: c.line, removed: c.removed, body: c.note }))
      const result = await invoke<{ done: number; said: boolean; error: string | null }>('post_remarks', {
        dir,
        number: pull.number,
        head: at,
        drafts,
        summary: last ? summary : '',
        verdict: last ? verdict : '',
      })
      sent.push(...items.slice(0, result.done))
      if (result.error) return { done: sent.length, said: false, error: result.error, sent }
      said = last && result.said
    }
    tell(
      pull,
      [`The user posted a review of your pull request #${pull.number} on ${hostName(pull)}: they ${VERDICTS[verdict] ?? VERDICTS['']}.`, summary.trim()].filter(Boolean).join('\n\n'),
      list,
    )
    return { done: sent.length, said, error: null, sent }
  } catch (e) {
    return { done: sent.length, said: false, error: String(e), sent }
  } finally {
    lookUp(dir, true)
  }
}

export async function approve(dir: string, pull: Pull) {
  const { error } = await post(dir, pull, [], '', 'approve')
  if (error) throw error
  closeRound(`${dir}#${pull.number}`, pull.head, 'approve', 0)
  markReviewed(dir, pull)
}

export async function requestChanges(dir: string, pull: Pull) {
  const { error } = await post(dir, pull, [], '', 'request_changes')
  if (error) throw error
  closeRound(`${dir}#${pull.number}`, pull.head, 'request_changes', 0)
  markReviewed(dir, pull)
}

export async function merge(dir: string, pull: Pull) {
  await invoke('merge_pull', { dir, number: pull.number })
  lookUpAll(true)
}

export const REVIEW_PROMPT = 'Leave out style nitpicks.\n\nEnd with a short summary of the pull request and of your main concerns.'

export const reviewPrompt = (project: string) => projects.list.find((p) => p.path === project)?.review_prompt || settings.review_prompt || REVIEW_PROMPT

export async function reviewWithAgent(project: string, pull: Pull) {
  const dir = await invoke<string>('review_checkout', { project, number: pull.number, branch: pull.branch, base: pull.base, head: pull.head })
  const prompt = [
    `Review pull request #${pull.number} “${pull.title}” by ${pull.author}, from ${pull.branch} into ${pull.base}, on ${hostName(pull)}.`,
    `This folder is a checkout of the pull request branch: git diff origin/${pull.base}...HEAD shows what it changes. Read the code around the changes too.`,
    'Do not change any file. For each problem worth raising, call add_review_comment with the file, a line the diff shows (changed or within its context: the line in the new version, or the removed line with removed: true) and a short comment. When it is about code the diff does not show, put it on the changed line it relates to best, or leave path and line out to comment on the whole pull request: what is wrong and what to do.',
    reviewPrompt(project),
  ].join('\n\n')
  const before = agents.current[project]
  const id = await create(project, t('review.agentName', { number: String(pull.number) }), defaultsFor(project), prompt, { dir })
  if (before) agents.current[project] = before
  else delete agents.current[project]
  const agent = agents.list.find((a) => a.id === id)
  if (agent) {
    agent.review = { project, number: pull.number, base: pull.base, title: pull.title, head: pull.head }
    roundFor(`${project}#${pull.number}`, pull.head, id)
    persist(agent)
  }
  return id
}

export const behind = (agent: Agent, pull: Pull) => (agent.review?.head ? !same(agent.review.head, pull.head) : moved(agent.review!.project, pull))

export const reviewerOf = (dir: string, pull: Pull) => agents.list.find((a) => a.review?.project === dir && a.review.number === pull.number)

export async function reviewChanges(agent: Agent, pull: Pull) {
  const review = agent.review!
  await invoke('review_checkout', { project: review.project, number: pull.number, branch: pull.branch, base: pull.base, head: pull.head })
  const since = review.head ?? sinceOf(review.project, pull)
  roundFor(`${review.project}#${pull.number}`, pull.head, agent.id)
  const remarks = await invoke<Remark[]>('pull_remarks', { dir: review.project, number: pull.number }).catch(() => null)
  const open = remarks && threadRoots(remarks).filter((r) => r.resolved !== true && !(r.path && r.line === null))
  const threads = !open
    ? 'Check whether the remarks already posted on the pull request are addressed: read_pull_comments shows them.'
    : open.length
      ? [
          'These threads are still open on the pull request (read_pull_comments shows them with their replies):',
          open.map((r) => `- ${r.id}: ${r.path ? `${r.path}:${r.line}` : 'whole pull request'}, by ${r.author}: ${r.body.split('\n')[0].slice(0, 120)}`).join('\n'),
          'For each of them, call judge_review_thread: fixed (with the commit that fixes it), answered (the author replied with a reason that holds), or open.',
        ].join('\n')
      : ''
  const shown = since && /^[0-9a-f]{7,40}$/.test(since) ? `git diff ${since}..HEAD shows them` : `git log origin/${pull.base}..HEAD lists them`
  send(
    agent.id,
    [
      `${pull.author} pushed new commits to pull request #${pull.number} since your review. This folder now holds them: ${shown}.`,
      threads,
      'Look only at what changed, and raise what is new and wrong with add_review_comment, as before. Do not change any file.',
    ]
      .filter(Boolean)
      .join('\n\n'),
    [],
    [],
    '',
    'End with a short summary of what changed and of what is still to fix.',
  )
  agent.review = { ...review, head: pull.head }
  persist(agent)
}

export async function fixWithAgent(project: string, pull: Pull, summary: string, list: Comment[]) {
  const dir = await invoke<string>('review_checkout', { project, number: pull.number, branch: pull.branch, base: pull.base, head: pull.head })
  const intro = [
    `These remarks come from a review of pull request #${pull.number} “${pull.title}” by ${pull.author}, from ${pull.branch} into ${pull.base}, on ${hostName(pull)}.`,
    `This folder is a detached checkout of origin/${pull.branch}: git diff origin/${pull.base}...HEAD shows what the pull request changes.`,
    `Address each remark in the code and commit the fixes. Do not push: say what is ready, the user pushes with git push origin HEAD:${pull.branch} or asks you to.`,
  ].join('\n\n')
  return create(project, t('review.fixAgentName', { number: String(pull.number) }), defaultsFor(project), `${intro}\n\n${prompt(summary, list)}`, { dir })
}

export const hostName = (on: { host: string }) => forgeName(on.host)

const STATES: Record<Pull['state'], Key> = {
  open: 'review.stateOpen',
  merged: 'review.stateMerged',
  declined: 'review.stateDeclined',
  closed: 'review.stateClosed',
  superseded: 'review.stateSuperseded',
}

export const stateLabel = (pull: Pull) =>
  pull.draft && pull.state === 'open' ? t('review.draft') : STATES[pull.state] ? t(STATES[pull.state]) : pull.state[0].toUpperCase() + pull.state.slice(1)
