import { invoke } from '@tauri-apps/api/core'
import { models } from './catalogs.svelte'
import { familyOf, type Model } from './commands'
import { settings } from './settings.svelte'
import type { Options } from './stream'

export const AUTO = 'auto'

export const AUTO_PROMPT = `You pick the Claude model and the reasoning effort for a coding agent from the first message the user gives it. Pick the cheapest pair that will do the job well on the first try: a weak pick that fails costs more than a strong one.

- haiku · low: a tiny, well-located change (typo, rename, wording, a value to change, a question with a short answer).
- sonnet · medium: an ordinary task in a known area (a bug to find and fix, a small feature, a test to write, a review of a short diff).
- sonnet · high: the same with several files or some unknowns.
- opus · high: a design, a refactor or a migration across modules, a hard bug, a plan, anything vague or open-ended.
- opus · xhigh: a large or risky change where mistakes are costly.`

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
