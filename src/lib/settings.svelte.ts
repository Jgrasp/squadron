import { invoke } from '@tauri-apps/api/core'
import { emitTo } from '@tauri-apps/api/event'
import { setLanguage, type Language } from './i18n/index.svelte'
import { label, pressed, binding, type Action } from './shortcuts'
import type { Options } from './stream'

export type TabEffect = 'pulse' | 'snake'

export type TabRing = 'off' | 'shake' | 'sound'

export type TabOrder = 'manual' | 'name' | 'activity'

export type App = { name: string; command: string }

export type Config = {
  claude: string
  defaults: Options
  tab_effect: TabEffect
  tab_ring: TabRing
  tab_order: TabOrder
  keys: Record<string, string>
  editors: App[]
  pilot: boolean
  pilot_all: boolean
  archive_after_days: number
  compact_window: number
  steps: 'grouped' | 'detailed'
  thinking: 'hidden' | 'duration' | 'summary' | 'shown'
  language: Language
  runs: boolean
  runs_linger: number
  runs_hidden: string[]
  runs_notify: boolean
}

export const settings = $state<Config>({
  claude: 'claude',
  defaults: { model: 'opus', effort: 'high', mode: 'default' },
  tab_effect: 'pulse',
  tab_ring: 'shake',
  tab_order: 'manual',
  keys: {},
  editors: [],
  pilot: true,
  pilot_all: false,
  archive_after_days: 0,
  compact_window: 0,
  steps: 'grouped',
  thinking: 'duration',
  language: 'system',
  runs: true,
  runs_linger: 10,
  runs_hidden: [],
  runs_notify: false,
})

export const shortcut = (event: KeyboardEvent) => pressed(event, settings.keys)

export const hint = (action: Action) => label(binding(settings.keys, action))

export async function loadSettings() {
  Object.assign(settings, await invoke<Config>('get_settings'))
  setLanguage(settings.language)
}

export async function saveSettings(config: Config) {
  await invoke('set_settings', { config })
  Object.assign(settings, config)
  setLanguage(settings.language)
  emitTo('board', 'settings-changed').catch(() => {})
}
