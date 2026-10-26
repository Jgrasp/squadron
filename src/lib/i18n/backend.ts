import type { Locale } from './index.svelte'

type Table = { exact: Record<string, string>; patterns: [RegExp, string][] }

export const backend: Partial<Record<Locale, Table>> = {
  fr: {
    exact: {},
    patterns: [],
  },
}
