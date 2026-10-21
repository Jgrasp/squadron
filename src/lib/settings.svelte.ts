import { invoke } from '@tauri-apps/api/core'
import { label, pressed, binding, type Action } from './shortcuts'
import type { Options } from './stream'

export type TabEffect = 'pulse' | 'snake'

export type Config = { claude: string; defaults: Options; tab_effect: TabEffect; keys: Record<string, string> }

export const settings = $state<Config>({
  claude: 'claude',
  defaults: { model: 'opus', effort: 'high', mode: 'default' },
  tab_effect: 'pulse',
  keys: {},
})

export const shortcut = (event: KeyboardEvent) => pressed(event, settings.keys)

export const hint = (action: Action) => label(binding(settings.keys, action))

export async function loadSettings() {
  Object.assign(settings, await invoke<Config>('get_settings'))
}

export async function saveSettings(config: Config) {
  await invoke('set_settings', { config })
  Object.assign(settings, config)
}
