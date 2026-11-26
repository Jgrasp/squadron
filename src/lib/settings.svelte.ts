import { invoke } from '@tauri-apps/api/core'
import { emitTo } from '@tauri-apps/api/event'
import { setLanguage, type Language } from './i18n/index.svelte'
import { label, pressed, binding, type Action } from './shortcuts'
import type { Follow } from './plugins'
import type { Options } from './stream'

export const COMPACT_WINDOWS = [0, 600_000, 400_000, 200_000]

export const COLD_TOKENS = [20_000, 50_000, 100_000, 200_000, 0]

export type TabEffect = 'pulse' | 'snake'

export type ProjectsEffect = 'snake' | 'bar' | 'pulse'

export type ChipsMotion = 'off' | 'fade' | 'down' | 'up'

export type TabRing = 'off' | 'shake' | 'sound'

export type TabOrder = 'manual' | 'name' | 'activity'

export type App = { name: string; command: string }

export type NotifyWhen = 'background' | 'always' | 'never'

export type NotifyRuns = 'all' | 'failures' | 'never'

export type NotifyRemote = 'keep' | 'skip' | 'background'

export type Config = {
  claude: string
  defaults: Options
  tab_effect: TabEffect
  projects_effect: ProjectsEffect
  tab_ring: TabRing
  tab_order: TabOrder
  tab_hours: number
  keys: Record<string, string>
  editors: App[]
  pilot: boolean
  pilot_all: boolean
  browser: boolean
  archive_after_days: number
  artifacts_after_days: number
  artifacts_since: number
  artifacts_extensions: Record<string, number>
  compact_window: number
  cold_tokens: number
  steps: 'grouped' | 'detailed'
  thinking: 'hidden' | 'duration' | 'summary' | 'shown'
  language: Language
  runs: boolean
  runs_linger: number
  runs_hidden: string[]
  review_prompt: string
  auto_prompt: string
  conversation_width: number
  sidebar: 'pinned' | 'peek'
  plugin_lines: 'off' | 'tag' | 'card' | 'gutter'
  plugin_card: 'normal' | 'minimal'
  plugin_projects: Record<string, string[]>
  plugin_groups: Record<string, Follow>
  notify_when: NotifyWhen
  notify_off: string[]
  notify_sound: string[]
  notify_turn_secs: number
  notify_runs: NotifyRuns
  notify_remote: NotifyRemote
  remote_new_agents: boolean
  merge_others: 'second' | 'menu' | 'hidden'
  merge_review: 'banner' | 'header'
  merge_method: 'repo' | MergeMethod
  pulls_others: boolean
  auto_continue: boolean
  chips_motion: ChipsMotion
  chips_every: 2 | 3 | 5
  chips_scope: 'all' | 'selected'
  ago_style: 'days' | 'weeks' | 'date'
  usage: boolean
  tour_offered: boolean
}

export type MergeMethod = 'merge' | 'squash' | 'rebase'

export const settings = $state<Config>({
  claude: 'claude',
  defaults: { model: 'opus', effort: 'high', mode: 'default' },
  tab_effect: 'pulse',
  projects_effect: 'snake',
  tab_ring: 'shake',
  tab_order: 'manual',
  tab_hours: 4,
  keys: {},
  editors: [],
  pilot: true,
  pilot_all: false,
  browser: true,
  archive_after_days: 0,
  artifacts_after_days: 30,
  artifacts_since: 0,
  artifacts_extensions: {},
  compact_window: 0,
  cold_tokens: 50_000,
  steps: 'grouped',
  thinking: 'duration',
  language: 'system',
  runs: true,
  runs_linger: 10,
  runs_hidden: [],
  review_prompt: '',
  auto_prompt: '',
  conversation_width: 740,
  sidebar: 'pinned',
  plugin_lines: 'tag',
  plugin_card: 'normal',
  plugin_projects: {},
  plugin_groups: {},
  notify_when: 'background',
  notify_off: [],
  notify_sound: ['question', 'permission', 'plan', 'password'],
  notify_turn_secs: 60,
  notify_runs: 'failures',
  notify_remote: 'keep',
  remote_new_agents: false,
  merge_others: 'menu',
  merge_review: 'banner',
  merge_method: 'repo',
  pulls_others: false,
  auto_continue: false,
  chips_motion: 'down',
  chips_every: 3,
  chips_scope: 'all',
  ago_style: 'days',
  usage: true,
  tour_offered: false,
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

export const patchSettings = (patch: Partial<Config>) => saveSettings({ ...$state.snapshot(settings), ...patch })
