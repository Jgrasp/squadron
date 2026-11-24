import type { Text } from './extensions'

export type Label = string | Text

export type Tone = 'ok' | 'err' | 'info'

export type DraftLink = { key: string; title: string; url?: string; tone?: Tone; logo?: false }

export type Chip = DraftLink & { extension: string }

export const CHIPS_MAX = 8

export type DraftChoice = { id: string; label: Label; on: boolean }

export type DraftStatus = { state: 'running' | 'done' | 'error'; text?: Label }

export type Draft = {
  name?: string
  text?: string
  goal?: string
  context?: string
  link?: DraftLink | null
  choices?: { label: Label; items: DraftChoice[] } | null
  status?: DraftStatus | null
}

export const FORM = ['name', 'text', 'goal'] as const

export type Form = Record<(typeof FORM)[number], string>

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)

const isLabel = (v: unknown): v is Label => typeof v === 'string' || (isObject(v) && typeof v.en === 'string' && Object.values(v).every((s) => typeof s === 'string'))

const STATES = ['running', 'done', 'error']

const TONES = ['ok', 'err', 'info']

export function parseLink(link: unknown): DraftLink | null {
  if (!isObject(link) || typeof link.key !== 'string' || !link.key.trim() || typeof link.title !== 'string') return null
  return {
    key: link.key.trim().slice(0, 40),
    title: link.title.slice(0, 200),
    ...(typeof link.url === 'string' && /^https:\/\/\S+$/.test(link.url) ? { url: link.url } : {}),
    ...(TONES.includes(link.tone as string) ? { tone: link.tone as Tone } : {}),
    ...(link.logo === false ? { logo: false as const } : {}),
  }
}

export function parseChips(extension: string, json: string): Chip[] | string {
  let raw: unknown
  try {
    raw = JSON.parse(json)
  } catch {
    return 'chips is not JSON.'
  }
  if (!Array.isArray(raw)) return 'chips is not a list.'
  if (raw.length > CHIPS_MAX) return `${CHIPS_MAX} chips at most.`
  const chips = raw.map(parseLink)
  const bad = chips.findIndex((c) => !c)
  if (bad >= 0) return `Chip ${bad + 1} needs a "key" and a "title".`
  return chips.map((c) => ({ ...c!, extension }))
}

export function parseDraft(json: string): Draft {
  let raw: unknown
  try {
    raw = JSON.parse(json)
  } catch {
    return {}
  }
  if (!isObject(raw)) return {}
  const draft: Draft = {}
  for (const key of [...FORM, 'context'] as const) if (typeof raw[key] === 'string') draft[key] = raw[key] as string
  if (raw.link === null) draft.link = null
  else {
    const link = parseLink(raw.link)
    if (link) draft.link = link
  }
  const choices = raw.choices
  if (choices === null) draft.choices = null
  else if (isObject(choices) && isLabel(choices.label) && Array.isArray(choices.items)) {
    const items = choices.items.filter((c): c is Record<string, unknown> => isObject(c) && typeof c.id === 'string' && isLabel(c.label))
    draft.choices = { label: choices.label, items: items.map((c) => ({ id: c.id as string, label: c.label as Label, on: c.on !== false })) }
  }
  const status = raw.status
  if (status === null) draft.status = null
  else if (isObject(status) && STATES.includes(status.state as string))
    draft.status = { state: status.state as DraftStatus['state'], ...(isLabel(status.text) ? { text: status.text } : {}) }
  return draft
}

export function fill(form: Form, applied: Form, update: Draft): { form: Form; applied: Form } {
  const next = { form: { ...form }, applied: { ...applied } }
  for (const key of FORM) {
    const value = update[key]
    if (value === undefined || form[key] !== applied[key]) continue
    next.form[key] = next.applied[key] = value
  }
  return next
}

export function withChips(list: Chip[], extension: string, next: Chip[]): Chip[] {
  const at = list.findIndex((c) => c.extension === extension)
  const rest = list.filter((c) => c.extension !== extension)
  rest.splice(at < 0 ? rest.length : list.slice(0, at).filter((c) => c.extension !== extension).length, 0, ...next)
  return rest
}
