import { invoke } from '@tauri-apps/api/core'
import { agents, create, persist } from './agents.svelte'
import { projects } from './projects.svelte'
import { settings } from './settings.svelte'
import { workdir, type Comment } from './stream'

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
  updated: string
  reviewed: 'approved' | 'changes_requested' | null
  reviewers: Reviewer[]
  conflicts: boolean
}

export type Reviewer = { name: string; avatar: string; verdict: 'approved' | 'changes_requested' | 'commented' | 'waiting' }

export function progress(pull: Pull) {
  const approved = pull.reviewers.filter((r) => r.verdict === 'approved').length
  return pull.reviewers.length ? `${approved}/${pull.reviewers.length} approved` : pull.approvals ? `${pull.approvals} approved` : 'No reviewer'
}

export function standing(pull: Pull): [string, string] {
  if (pull.changes_requested) return ['changes_requested', 'Changes requested']
  if (pull.reviewers.length && pull.reviewers.every((r) => r.verdict === 'approved')) return ['approved', 'Approved']
  if (pull.draft) return ['draft', 'Draft']
  return ['open', 'In review']
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
export const reviews = $state<Record<string, Comment[]>>({})
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

export const needsYou = (project: string) => {
  const found = lists[project]
  return found ? found.review.filter((p) => !p.reviewed).length + found.mine.filter((p) => p.changes_requested).length : 0
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

export const lookUpAll = (force = false) => new Set(agents.list.map(workdir)).forEach((dir) => lookUp(dir, force))

export async function post(dir: string, pull: Pull, list: Comment[], summary = '', verdict = '') {
  const drafts = list.map((c) => ({ path: c.path, line: c.line, removed: c.removed, body: c.note }))
  try {
    return await invoke<{ done: number; error: string | null }>('post_remarks', { dir, number: pull.number, head: pull.head, drafts, summary, verdict })
  } catch (e) {
    return { done: 0, error: String(e) }
  } finally {
    lookUp(dir, true)
  }
}

export async function reviewWithAgent(project: string, pull: Pull) {
  const dir = await invoke<string>('review_checkout', { project, number: pull.number, branch: pull.branch, base: pull.base })
  const prompt = [
    `Review pull request #${pull.number} “${pull.title}” by ${pull.author}, from ${pull.branch} into ${pull.base}, on ${hostName(pull)}.`,
    `This folder is a checkout of the pull request branch: git diff origin/${pull.base}...HEAD shows what it changes. Read the code around the changes too.`,
    'Do not change any file. For each problem worth raising, call add_review_comment with the file, a line the diff shows (changed or within its context: the line in the new version, or the removed line with removed: true) and a short comment. When it is about code the diff does not show, put it on the changed line it relates to best, or leave path and line out to comment on the whole pull request: what is wrong and what to do. Leave out style nitpicks.',
    'End with a short summary of the pull request and of your main concerns.',
  ].join('\n\n')
  const id = await create(project, `Review #${pull.number}`, { ...settings.defaults }, prompt, { dir })
  const agent = agents.list.find((a) => a.id === id)
  if (agent) {
    agent.review = { project, number: pull.number, base: pull.base }
    persist(agent)
  }
  return id
}

export const hostName = (pull: Pull) => (pull.host === 'github' ? 'GitHub' : 'Bitbucket')

export const stateLabel = (pull: Pull) => (pull.draft && pull.state === 'open' ? 'Draft' : pull.state[0].toUpperCase() + pull.state.slice(1))
