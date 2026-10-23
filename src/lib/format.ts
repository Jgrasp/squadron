import { models } from './catalogs.svelte'
import { modelName } from './commands'
import type { Agent, Options } from './stream'

const number = new Intl.NumberFormat('en-GB', { maximumFractionDigits: 1 })

export const cost = (usd: number) => `$${usd.toFixed(usd > 0 && usd < 0.01 ? 3 : 2)}`

export const tokens = (n: number) => (n < 1000 ? `${n}` : n < 1e6 ? `${number.format(n / 1000)}k` : `${number.format(n / 1e6)} M`)

export function duration(ms: number) {
  const s = Math.round(ms / 1000)
  if (s < 60) return `${s} s`
  return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, '0')} s`
}

export function ago(at: number, now: number) {
  const min = Math.floor((now - at) / 60_000)
  if (min < 1) return 'just now'
  if (min < 60) return `${min} min ago`
  return `${Math.floor(min / 60)} h ago`
}

export const time = (at: number) =>
  new Date(at).toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })

export const modelLabel = (model: string) => modelName(model, models())

export const modeLabel: Record<Options['mode'], string> = {
  default: 'ask for everything',
  acceptEdits: 'accept edits',
  plan: 'plan',
  auto: 'auto',
}

export const meta = (agent: Agent) =>
  `${modelLabel(agent.options.model)} · effort ${agent.options.effort} · ${modeLabel[agent.options.mode]}`

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
