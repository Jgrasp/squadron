import type { Snippet } from 'svelte'
import en from './en'
import fr from './fr'
import { backend } from './backend'

export type Plural = { zero?: string; one: string; other: string }

type Tree = typeof en

export type Catalog<T = Tree> = { [N in keyof T]: { [K in keyof T[N]]: T[N][K] extends string ? string : Plural } }

export type Key = { [N in keyof Tree & string]: `${N}.${keyof Tree[N] & string}` }[keyof Tree & string]

export type Locale = 'en' | 'fr'

export type Language = Locale | 'system'

export type Params = Record<string, string | number>

export const LANGUAGES: [Locale, string][] = [
  ['en', 'English'],
  ['fr', 'Français'],
]

const catalogs: Record<Locale, Catalog> = { en, fr }

const TAGS: Record<Locale, string> = { en: 'en-GB', fr: 'fr-FR' }

const supported = (code: string) => LANGUAGES.find(([id]) => code.toLowerCase().startsWith(id))?.[0]

export const systemLocale = (): Locale =>
  (typeof navigator === 'undefined' ? [] : navigator.languages ?? [navigator.language]).map(supported).find(Boolean) ?? 'en'

const state = $state<{ language: Language }>({ language: 'system' })

export const locale = (): Locale => (state.language === 'system' ? systemLocale() : state.language)

export const tag = () => TAGS[locale()]

export function setLanguage(language: Language | undefined) {
  state.language = language && (language === 'system' || language in catalogs) ? language : 'system'
  if (typeof document !== 'undefined') document.documentElement.lang = locale()
}

function message(loc: Locale, key: Key, count: unknown) {
  const at = key.indexOf('.')
  const ns = key.slice(0, at) as keyof Tree
  const name = key.slice(at + 1)
  const value = (catalogs[loc][ns] as Record<string, string | Plural>)?.[name] ?? (catalogs.en[ns] as Record<string, string | Plural>)[name]
  if (typeof value === 'string' || value === undefined) return value ?? key
  const n = typeof count === 'number' ? count : 0
  if (n === 0 && value.zero !== undefined) return value.zero
  return new Intl.PluralRules(TAGS[loc]).select(n) === 'one' ? value.one : value.other
}

const show = (loc: Locale, value: unknown) => (typeof value === 'number' ? new Intl.NumberFormat(TAGS[loc]).format(value) : String(value))

function translate(loc: Locale, key: Key, params?: Params) {
  const text = message(loc, key, params?.count)
  return params ? text.replace(/\{(\w+)\}/g, (all, name) => (name in params ? show(loc, params[name]) : all)) : text
}

export const t = (key: Key, params?: Params) => translate(locale(), key, params)

export const english = (key: Key, params?: Params) => translate('en', key, params)

export type Part = string | Snippet

export function parts(key: Key, values: Record<string, string | number | Snippet>): Part[] {
  const loc = locale()
  return message(loc, key, values.count)
    .split(/(\{\w+\})/)
    .filter(Boolean)
    .map((piece) => {
      const name = /^\{(\w+)\}$/.exec(piece)?.[1]
      if (!name || !(name in values)) return piece
      const value = values[name]
      return typeof value === 'function' ? value : show(loc, value)
    })
}

export function errorText(error: unknown) {
  const text = String(error)
  const table = backend[locale()]
  if (!table) return text
  const bare = text.replace(/^Error: /, '')
  if (table.exact[bare]) return table.exact[bare]
  for (const [pattern, out] of table.patterns) if (pattern.test(bare)) return bare.replace(pattern, out)
  return text
}

export const number = (n: number, options?: Intl.NumberFormatOptions) => new Intl.NumberFormat(tag(), options).format(n)

export const date = (at: number | Date, options?: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(tag(), options).format(at)

export const list = (items: string[], type: Intl.ListFormatType = 'conjunction') => new Intl.ListFormat(tag(), { type }).format(items)
