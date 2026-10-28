import { invoke } from '@tauri-apps/api/core'
import { agents, create, persist, send } from './agents.svelte'
import { projects } from './projects.svelte'
import { settings } from './settings.svelte'
import { prompt, workdir, type Agent, type Comment } from './stream'
import { pushedTo } from './pushed'
import type { Linked } from './linked.svelte'
import { t, type Key } from './i18n/index.svelte'

export type Pull = {
  host: 'bitbucket' | 'github'
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

export type Lists = { review: Pull[]; mine: Pull[]; error: string }

export type Remark = {
  id: number
  author: string
  body: string
  path: string | null
  line: number | null
  removed: boolean
  reply_to: number | null
  at: string
  kind: 'line' | 'general'
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
export const reopen = $state<{ number: number | null }>({ number: null })

export function openReview(project: string, number: number) {
  reopen.number = number
  projects.overview = false
  projects.board = false
  projects.current = project
  projects.pulls = true
}
export const lists = $state<Record<string, Lists>>({})
const listed: Record<string, number> = {}

export async function lookUpLists(project: string, force = false) {
  if (!force && Date.now() - (listed[project] ?? 0) < 60_000) return
  listed[project] = Date.now()
  try {
    lists[project] = { ...(await invoke<Omit<Lists, 'error'>>('pull_lists', { dir: project })), error: '' }
  } catch (e) {
    lists[project] = { review: [], mine: [], error: String(e) }
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

export function markReviewed(dir: string, pull: Pull) {
  reviewedHeads[`${dir}#${pull.number}`] = pull.head
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

export const waitsFor = (dir: string, pull: Pull) => !pull.reviewed || moved(dir, pull)

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
  const ref = pushedTo(agent)
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

const VERDICTS: Record<string, string> = { approve: 'approved it', request_changes: 'asked for changes', '': 'commented on it' }

function tell(pull: Pull, text: string, comments: Comment[] = []) {
  for (const agent of agents.list)
    if (!agent.review && agent.alive && !agent.asks.length && agent.status !== 'error' && pullOf(agent)?.url === pull.url)
      send(agent.id, `${text}\n\nThis is for your information: wait for the user before you change, commit or push anything about it.`, comments)
}

export async function post(dir: string, pull: Pull, list: Comment[], summary = '', verdict = '') {
  const drafts = list.map((c) => ({ path: c.path, line: c.line, removed: c.removed, body: c.note }))
  try {
    const result = await invoke<{ done: number; error: string | null }>('post_remarks', { dir, number: pull.number, head: pull.head, drafts, summary, verdict })
    if (!result.error)
      tell(
        pull,
        [`The user posted a review of your pull request #${pull.number} on ${hostName(pull)}: they ${VERDICTS[verdict] ?? VERDICTS['']}.`, summary.trim()].filter(Boolean).join('\n\n'),
        list,
      )
    return result
  } catch (e) {
    return { done: 0, error: String(e) }
  } finally {
    lookUp(dir, true)
  }
}

export async function approve(dir: string, pull: Pull) {
  const { error } = await post(dir, pull, [], '', 'approve')
  if (error) throw error
  markReviewed(dir, pull)
}

export async function merge(dir: string, pull: Pull) {
  await invoke('merge_pull', { dir, number: pull.number })
  tell(pull, `The user merged your pull request #${pull.number} into ${pull.base} on ${hostName(pull)}.`)
  lookUpAll(true)
}

export const REVIEW_PROMPT = 'Leave out style nitpicks.\n\nEnd with a short summary of the pull request and of your main concerns.'

export const reviewPrompt = (project: string) => projects.list.find((p) => p.path === project)?.review_prompt || settings.review_prompt || REVIEW_PROMPT

export async function reviewWithAgent(project: string, pull: Pull) {
  const dir = await invoke<string>('review_checkout', { project, number: pull.number, branch: pull.branch, base: pull.base })
  const prompt = [
    `Review pull request #${pull.number} “${pull.title}” by ${pull.author}, from ${pull.branch} into ${pull.base}, on ${hostName(pull)}.`,
    `This folder is a checkout of the pull request branch: git diff origin/${pull.base}...HEAD shows what it changes. Read the code around the changes too.`,
    'Do not change any file. For each problem worth raising, call add_review_comment with the file, a line the diff shows (changed or within its context: the line in the new version, or the removed line with removed: true) and a short comment. When it is about code the diff does not show, put it on the changed line it relates to best, or leave path and line out to comment on the whole pull request: what is wrong and what to do.',
    reviewPrompt(project),
  ].join('\n\n')
  const id = await create(project, t('review.agentName', { number: String(pull.number) }), { ...settings.defaults }, prompt, { dir })
  const agent = agents.list.find((a) => a.id === id)
  if (agent) {
    agent.review = { project, number: pull.number, base: pull.base, title: pull.title }
    persist(agent)
  }
  return id
}

export async function fixWithAgent(project: string, pull: Pull, summary: string, list: Comment[]) {
  const dir = await invoke<string>('review_checkout', { project, number: pull.number, branch: pull.branch, base: pull.base })
  const intro = [
    `These remarks come from a review of pull request #${pull.number} “${pull.title}” by ${pull.author}, from ${pull.branch} into ${pull.base}, on ${hostName(pull)}.`,
    `This folder is a detached checkout of origin/${pull.branch}: git diff origin/${pull.base}...HEAD shows what the pull request changes.`,
    `Address each remark in the code and commit the fixes. Do not push: say what is ready, the user pushes with git push origin HEAD:${pull.branch} or asks you to.`,
  ].join('\n\n')
  return create(project, t('review.fixAgentName', { number: String(pull.number) }), { ...settings.defaults }, `${intro}\n\n${prompt(summary, list)}`, { dir })
}

export const hostName = (pull: Pull) => (pull.host === 'github' ? 'GitHub' : 'Bitbucket')

const STATES: Record<Pull['state'], Key> = {
  open: 'review.stateOpen',
  merged: 'review.stateMerged',
  declined: 'review.stateDeclined',
  closed: 'review.stateClosed',
  superseded: 'review.stateSuperseded',
}

export const stateLabel = (pull: Pull) =>
  pull.draft && pull.state === 'open' ? t('review.draft') : STATES[pull.state] ? t(STATES[pull.state]) : pull.state[0].toUpperCase() + pull.state.slice(1)
