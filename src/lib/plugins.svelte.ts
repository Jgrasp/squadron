import { invoke } from '@tauri-apps/api/core'
import { errorText } from './i18n/index.svelte'
import { contained } from './groups'
import { ALL, activeIn, after, everywhere, followed, plan, type Details, type Entry, type Flags, type Follow, type Installed, type Marketplace, type Target } from './plugins'
import type { Kind, PluginRef } from './pluginuse'
import { projects } from './projects.svelte'
import { saveSettings, settings } from './settings.svelte'

export const plugins = $state({
  installed: null as Installed[] | null,
  catalog: null as Entry[] | null,
  marketplaces: [] as Marketplace[],
  flags: { user: {}, projects: {} } as Flags,
  details: {} as Record<string, Details | null>,
  loading: false,
  error: '',
  busy: '',
  changed: false,
})

const paths = () => projects.list.map((p) => p.path)

export const members = (group: string) => contained(projects.groups, projects.list, group).map((p) => p.path)

export async function loadPlugins() {
  if (plugins.loading) return
  plugins.loading = true
  plugins.error = ''
  try {
    const [installed, marketplaces, flags] = await Promise.all([
      invoke<Installed[]>('plugins_installed'),
      invoke<Marketplace[]>('plugins_marketplaces').catch(() => []),
      invoke<Flags>('plugin_flags', { projects: paths() }),
    ])
    plugins.installed = installed.filter((p, i) => installed.findIndex((q) => q.id === p.id) === i)
    plugins.marketplaces = marketplaces
    plugins.flags = flags
  } catch (e) {
    plugins.error = errorText(e)
  } finally {
    plugins.loading = false
  }
}

export async function loadCatalog() {
  if (plugins.catalog) return
  try {
    plugins.catalog = await invoke<Entry[]>('plugins_catalog')
  } catch (e) {
    plugins.catalog = []
    plugins.error = errorText(e)
  }
}

export function fetchDetails(ids: string[]) {
  for (const id of ids) {
    if (id in plugins.details) continue
    plugins.details[id] = null
    invoke<Details>('plugin_details', { id })
      .then((d) => {
        plugins.details[id] = d
      })
      .catch(() => {})
  }
}

export const detailsOf = (id: string) => plugins.details[id] ?? null

export const skillCost = (refs: PluginRef[]) => (plugin: string, skill: string) =>
  detailsOf(refs.find((r) => r.name === plugin)?.id ?? '')?.components.find((c) => c.kind === 'skill' && c.name === skill)?.invoke ?? 0

export const where = (id: string) => {
  const all = paths()
  return { all: everywhere(plugins.flags, id, all), on: activeIn(plugins.flags, id, all), total: all.length }
}

export const remembered = (id: string) => settings.plugin_projects[id]

export const followOf = (id: string): Follow | undefined => settings.plugin_groups[id]

async function attempt(busy: string, work: () => Promise<unknown>) {
  plugins.busy = busy
  plugins.error = ''
  try {
    await work()
    plugins.changed = true
  } catch (e) {
    plugins.error = errorText(e)
  } finally {
    plugins.busy = ''
    await loadPlugins()
  }
}

async function steps(id: string, target: Target) {
  for (const step of plan(plugins.flags, id, paths(), target)) {
    await invoke('plugin_run', { args: step.args, dir: step.dir ?? null })
    plugins.flags = after(plugins.flags, [step])
  }
}

async function remember(id: string, kept: string[] | null) {
  const next = { ...$state.snapshot(settings.plugin_projects) }
  if (kept) next[id] = kept
  else delete next[id]
  await saveSettings({ ...$state.snapshot(settings), plugin_projects: next })
}

async function follow(id: string, next: Follow | null) {
  if (!next?.groups.length && !followOf(id)) return
  const all = { ...$state.snapshot(settings.plugin_groups) }
  if (next?.groups.length) all[id] = next
  else delete all[id]
  await saveSettings({ ...$state.snapshot(settings), plugin_groups: all })
}

const following = (target: Target, groups: string[]): Follow | null =>
  Array.isArray(target) && groups.length ? { groups, paths: target.filter((p) => !groups.some((g) => members(g).includes(p))) } : null

export const choose = (id: string, target: Target, groups: string[] = []) =>
  attempt(id, async () => {
    await steps(id, target)
    if (target !== 'off' && remembered(id)) await remember(id, null)
    await follow(id, following(target, groups))
  })

