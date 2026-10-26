import { invoke } from '@tauri-apps/api/core'
import { label, pressed, binding, type Action } from './shortcuts'
import type { Options } from './stream'

export type TabEffect = 'pulse' | 'snake'

export type TabOrder = 'manual' | 'name' | 'activity'

export type App = { name: string; command: string }

export type Config = {
  claude: string
  defaults: Options
  tab_effect: TabEffect
  tab_order: TabOrder
  keys: Record<string, string>
  editors: App[]
  pilot: boolean
  pilot_all: boolean
  archive_after_days: number
  steps: 'grouped' | 'detailed'
  thinking: 'hidden' | 'duration' | 'summary' | 'shown'
}

export const settings = $state<Config>({
  claude: 'claude',
  defaults: { model: 'opus', effort: 'high', mode: 'default' },
  tab_effect: 'pulse',
  tab_order: 'manual',
  keys: {},
  editors: [],
  pilot: true,
  pilot_all: false,
  archive_after_days: 0,
  steps: 'grouped',
  thinking: 'duration',
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
