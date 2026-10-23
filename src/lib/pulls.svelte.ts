import { invoke } from '@tauri-apps/api/core'
import { agents } from './agents.svelte'
import { workdir } from './stream'

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
}

export type Remark = { id: number; author: string; body: string; path: string | null; line: number | null; removed: boolean; reply_to: number | null; at: string }

export const pulls = $state<Record<string, Pull | null>>({})
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

export const hostName = (pull: Pull) => (pull.host === 'github' ? 'GitHub' : 'Bitbucket')

export const stateLabel = (pull: Pull) => (pull.draft && pull.state === 'open' ? 'Draft' : pull.state[0].toUpperCase() + pull.state.slice(1))
