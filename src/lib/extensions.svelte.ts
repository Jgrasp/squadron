import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { run, type Context } from './actions.svelte'
import { agents, archivable, archive, find, shownAgent } from './agents.svelte'
import { pointed } from './artifacts.svelte'
import { ask, message } from './asking.svelte'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { errorText, locale, t } from './i18n/index.svelte'
import { layout, setPanel } from './layout.svelte'
import { AGENTS, argsOf, ARGS, catalogOf, EXEC, type Source, grantsOf, itemsOf, spotlightOf, load, NET, panelsOf, text, themeOf, toolsOf, updatesOf, type CatalogEntry, type Found, type Item, type Extension, type Panel } from './extensions'
import type { Slot } from './slots'
import { workdir, type Agent } from './stream'
import { type as typeInTerminal } from './terminals.svelte'

export const extensions = $state({ list: [] as Extension[], sources: [] as string[], origins: {} as Record<string, string>, errors: {} as Record<string, string>, shown: {} as Record<string, { badge?: string | null; label?: string | null }> })

export const shownLabel = (key: string) => extensions.shown[key]?.label

export const badgeOf = (key: string) => extensions.shown[key]?.badge

const info = (agent: Agent) => ({ id: agent.id, name: agent.name, project: agent.project, dir: workdir(agent) })

const stateOf = (agent: Agent) => ({
  ...info(agent),
  status: agent.status,
  since: Math.round(agent.since),
  pinned: !!agent.pinned,
  remote: !!agent.remote,
  shown: agent.id === shownAgent(),
  branch: agent.worktree?.branch ?? null,
  archivable: archivable(agent),
})

export const commandLine = (key: string) =>
  key.startsWith(EXEC)
    ? t('extensions.anyArgs', { program: key.slice(EXEC.length) })
    : key.startsWith(NET)
      ? t('extensions.network', { host: key.slice(NET.length) })
      : key.startsWith(ARGS)
        ? t('extensions.claudeArgs', { args: (JSON.parse(key.slice(ARGS.length)) as string[]).join(' ') })
        : key === `${AGENTS}read`
          ? t('extensions.agentsRead')
          : key === `${AGENTS}archive`
            ? t('extensions.agentsArchive')
            : key

export async function loadExtensions() {
  const listing = await invoke<{ extensions: Found[]; disabled: string[]; approved: Record<string, string[]>; sources: string[]; origins: Record<string, string> }>('extensions_list')
  extensions.list = load(listing.extensions, listing.disabled, listing.approved)
  extensions.sources = listing.sources
  extensions.origins = listing.origins
  for (const [project, kind] of Object.entries(layout.panels)) if (kind.startsWith('extension:') && !panelsOf(extensions.list).some((p) => p.key === kind)) setPanel(project, null)
  for (const extension of extensions.list) {
    const key = `${extension.id}\n${extension.pending.join('\n')}`
    if (!extension.enabled || !extension.manifest || !extension.pending.length || asked.has(key)) continue
    asked.add(key)
    requestApproval(extension)
  }
}

const asked = new Set<string>()

export async function requestApproval(extension: Extension) {
  if (!extension.manifest) return false
  const name = text(extension.manifest.name, locale())
  const yes = await ask(t('extensions.askText', { name }), {
    title: t('extensions.askTitle', { name }),
    kind: 'warning',
    okLabel: t('extensions.allow'),
    cancelLabel: t('extensions.later'),
    lines: extension.pending.map(commandLine),
  })
  if (yes) await approve(extension)
  return yes
}

listen('extensions-changed', () => loadExtensions())

