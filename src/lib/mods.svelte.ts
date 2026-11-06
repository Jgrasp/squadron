import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { run, type Context } from './actions.svelte'
import { pointed } from './artifacts.svelte'
import { setPanel } from './layout.svelte'
import { itemsOf, load, themeOf, type Found, type Item, type Mod } from './mods'
import type { Slot } from './slots'
import { workdir } from './stream'
import { type as typeInTerminal } from './terminals.svelte'

export const mods = $state({ list: [] as Mod[] })

export async function loadMods() {
  const { mods: found, disabled } = await invoke<{ mods: Found[]; disabled: string[] }>('mods_list')
  mods.list = load(found, disabled)
}

listen('mods-changed', () => loadMods())

export const modItems = (slot: Slot) => itemsOf(mods.list, slot)

export const modItem = (slot: Slot, key: string) => modItems(slot).find((i) => i.key === key)

export function runItem(item: Item, { project, agent }: Context) {
  const action = item.action
  if ('squadron' in action) return run(action.squadron, { project, agent })
  if ('url.open' in action) return invoke('open_link', { url: action['url.open'] })
  if (!agent) return
  if ('prompt.insert' in action) pointed[agent.id] = [...(pointed[agent.id] ?? []), action['prompt.insert']]
  else {
    typeInTerminal(workdir(agent), action['terminal.run'])
    setPanel(agent.project, 'terminal')
  }
}

let themed: string[] = []

$effect.root(() => {
  $effect(() => {
    const theme = themeOf(mods.list)
    const style = document.documentElement.style
    for (const name of themed) style.removeProperty(`--${name}`)
    themed = Object.keys(theme)
    for (const [name, value] of Object.entries(theme)) style.setProperty(`--${name}`, value)
  })
})
