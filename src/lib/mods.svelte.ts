import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { run, type Context } from './actions.svelte'
import { agents, find } from './agents.svelte'
import { pointed } from './artifacts.svelte'
import { ask, message } from './asking.svelte'
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { errorText, locale, t } from './i18n/index.svelte'
import { layout, setPanel } from './layout.svelte'
import { EXEC, grantsOf, itemsOf, load, NET, panelsOf, text, themeOf, toolsOf, type Found, type Item, type Mod, type Panel } from './mods'
import type { Slot } from './slots'
import { workdir, type Agent } from './stream'
import { type as typeInTerminal } from './terminals.svelte'

export const mods = $state({ list: [] as Mod[], errors: {} as Record<string, string>, shown: {} as Record<string, { badge?: string | null; label?: string | null }> })

export const shownLabel = (key: string) => mods.shown[key]?.label

export const badgeOf = (key: string) => mods.shown[key]?.badge

const info = (agent: Agent) => ({ id: agent.id, name: agent.name, project: agent.project, dir: workdir(agent) })

export const commandLine = (key: string) =>
  key.startsWith(EXEC) ? t('mods.anyArgs', { program: key.slice(EXEC.length) }) : key.startsWith(NET) ? t('mods.network', { host: key.slice(NET.length) }) : key

export async function loadMods() {
  const { mods: found, disabled, approved } = await invoke<{ mods: Found[]; disabled: string[]; approved: Record<string, string[]> }>('mods_list')
  mods.list = load(found, disabled, approved)
  for (const [project, kind] of Object.entries(layout.panels)) if (kind.startsWith('mod:') && !panelsOf(mods.list).some((p) => p.key === kind)) setPanel(project, null)
  for (const mod of mods.list) {
    const key = `${mod.id}\n${mod.pending.join('\n')}`
    if (!mod.enabled || !mod.manifest || !mod.pending.length || asked.has(key)) continue
    asked.add(key)
    requestApproval(mod)
  }
}

const asked = new Set<string>()

export async function requestApproval(mod: Mod) {
  if (!mod.manifest) return false
  const name = text(mod.manifest.name, locale())
  const yes = await ask(t('mods.askText', { name }), {
    title: t('mods.askTitle', { name }),
    kind: 'warning',
    okLabel: t('mods.allow'),
    cancelLabel: t('mods.later'),
    lines: mod.pending.map(commandLine),
  })
  if (yes) await approve(mod)
  return yes
}

listen('mods-changed', () => loadMods())

listen<{ id: string; agent: string; tool: string; args: unknown }>('mods-tool', async ({ payload: { id, agent, tool, args } }) => {
  const reply = (ok: boolean, text: string) => invoke('mods_tool_reply', { id, ok, text })
  const found = toolsOf(mods.list).find((t) => t.name === tool)
  const caller = find(agent)
  if (!found || !caller) return reply(false, found ? 'Only agents started by Squadron can use mod tools.' : `The tool ${tool} is no longer available.`)
  try {
    if (!found.command) return reply(true, await invoke<string>('mods_wasm_tool', { id: found.mod, name: found.tool, input: JSON.stringify(args ?? {}), agent: info(caller) }))
    const secrets = (mods.list.find((m) => m.id === found.mod)?.manifest?.contributes?.secrets ?? []).map((s) => s.id)
    const [ok, text] = await invoke<[boolean, string]>('mod_exec', { dir: workdir(caller), command: found.command, id: found.mod, secrets })
    reply(ok, text || '(no output)')
  } catch (e) {
    reply(false, String(e))
  }
})

export const modItems = (slot: Slot) => itemsOf(mods.list, slot)

export const modItem = (slot: Slot, key: string) => modItems(slot).find((i) => i.key === key)

