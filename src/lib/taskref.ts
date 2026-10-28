import type { Card } from './board'

const CODE = /^(?=[A-Z-]*\d)[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*(?![\w-])/
const PREFIX = /^\s*((?=[A-Z-]*\d)[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)\s+[—–:-]\s+/

export const codeOf = (title: string) => PREFIX.exec(title)?.[1]

export const nameOf = (title: string) => title.replace(PREFIX, '')

export const taskId = (href: string) => decodeURIComponent(href.replace(/^squadron:task\//, ''))

export function leads(item: Element, node: Node) {
  for (let at: Node = node; at !== item && at.parentNode; at = at.parentNode)
    for (let before = at.previousSibling; before; before = before.previousSibling) if (before.textContent?.trim()) return false
  return true
}

export function taskAt(cards: Card[], text: string) {
  const code = CODE.exec(text.trimStart())?.[0]
  return code ? cards.find((c) => codeOf(c.title) === code) : undefined
}

export const leading = (code: string) => new RegExp(`^\\s*${code}(?:\\s*[—–:-]\\s+|$)`)

export function mentions(cards: Card[], text: string): (string | { code: string; card: Card })[] {
  const byCode = new Map(cards.flatMap((c) => (codeOf(c.title) ? [[codeOf(c.title)!, c] as const] : [])))
  if (!byCode.size) return [text]
  const parts: (string | { code: string; card: Card })[] = []
  let at = 0
  for (const m of text.matchAll(new RegExp(`(?<![\\w-])(${[...byCode.keys()].join('|')})(?![\\w-])`, 'g'))) {
    if (m.index > at) parts.push(text.slice(at, m.index))
    parts.push({ code: m[1], card: byCode.get(m[1])! })
    at = m.index + m[1].length
  }
  if (at < text.length) parts.push(text.slice(at))
  return parts
}
