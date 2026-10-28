import type { Card } from './board'

const CODE = /^(?=[A-Z-]*\d)[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*(?![\w-])/
const PREFIX = /^\s*((?=[A-Z-]*\d)[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*)\s+[—–:-]\s+/

export const codeOf = (title: string) => PREFIX.exec(title)?.[1]

export const nameOf = (title: string) => title.replace(PREFIX, '')

export function taskAt(cards: Card[], text: string) {
  const code = CODE.exec(text.trimStart())?.[0]
  return code ? cards.find((c) => codeOf(c.title) === code) : undefined
}

export const leading = (code: string) => new RegExp(`^\\s*${code}(?:\\s*[—–:-]\\s+|$)`)