export async function runItem(item: Item, { project, agent }: Context) {
  const action = item.action
  if ('squadron' in action) return run(action.squadron, { project, agent })
  if ('url.open' in action) return invoke('open_link', { url: action['url.open'] })
  if ('panel.open' in action) {
    if (!agent) return
    const kind = `mod:${item.mod}.${action['panel.open']}` as const
    return setPanel(agent.project, layout.panels[agent.project] === kind ? null : kind)
  }
  if ('mod' in action)
    return invoke('mods_wasm_run', { id: item.mod, action: action.mod, agent: agent && info(agent) }).catch((e) => message(errorText(e), { kind: 'error', title: item.mod }))
  if (!agent) return
  if ('prompt.insert' in action) pointed[agent.id] = [...(pointed[agent.id] ?? []), action['prompt.insert']]
  else {
    const mod = mods.list.find((m) => m.id === item.mod)
    if (!item.allowed && !(mod && (await requestApproval(mod)))) return
    typeInTerminal(workdir(agent), `${action['terminal.run']}\r`)
    setPanel(agent.project, 'terminal')
  }
}

const posts = new Set<(p: { mod: string; panel: string; message: string }) => void>()

export function onPanelPost(mod: string, panel: string, deliver: (message: unknown) => void) {
  const listener = (p: { mod: string; panel: string; message: string }) => {
    if (p.mod !== mod || p.panel !== panel) return
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
  invoke('mods_wasm_event', { id: panel.mod, name: 'panel.message', agent: info(agent), data: JSON.stringify({ panel: panel.id, message: message ?? null }) })

export const modPanel = (kind: string | null) => panelsOf(mods.list).find((p) => p.key === kind)

export const approve = (mod: Mod) => invoke('mods_approve', { id: mod.id, commands: mod.commands }).then(loadMods)

listen<{ mod: string; call: string; [key: string]: unknown }>('mods-host', async ({ payload: p }) => {
  if (p.call === 'error') mods.errors[p.mod] = String(p.error)
  if (p.call === 'panel-post') for (const post of posts) post({ mod: p.mod, panel: String(p.panel), message: String(p.message) })
  if (p.call === 'set-badge' || p.call === 'set-label') {
    const key = `${p.mod}.${p.item}`
    mods.shown[key] = { ...mods.shown[key], [p.call === 'set-badge' ? 'badge' : 'label']: (p.value as string | null) ?? null }
  }
  if (p.call === 'insert-prompt' && find(String(p.agent))) pointed[String(p.agent)] = [...(pointed[String(p.agent)] ?? []), String(p.text)]
  if (p.call === 'notify' && ((await isPermissionGranted()) || (await requestPermission()) === 'granted')) sendNotification({ title: String(p.title), body: String(p.body) })
})

const seen = new Map<string, string>()

let themed: string[] = []

$effect.root(() => {
  $effect(() => {
    const theme = themeOf(mods.list)
    const style = document.documentElement.style
    for (const name of themed) style.removeProperty(`--${name}`)
    themed = Object.keys(theme)
    for (const [name, value] of Object.entries(theme)) style.setProperty(`--${name}`, value)
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    invoke('mods_set_tools', { list: toolsOf(mods.list).map(({ name, description, input }) => ({ name, description, input })) })
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    const list = mods.list.filter((m) => m.enabled && m.manifest?.wasm).map((m) => ({ id: m.id, dir: m.dir, wasm: m.manifest!.wasm, allowed: grantsOf(m) }))
    invoke<Record<string, string>>('mods_wasm_sync', { list }).then((errors) => (mods.errors = errors))
  })
  $effect(() => {
    if (getCurrentWindow().label !== 'main') return
    for (const agent of agents.list) {
      const before = seen.get(agent.id)
      seen.set(agent.id, agent.status)
      if (before === undefined || before === agent.status) continue
      if (agent.status === 'done' || agent.status === 'waiting')
        invoke('mods_wasm_event', { id: null, name: `agent.${agent.status}`, agent: info(agent), data: JSON.stringify(agent.status === 'waiting' ? { waiting: agent.action } : {}) })
    }
  })
})
