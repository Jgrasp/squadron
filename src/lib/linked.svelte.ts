import { invoke } from '@tauri-apps/api/core'
import { projects } from './projects.svelte'
import type { Pull } from './pulls.svelte'

export type Linked = {
  repo: string
  pull: Pull
  additions: number | null
  deletions: number | null
  files: number | null
  project: string | null
}

const PULL = /^https?:\/\/(www\.)?(github\.com\/[\w.-]+\/[\w.-]+\/pull|bitbucket\.org\/[\w.-]+\/[\w.-]+\/pull-requests)\/\d+([/?#].*)?$/

export const isPullLink = (url: string) => PULL.test(url)

const found = new Map<string, { at: number; linked: Promise<Linked> }>()

export function lookUpLink(url: string) {
  const hit = found.get(url)
  if (hit && Date.now() - hit.at < 5 * 60_000) return hit.linked
  const linked = invoke<Linked>('linked_pull', { url, projects: projects.list.map((p) => p.path) })
  found.set(url, { at: Date.now(), linked })
  linked.catch(() => found.delete(url))
  return linked
}

export function showPulls(project: string) {
  projects.overview = false
  projects.board = false
  projects.current = project
  projects.pulls = true
}
