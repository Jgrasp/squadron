import { invoke } from '@tauri-apps/api/core'
import { settingsOf, type Setting } from './commands'
import { t, type Key } from './i18n/index.svelte'

export const TERMINAL = new Set([
  'autoConnectIde',
  'autoScroll',
  'copyFullResponse',
  'copyOnSelect',
  'defaultToAgentsView',
  'editor',
  'externalEditorContext',
  'gitignore',
  'leftArrowOpensAgents',
  'notifChannel',
  'prStatus',
  'progressBar',
  'reduceMotion',
  'theme',
  'timeFormat',
  'tips',
  'turnDuration',
  'verbose',
])

export const claudeConfig = $state({
  settings: null as Setting[] | null,
  loading: false,
  failed: '',
  query: '',
  chosen: {} as Record<string, string>,
})

export function labelOf(key: string) {
  const id = `claudeKeys.${key}` as Key
  const label = t(id)
  return label === id ? '' : label
}

export async function loadSettings() {
  if (claudeConfig.settings || claudeConfig.loading) return
  claudeConfig.loading = true
  claudeConfig.failed = ''
  try {
    claudeConfig.settings = settingsOf(await invoke<string>('claude_config', { setting: '' }))
  } catch (e) {
    claudeConfig.failed = String(e)
  } finally {
    claudeConfig.loading = false
  }
}

export async function applySetting(key: string, value: string) {
  const reply = await invoke<string>('claude_config', { setting: `${key}=${value}` })
  claudeConfig.chosen[key] = value
  return reply
}