export const toggle = (id: string, on: boolean) =>
  attempt(id, async () => {
    if (on) {
      const kept = remembered(id)
      const f = followOf(id)
      await steps(id, f ? followed(f, members, paths()) : !kept || kept.includes(ALL) ? 'all' : kept)
      await remember(id, null)
    } else {
      const now = where(id)
      await remember(id, now.all ? [ALL] : now.on)
      await steps(id, 'off')
    }
  })

export const install = (id: string, target: Target, groups: string[] = []) =>
  attempt(id, async () => {
    await invoke('plugin_run', { args: ['install', id, '--scope', 'user'], dir: null })
    plugins.flags = after(plugins.flags, [{ args: ['enable', id, '--scope', 'user'] }])
    await steps(id, target)
    await follow(id, following(target, groups))
  })

export const uninstall = (plugin: Installed) =>
  attempt(plugin.id, async () => {
    await invoke('plugin_run', { args: ['uninstall', plugin.id, '--scope', plugin.scope], dir: plugin.projectPath ?? null })
    await follow(plugin.id, null)
  })

export const followKey = () =>
  JSON.stringify([projects.list.map((p) => [p.path, p.group]), projects.groups.map((g) => [g.id, g.parent]), settings.plugin_groups])

let syncing = false
let again = false

export async function syncFollows() {
  const ids = Object.keys(settings.plugin_groups).filter((id) => !remembered(id))
  if (!ids.length) return
  if (syncing) return void (again = true)
  syncing = true
  try {
    const all = paths()
    const [installed, flags] = await Promise.all([invoke<Installed[]>('plugins_installed'), invoke<Flags>('plugin_flags', { projects: all })])
    plugins.flags = flags
    for (const id of ids) {
      const f = followOf(id)
      if (!f) continue
      if (!installed.some((p) => p.id === id)) {
        await follow(id, null)
        continue
      }
      const alive = f.groups.filter((g) => projects.groups.some((x) => x.id === g))
      const target = followed({ groups: alive, paths: f.paths }, members, all)
      const kept = alive.length < f.groups.length ? activeIn(flags, id, all).filter((p) => !target.includes(p)) : []
      if (alive.length < f.groups.length) await follow(id, { groups: alive, paths: [...f.paths, ...kept] })
      const want = [...target, ...kept]
      if (plan(plugins.flags, id, all, want).length) await attempt(id, () => steps(id, want))
    }
  } catch (e) {
    plugins.error = errorText(e)
  } finally {
    syncing = false
    if (again) {
      again = false
      syncFollows()
    }
  }
}

export const update = (plugin: Installed) =>
  attempt(plugin.id, async () => {
    await invoke('plugin_run', { args: ['update', plugin.id, '--scope', plugin.scope], dir: plugin.projectPath ?? null })
    delete plugins.details[plugin.id]
  })

export const addMarketplace = (source: string) =>
  attempt('marketplace', async () => {
    await invoke('plugin_run', { args: ['marketplace', 'add', source], dir: null })
    plugins.catalog = null
  })

export const removeMarketplace = (name: string) =>
  attempt('marketplace', async () => {
    await invoke('plugin_run', { args: ['marketplace', 'remove', name], dir: null })
    plugins.catalog = null
  })

export type View = { plugin: string } | { kind: Kind | 'start'; plugin?: string } | { use: string }

export const usage = $state({ agent: '', stack: [] as View[] })

export function openUsage(agent: string, view: View, nested = false) {
  usage.stack = nested && usage.agent === agent ? [...usage.stack, view] : [view]
  usage.agent = agent
}

export function closeUsage() {
  usage.stack = []
}

export function reveal(id: string) {
  const row = document.querySelector<HTMLElement>(`[data-call~="${CSS.escape(id)}"]`)
  if (!row) return
  for (let up = row.parentElement?.closest('details'); up; up = up.parentElement?.closest('details')) up.open = true
  row.scrollIntoView({ block: 'center', behavior: 'smooth' })
  row.animate(
    [{ boxShadow: 'inset 0 0 0 1px var(--accent)', background: 'color-mix(in srgb, var(--accent) 18%, transparent)' }, { boxShadow: 'none', background: 'transparent' }],
    { duration: 1800, easing: 'ease-out' },
  )
}
