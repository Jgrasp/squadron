import { models } from './catalogs.svelte'
import { modelName } from './commands'
import { date, number, t, type Key } from './i18n/index.svelte'
import type { Agent, Options } from './stream'

export const cost = (usd: number) => {
  const digits = usd > 0 && usd < 0.01 ? 3 : 2
  return number(usd, { style: 'currency', currency: 'USD', currencyDisplay: 'narrowSymbol', minimumFractionDigits: digits, maximumFractionDigits: digits })
}

const tenth = (n: number) => number(n, { maximumFractionDigits: 1 })

export const tokens = (n: number) =>
  n < 1000 ? number(n) : n < 1e6 ? t('format.thousands', { n: tenth(n / 1000) }) : t('format.millions', { n: tenth(n / 1e6) })

export function duration(ms: number) {
  const s = Math.round(ms / 1000)
  if (s < 60) return t('format.seconds', { s })
  return t('format.minutes', { m: Math.floor(s / 60), s: String(s % 60).padStart(2, '0') })
}

export function ago(at: number, now: number) {
  const min = Math.floor((now - at) / 60_000)
  if (min < 1) return t('format.justNow')
  if (min < 60) return t('format.minutesAgo', { count: min })
  return t('format.hoursAgo', { count: Math.floor(min / 60) })
}

export const time = (at: number) => date(at, { hour: '2-digit', minute: '2-digit' })

export const modelLabel = (model: string) => modelName(model, models())

const MODES: Record<Options['mode'], Key> = {
  default: 'format.modeDefault',
  acceptEdits: 'format.modeAcceptEdits',
  plan: 'format.modePlan',
  auto: 'format.modeAuto',
}

export const modeLabel = (mode: Options['mode']) => t(MODES[mode])

const EFFORTS: Record<string, Key> = {
  low: 'settings.effortLow',
  medium: 'settings.effortMedium',
  high: 'settings.effortHigh',
  xhigh: 'settings.effortXhigh',
  max: 'settings.effortMax',
}

export const effortLabel = (effort: string) => (EFFORTS[effort] ? t(EFFORTS[effort]) : effort)

export const meta = (agent: Agent) =>
  t('format.meta', { model: modelLabel(agent.options.model), effort: effortLabel(agent.options.effort), mode: modeLabel(agent.options.mode) })

export function short(name: string) {
  const words = name.split(/\s+/).filter(Boolean)
  const id = words.find((w) => /\d/.test(w))
  if (id) return (id.includes('-') && /^[A-Za-z]+-/.test(id) ? id.split('-')[0] : id).slice(0, 4)
  return words
    .slice(0, 3)
    .map((w) => w[0])
    .join('')
    .toUpperCase()
}
