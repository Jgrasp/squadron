import { readdirSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const read = (path: string) => readFileSync(path, 'utf8')
const design = read('DESIGN.md')
const css = read('src/app.css')
const sources = readdirSync('src/lib', { recursive: true, encoding: 'utf8' })
  .filter((f) => /\.(svelte|ts)$/.test(f) && !f.endsWith('.test.ts') && !f.startsWith('i18n'))
  .map((f) => [f, read(`src/lib/${f}`)] as const)
const root = css.slice(css.indexOf(':root'), css.indexOf('\n}', css.indexOf(':root')))
const tokens = new Set(root.match(/--[a-z0-9-]+(?=:)/g))
const values = new Set(root.match(/#[0-9a-f]{3,6}\b/gi)?.map((h) => h.toLowerCase()))
const hex = /#(?:[0-9a-f]{6}|[0-9a-f]{3})\b/gi

describe('DESIGN.md', () => {
  it('documents every token of app.css', () => {
    expect([...tokens].filter((t) => !new RegExp(`${t}(?![a-z0-9-])`).test(design))).toEqual([])
  })

  it('names only tokens that exist', () => {
    expect([...new Set(design.match(/--[a-z][a-z0-9-]*[a-z0-9](?![a-z0-9*-])/g))].filter((t) => !tokens.has(t) && t !== '--project')).toEqual([])
  })

  it('lists every hard-coded color of the components', () => {
    const known = design.toLowerCase()
    const stray = sources.flatMap(([file, text]) =>
      (text.match(hex) ?? []).map((h) => h.toLowerCase()).filter((h) => !values.has(h) && !known.includes(h)).map((h) => `${file} ${h}`),
    )
    expect(stray).toEqual([])
  })
})