listen<{ id: string; agent: string; tool: string; args: unknown }>('extensions-tool', async ({ payload: { id, agent, tool, args } }) => {
  const reply = (ok: boolean, text: string) => invoke('extensions_tool_reply', { id, ok, text })
  const found = toolsOf(extensions.list).find((t) => t.name === tool)
  const caller = find(agent)
  if (!found || !caller) return reply(false, found ? 'Only agents started by Squadron can use extension tools.' : `The tool ${tool} is no longer available.`)
  try {
    if (!found.command) return reply(true, await invoke<string>('extensions_wasm_tool', { id: found.extension, name: found.tool, input: JSON.stringify(args ?? {}), agent: info(caller) }))
    const secrets = (extensions.list.find((m) => m.id === found.extension)?.manifest?.contributes?.secrets ?? []).map((s) => s.id)
    const [ok, text] = await invoke<[boolean, string]>('extension_exec', { dir: workdir(caller), command: found.command, id: found.extension, secrets })
    reply(ok, text || '(no output)')
  } catch (e) {
    reply(false, String(e))
  }
})

export const extensionItems = (slot: Slot) => itemsOf(extensions.list, slot)

export const spotlightItems = () => spotlightOf(extensions.list)

export const extensionItem = (slot: Slot, key: string) => extensionItems(slot).find((i) => i.key === key)

export async function runItem(item: Item, { project, agent }: Context) {
  const action = item.action
  if ('squadron' in action) return run(action.squadron, { project, agent })
  if ('url.open' in action) return invoke('open_link', { url: action['url.open'] })
  if ('panel.open' in action) {
    if (!agent) return
    const kind = `extension:${item.extension}.${action['panel.open']}` as const
    return setPanel(agent.project, layout.panels[agent.project] === kind ? null : kind)
  }
  if ('run' in action)
    return invoke('extensions_wasm_run', { id: item.extension, action: action.run, agent: agent && info(agent) }).catch((e) => message(errorText(e), { kind: 'error', title: item.extension }))
  if (!agent) return
  if ('prompt.insert' in action) pointed[agent.id] = [...(pointed[agent.id] ?? []), action['prompt.insert']]
  else {
    const extension = extensions.list.find((m) => m.id === item.extension)
    if (!item.allowed && !(extension && (await requestApproval(extension)))) return
    typeInTerminal(workdir(agent), `${action['terminal.run']}\r`)
    setPanel(agent.project, 'terminal')
  }
}

const posts = new Set<(p: { extension: string; panel: string; message: string }) => void>()

export function onPanelPost(extension: string, panel: string, deliver: (message: unknown) => void) {
  const listener = (p: { extension: string; panel: string; message: string }) => {
    if (p.extension !== extension || p.panel !== panel) return
    try {
      deliver(JSON.parse(p.message))
    } catch {
      deliver(p.message)
    }
  }
  posts.add(listener)
  return () => posts.delete(listener)
}

export const panelMessage = (panel: Panel, agent: Agent, message: unknown) =>
  invoke('extensions_wasm_event', { id: panel.extension, name: 'panel.message', agent: info(agent), data: JSON.stringify({ panel: panel.id, message: message ?? null }) })

export const extensionPanel = (kind: string | null) => panelsOf(extensions.list).find((p) => p.key === kind)

export const catalog = $state({ entries: null as CatalogEntry[] | null, sources: [] as Source[], error: '' })

export async function loadCatalog() {
  try {
    Object.assign(catalog, catalogOf(await invoke('extensions_catalog')), { error: '' })
  } catch (e) {
    catalog.error = errorText(e)
  }
}

export const install = (entry: CatalogEntry) =>
  invoke('extensions_install', { source: entry.source, id: entry.id, archive: entry.archive, sha256: entry.sha256 }).then(loadExtensions)

export const addSource = (source: string) => invoke('extensions_source_add', { source }).then(loadExtensions).then(loadCatalog)

export const removeSource = (source: string) => invoke('extensions_source_remove', { source }).then(loadExtensions).then(loadCatalog)

export async function uninstall(extension: Extension) {
  for (const secret of extension.manifest?.contributes?.secrets ?? []) await invoke('extensions_secret_set', { id: extension.id, name: secret.id, value: null })
  await invoke('extensions_uninstall', { id: extension.id })
  await loadExtensions()
}

export const asks = (entry: CatalogEntry) => [
  ...(entry.permissions?.exec ?? []).map((p) => commandLine(EXEC + p)),
  ...(entry.permissions?.network ?? []).map((h) => commandLine(NET + h)),
  ...(entry.permissions?.agents ?? []).map((a) => commandLine(AGENTS + a)),
]

