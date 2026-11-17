import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { activityOf, agentChanges, groupOf, projectChanges, projectOf, usageOf, type AgentActivity, type Fed } from './activity'
import { agents, archived, archiveShown, claudeApi, find, limits, show, shownAgent, type Archived } from './agents.svelte'
import { active, grantsOf } from './extensions'
import { extensions } from './extensions.svelte'
import { layout } from './layout.svelte'
import { projects } from './projects.svelte'

const ACTIVITY = 'agents:activity'
const PROJECTS = 'projects:read'
const USAGE = 'usage:read'
const STALE = 5 * 60_000

const main = getCurrentWindow().label === 'main'

const grants = () => new Set(active(extensions.list).filter((m) => m.manifest.wasm).flatMap(grantsOf))

const shelf = { list: [] as Archived[], at: 0, loading: null as Promise<Archived[]> | null }

function shelved(fresh = false) {
  if (!fresh && shelf.at && Date.now() - shelf.at < STALE) return Promise.resolve(shelf.list)
  shelf.loading ??= archived()
    .then((list) => Object.assign(shelf, { list, at: Date.now() }).list)
    .catch(() => shelf.list)
    .finally(() => (shelf.loading = null))
  return shelf.loading
}

const live = () => agents.list.map((a) => activityOf(a))

async function everyone(fresh = false): Promise<AgentActivity[]> {
  const ids = new Set(agents.list.map((a) => a.id))
  const old = (await shelved(fresh)).filter((a) => !ids.has(a.id)).map((a) => activityOf(a, true))
  return [...live(), ...old]
}

async function projectList() {
  const all = await everyone()
  return [...projects.list, ...projects.archived].map((p) => projectOf(p, all))
}

const usage = () => usageOf(limits, claudeApi)

function onScreen(extension: string) {
  const kind = layout.panels[projects.current]
  return !projects.overview && !projects.board && !projects.pulls && !!shownAgent() && !!kind?.startsWith(`extension:${extension}.`) && document.visibilityState === 'visible'
}

async function showAgent(extension: string, id: string) {
  if (!onScreen(extension)) throw new Error('show-agent works while one of the extension’s panels is on screen.')
  const agent = find(id)
  if (agent) return show(agent)
  const old = (await shelved(true)).find((a) => a.id === id)
  if (!old || !projects.list.some((p) => p.path === old.project && p.exists)) throw new Error(`No agent has the id ${id}.`)
  projects.overview = projects.board = projects.pulls = false
  projects.current = old.project
  Object.assign(archiveShown, { project: old.project, agent: old.id })
}

const CALLS: Record<string, (extension: string, args: Record<string, unknown>) => Promise<unknown> | unknown> = {
  projects: () => projectList(),
  groups: () => projects.groups.map(groupOf),
  'all-agents': () => everyone(),
  usage: () => usage(),
  'show-agent': (extension, args) => showAgent(extension, String(args.agent)).then(() => null),
}

if (main)
  listen<{ extension: string; call: string; reply?: string; [key: string]: unknown }>('extensions-host', async ({ payload: p }) => {
    const call = CALLS[p.call]
    if (!call || typeof p.reply !== 'string') return
    try {
      await invoke('extensions_wasm_answer', { reply: p.reply, value: (await call(p.extension, p)) ?? null, error: null })
    } catch (e) {
      await invoke('extensions_wasm_answer', { reply: p.reply, value: null, error: e instanceof Error ? e.message : String(e) })
    }
  })

const seen = { agents: new Map<string, string>(), projects: new Map<string, string>(), groups: '', usage: '', started: new Set<string>() }

const feed = (list: Fed[]) => list.length && invoke('extensions_wasm_feed', { list }).catch(() => {})

const fresh = (grant: string) => {
  const first = !seen.started.has(grant)
  seen.started.add(grant)
  return first
}

async function agentEvents(first: boolean) {
  const changes = agentChanges(seen.agents, live(), new Set(shelf.list.map((a) => a.id)), first)
  if (!changes.gone.length && !first) return changes.events
  const old = await shelved(true)
  return [
    ...changes.events,
    ...changes.gone.flatMap((id) => {
      const agent = old.find((a) => a.id === id)
      return agent ? [{ name: 'agent.archived', data: activityOf(agent, true) }] : []
    }),
  ]
}

async function projectEvents(first: boolean) {
  const list = projectChanges(seen.projects, await projectList(), first)
  const groups = JSON.stringify(projects.groups.map(groupOf))
  if (!first && groups !== seen.groups) list.push({ name: 'groups.changed', data: { groups: JSON.parse(groups) } })
  seen.groups = groups
  return list
}

function usageEvents(first: boolean) {
  const now = JSON.stringify(usage())
  const changed = !first && now !== seen.usage
  seen.usage = now
  return changed ? [{ name: 'usage', data: JSON.parse(now) }] : []
}

let ticking = false

async function tick() {
  const wanted = grants()
  if (ticking) return
  ticking = true
  try {
    const list: Fed[] = []
    if (wanted.has(ACTIVITY)) list.push(...(await agentEvents(fresh(ACTIVITY))))
    else seen.started.delete(ACTIVITY)
    if (wanted.has(PROJECTS)) list.push(...(await projectEvents(fresh(PROJECTS))))
    else seen.started.delete(PROJECTS)
    if (wanted.has(USAGE)) list.push(...usageEvents(fresh(USAGE)))
    else seen.started.delete(USAGE)
    await feed(list)
  } finally {
    ticking = false
  }
}

if (main) setInterval(tick, 1000)

export const messageSent = (from: string, to: string) => {
  if (grants().has(ACTIVITY)) feed([{ name: 'agent.message', data: { from, to } }])
}
