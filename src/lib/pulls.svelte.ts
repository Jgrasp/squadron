import { invoke } from '@tauri-apps/api/core'
import { agents } from './agents.svelte'
import { workdir, type Comment } from './stream'

export type Pull = {
  host: 'bitbucket' | 'github'
  number: number
  title: string
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
}

export type Lists = { review: Pull[]; mine: Pull[]; error: string }

export type Remark = { id: number; author: string; body: string; path: string | null; line: number | null; removed: boolean; reply_to: number | null; at: string }

export const pulls = $state<Record<string, Pull | null>>({})
export const reviews = $state<Record<string, Comment[]>>({})
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

export const hostName = (pull: Pull) => (pull.host === 'github' ? 'GitHub' : 'Bitbucket')

export const stateLabel = (pull: Pull) => (pull.draft && pull.state === 'open' ? 'Draft' : pull.state[0].toUpperCase() + pull.state.slice(1))
