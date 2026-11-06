import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { run, type Context } from './actions.svelte'
import { find } from './agents.svelte'
import { pointed } from './artifacts.svelte'
import { setPanel } from './layout.svelte'
import { itemsOf, load, themeOf, toolsOf, type Found, type Item, type Mod } from './mods'
import type { Slot } from './slots'
import { workdir } from './stream'
import { type as typeInTerminal } from './terminals.svelte'

export const mods = $state({ list: [] as Mod[] })

export async function loadMods() {
  const { mods: found, disabled, approved } = await invoke<{ mods: Found[]; disabled: string[]; approved: Record<string, string[]> }>('mods_list')
  mods.list = load(found, disabled, approved)
}

listen('mods-changed', () => loadMods())

listen<{ id: string; agent: string; tool: string }>('mods-tool', async ({ payload: { id, agent, tool } }) => {
  const reply = (ok: boolean, text: string) => invoke('mods_tool_reply', { id, ok, text })
  const found = toolsOf(mods.list).find((t) => t.name === tool)
  const caller = find(agent)
  if (!found || !caller) return reply(false, found ? 'Only agents started by Squadron can use mod tools.' : `The tool ${tool} is no longer available.`)
  try {
    const [ok, text] = await invoke<[boolean, string]>('mod_exec', { dir: workdir(caller), command: found.command })
    reply(ok, text || '(no output)')
  } catch (e) {
    reply(false, String(e))
  }
})

export const modItems = (slot: Slot) => itemsOf(mods.list, slot)

export const modItem = (slot: Slot, key: string) => modItems(slot).find((i) => i.key === key)

export function runItem(item: Item, { project, agent }: Context) {
  const action = item.action
  if ('squadron' in action) return run(action.squadron, { project, agent })
  if ('url.open' in action) return invoke('open_link', { url: action['url.open'] })
  if (!agent) return
  if ('prompt.insert' in action) pointed[agent.id] = [...(pointed[agent.id] ?? []), action['prompt.insert']]
  else {
    typeInTerminal(workdir(agent), item.allowed ? `${action['terminal.run']}\r` : action['terminal.run'])
    setPanel(agent.project, 'terminal')
  }
}

export const approve = (mod: Mod) => invoke('mods_approve', { id: mod.id, commands: mod.commands }).then(loadMods)

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
    invoke('mods_set_tools', { list: toolsOf(mods.list).map(({ name, description }) => ({ name, description })) })
  })
})
