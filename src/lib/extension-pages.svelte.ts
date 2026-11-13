import { errorText, locale, t } from './i18n/index.svelte'
import { OFFICIAL, text, updatesOf, type Extension, type Text } from './extensions'
import { catalog, commandLine, extensions, loadExtensions } from './extensions.svelte'

export type From = 'official' | 'imported' | 'local'

const COLORS = ['#e08a5b', '#4fa3e0', '#a78bfa', '#7cc47c', '#e3b341', '#6fa0d0', '#3cc39a', '#e07a9b']

export const colorOf = (id: string) => COLORS[[...id].reduce((sum, c) => sum + c.charCodeAt(0), 0) % COLORS.length]

export const named = (value: Text | undefined) => (value ? text(value, locale()) : '')

export const nameOf = (e: Extension) => (e.manifest ? named(e.manifest.name) : e.id)

export const originOf = (e: Extension) => (e.linked ? undefined : extensions.origins[e.id])

export function fromOf(e: Extension): From {
  const origin = originOf(e)
  return !origin ? 'local' : origin === OFFICIAL ? 'official' : 'imported'
}

export const updateOf = (e: Extension) => (e.linked ? undefined : updatesOf([e], catalog.entries ?? [], extensions.origins)[0])

export const waitingOf = (e: Extension) => e.enabled && !!e.manifest && e.pending.length > 0

export const problemOf = (e: Extension) =>
  [...e.problems.map(([key, values]) => t(key, values)), ...(e.enabled && extensions.errors[e.id] ? [extensions.errors[e.id]] : [])].join(' ')

export const pendingOf = (e: Extension) => (waitingOf(e) ? e.pending.map(commandLine) : [])

const rank = (e: Extension) => (waitingOf(e) ? 0 : updateOf(e) ? 1 : 2)

export const sorted = (list: Extension[]) => [...list].sort((a, b) => rank(a) - rank(b) || nameOf(a).localeCompare(nameOf(b), locale()))

export const work = $state({ busy: '', error: '' })

export async function attempt(job: () => Promise<unknown>, key = '') {
  work.error = ''
  work.busy = key
  try {
    await job()
    await loadExtensions()
  } catch (e) {
    work.error = errorText(e)
  }
  work.busy = ''
  return !work.error
}
