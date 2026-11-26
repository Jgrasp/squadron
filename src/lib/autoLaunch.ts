import { invoke } from '@tauri-apps/api/core'
import prompt from './autoPrompt.md?raw'
import { models } from './catalogs.svelte'
import { familyOf, type Model } from './commands'
import { settings } from './settings.svelte'
import type { Options } from './stream'

export const AUTO = 'auto'

export const AUTO_PROMPT = prompt.trim()

type Pick = { model: string; effort: string }

const FALLBACK: Pick = { model: 'opus', effort: 'high' }

const ADVISORS: Record<string, string> = { haiku: 'sonnet', sonnet: 'opus' }

export const isAuto = (options: Options) => options.model === AUTO || options.effort === AUTO || options.advisor === AUTO

export function settle(options: Options, pick: Pick | null, catalog: Model[]): Options {
  const chosen = pick ?? FALLBACK
  const model = options.model === AUTO ? chosen.model : options.model
  let effort = (options.effort === AUTO ? chosen.effort : options.effort) as Options['effort']
  const levels = catalog.find((m) => m.value === model)?.supportedEffortLevels
  if (levels?.length && !levels.includes(effort)) effort = (levels.includes(FALLBACK.effort) ? FALLBACK.effort : levels[levels.length - 1]) as Options['effort']
  const advisor = options.advisor === AUTO ? (ADVISORS[familyOf(model, catalog)] ?? 'none') : options.advisor
  return { ...options, model, effort, ...(advisor ? { advisor } : {}) }
}

export async function resolveAuto(options: Options, text: string): Promise<Options> {
  if (!isAuto(options)) return options
  const pick = text.trim() ? await invoke<Pick>('suggest_launch', { text, instruction: settings.auto_prompt || AUTO_PROMPT }).catch(() => null) : null
  return settle(options, pick, models())
}
