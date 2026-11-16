import type { Text } from './extensions'

export type Label = string | Text

export type DraftLink = { key: string; title: string; url?: string }

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
  const link = raw.link
  if (link === null) draft.link = null
  else if (isObject(link) && typeof link.key === 'string' && typeof link.title === 'string')
    draft.link = { key: link.key, title: link.title, ...(typeof link.url === 'string' && /^https:\/\/\S+$/.test(link.url) ? { url: link.url } : {}) }
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
