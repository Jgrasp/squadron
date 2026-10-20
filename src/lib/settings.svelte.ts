import { invoke } from '@tauri-apps/api/core'
import type { Options } from './stream'

export type Config = { claude: string; defaults: Options }

export const settings = $state<Config>({ claude: 'claude', defaults: { model: 'opus', effort: 'high', mode: 'default' } })

export async function loadSettings() {
  Object.assign(settings, await invoke<Config>('get_settings'))
}

export async function saveSettings(config: Config) {
  await invoke('set_settings', { config })
  Object.assign(settings, config)
}
