import { invoke } from '@tauri-apps/api/core'
import type { Options } from './stream'

export type TabEffect = 'pulse' | 'snake'

export type Config = { claude: string; defaults: Options; tab_effect: TabEffect }

export const settings = $state<Config>({
  claude: 'claude',
  defaults: { model: 'opus', effort: 'high', mode: 'default' },
  tab_effect: 'pulse',
})

export async function loadSettings() {
  Object.assign(settings, await invoke<Config>('get_settings'))
}

export async function saveSettings(config: Config) {
  await invoke('set_settings', { config })
  Object.assign(settings, config)
}