const TOLD = 'squadron.extension-updates'

export async function checkUpdates() {
  if (getCurrentWindow().label !== 'main') return
  await loadCatalog()
  const told: string[] = JSON.parse(localStorage.getItem(TOLD) ?? '[]')
  const fresh = updatesOf(extensions.list, catalog.entries ?? [], extensions.origins).filter((c) => !told.includes(`${c.id}@${c.version}`))
  if (!fresh.length) return
  localStorage.setItem(TOLD, JSON.stringify([...told, ...fresh.map((c) => `${c.id}@${c.version}`)]))
  if ((await isPermissionGranted()) || (await requestPermission()) === 'granted')
    for (const c of fresh) sendNotification({ title: t('extensions.updateTitle'), body: t('extensions.updateBody', { name: text(c.name, locale()), version: c.version }) })
}

export const approve = (extension: Extension) => invoke('extensions_approve', { id: extension.id, commands: extension.commands }).then(loadExtensions)

listen<{ extension: string; call: string; [key: string]: unknown }>('extensions-host', async ({ payload: p }) => {
  if (p.call === 'error') extensions.errors[p.extension] = String(p.error)
  if (p.call === 'panel-post') for (const post of posts) post({ extension: p.extension, panel: String(p.panel), message: String(p.message) })
  if (p.call === 'set-badge' || p.call === 'set-label') {
    const key = `${p.extension}.${p.item}`
    extensions.shown[key] = { ...extensions.shown[key], [p.call === 'set-badge' ? 'badge' : 'label']: (p.value as string | null) ?? null }
  }
  if ((p.call === 'agents' || p.call === 'archive-agent') && getCurrentWindow().label === 'main') answerHost(String(p.reply), p.call, String(p.agent))
  if (p.call === 'insert-prompt' && find(String(p.agent))) pointed[String(p.agent)] = [...(pointed[String(p.agent)] ?? []), String(p.text)]
  if (p.call === 'notify' && ((await isPermissionGranted()) || (await requestPermission()) === 'granted')) sendNotification({ title: String(p.title), body: String(p.body) })
})

async function answerHost(reply: string, call: string, id: string) {
  const answer = (value: unknown, error: string | null = null) => invoke('extensions_wasm_answer', { reply, value, error })
  if (call === 'agents') return answer(agents.list.map(stateOf))
  const agent = find(id)
  if (!agent) return answer(null, `No open agent has the id ${id}.`)
  if (!archivable(agent)) return answer(null, `${agent.name} stays: it works, waits for the user, is pinned, remote or shown.`)
  try {
    await archive(agent.id)
    answer(null)
  } catch (e) {
    answer(null, errorText(e))
  }
}

const seen = new Map<string, string>()

let themed: string[] = []

$effect.root(() => {
  $effect(() => {
    const theme = themeOf(extensions.list)
    const style = document.documentElement.style
    for (const name of themed) style.removeProperty(`--${name}`)
    themed = Object.keys(theme)
    for (const [name, value] of Object.entries(theme)) style.setProperty(`--${name}`, value)
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    invoke('extensions_set_tools', { list: toolsOf(extensions.list).map(({ name, description, input }) => ({ name, description, input })) })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    invoke('extensions_set_args', { args: argsOf(extensions.list) })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    const list = extensions.list.filter((m) => m.enabled && m.manifest?.wasm).map((m) => ({ id: m.id, dir: m.dir, wasm: m.manifest!.wasm, allowed: grantsOf(m) }))
    invoke<Record<string, string>>('extensions_wasm_sync', { list }).then((errors) => {
      extensions.errors = errors
    })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    for (const agent of agents.list) {
      const before = seen.get(agent.id)
      seen.set(agent.id, agent.status)
      if (before === undefined || before === agent.status) continue
      if (agent.status === 'done' || agent.status === 'waiting')
        invoke('extensions_wasm_event', { id: null, name: `agent.${agent.status}`, agent: info(agent), data: JSON.stringify(agent.status === 'waiting' ? { waiting: agent.action } : {}) })
    }
  })
})
