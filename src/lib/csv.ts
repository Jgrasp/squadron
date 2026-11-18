export const TABULAR = /\.(csv|tsv|tab)$/i

export function delimiterOf(text: string, name = ''): string {
  if (/\.(tsv|tab)$/i.test(name)) return '\t'
  const counts = new Map<string, number>([[',', 0], [';', 0], ['\t', 0], ['|', 0]])
  let quoted = false
  for (const char of text.slice(0, 20_000)) {
    if (char === '"') quoted = !quoted
    else if (!quoted && (char === '\n' || char === '\r')) break
    else if (!quoted && counts.has(char)) counts.set(char, counts.get(char)! + 1)
  }
  return [...counts].reduce((best, next) => (next[1] > best[1] ? next : best))[0]
}

export function parseCsv(text: string, delimiter: string): string[][] {
  const rows: string[][] = []
  let row: string[] = []
  let cell = ''
  let quoted = false
  let i = text.charCodeAt(0) === 0xfeff ? 1 : 0
  for (; i < text.length; i++) {
    const char = text[i]
    if (quoted) {
      if (char !== '"') cell += char
      else if (text[i + 1] === '"') (cell += '"'), i++
      else quoted = false
    } else if (char === '"' && cell === '') quoted = true
    else if (char === delimiter) row.push(cell), (cell = '')
    else if (char === '\n' || char === '\r') {
      if (char === '\r' && text[i + 1] === '\n') i++
      row.push(cell), rows.push(row), (row = []), (cell = '')
    } else cell += char
  }
  if (cell !== '' || row.length) row.push(cell), rows.push(row)
  return rows
}

const NUMBER = /^[-+]?(\d{1,3}([\s.,]\d{3})*|\d+)([.,]\d+)?\s*%?$/

export function numericColumns(rows: string[][], width: number): boolean[] {
  return Array.from({ length: width }, (_, column) => {
    let seen = false
    for (const row of rows) {
      const value = row[column]?.trim()
      if (!value) continue
      if (!NUMBER.test(value)) return false
      seen = true
    }
    return seen
  })
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })

function numberOf(value: string): number {
  const bare = value.replace(/[\s%]/g, '')
  const decimal = /[.,]\d{1,2}$|[.,]\d{4,}$/.test(bare) ? bare.match(/[.,](?=\d+$)/)?.index : undefined
  const whole = decimal === undefined ? bare.replace(/[.,]/g, '') : bare.slice(0, decimal).replace(/[.,]/g, '') + '.' + bare.slice(decimal + 1)
  return Number(whole)
}

export function compareCells(a = '', b = '', numeric = false): number {
  if (!a || !b) return a ? -1 : b ? 1 : 0
  if (numeric) return numberOf(a) - numberOf(b)
  return collator.compare(a, b)
}
