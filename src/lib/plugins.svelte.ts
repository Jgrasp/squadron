import { invoke } from '@tauri-apps/api/core'
import { errorText } from './i18n/index.svelte'
import { ALL, activeIn, after, everywhere, plan, type Details, type Entry, type Flags, type Installed, type Marketplace, type Target } from './plugins'
import type { Kind } from './pluginuse'
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

export const where = (id: string) => {
  const all = paths()
  return { all: everywhere(plugins.flags, id, all), on: activeIn(plugins.flags, id, all), total: all.length }
}

export const remembered = (id: string) => settings.plugin_projects[id]

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

export const choose = (id: string, target: Target) =>
  attempt(id, async () => {
    await steps(id, target)
    if (target !== 'off' && remembered(id)) await remember(id, null)
  })

export const toggle = (id: string, on: boolean) =>
  attempt(id, async () => {
    if (on) {
      const kept = remembered(id)
      await steps(id, !kept || kept.includes(ALL) ? 'all' : kept)
      await remember(id, null)
    } else {
      const now = where(id)
      await remember(id, now.all ? [ALL] : now.on)
      await steps(id, 'off')
    }
  })

export const install = (id: string, target: Target) =>
  attempt(id, async () => {
    await invoke('plugin_run', { args: ['install', id, '--scope', 'user'], dir: null })
    plugins.flags = after(plugins.flags, [{ args: ['enable', id, '--scope', 'user'] }])
    await steps(id, target)
  })

export const uninstall = (plugin: Installed) =>
  attempt(plugin.id, () => invoke('plugin_run', { args: ['uninstall', plugin.id, '--scope', plugin.scope], dir: plugin.projectPath ?? null }))

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
