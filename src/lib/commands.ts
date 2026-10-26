export type Command = { name: string; description: string; argumentHint?: string; aliases?: string[]; builtin?: boolean }

export type Model = { value: string; displayName?: string; description?: string; supportedEffortLevels?: string[] }

const ALIASES = ['opus', 'sonnet', 'haiku']

export function families(models: Model[]) {
  const aliases = models.map((m) => m.value).filter((v) => v !== 'default' && !v.startsWith('claude-'))
  return aliases.length ? aliases : ALIASES
}

export function familyOf(value: string, models: Model[]) {
  const name = (models.find((m) => m.value === value)?.displayName ?? value).toLowerCase()
  return families(models).find((f) => name.startsWith(f)) ?? value
}

export const versionsOf = (family: string, models: Model[]) => models.filter((m) => m.value !== 'default' && familyOf(m.value, models) === family)

const MODEL_ID = /^claude-([a-z]+)-(\d+)-(\d+)(?:-\d{8})?(\[1m\])?$/

export function modelName(value: string, models: Model[]) {
  const known = models.find((m) => m.value === value)?.displayName
  if (known) return known
  const id = MODEL_ID.exec(value)
  if (id) return `${id[1][0].toUpperCase()}${id[1].slice(1)} ${id[2]}.${id[3]}${id[4] ? ' (1M)' : ''}`
  return value[0].toUpperCase() + value.slice(1)
}

export type Catalog = { commands: Command[]; models: Model[]; styles: string[] }

export type Choice = { value: string; label?: string; detail?: string }

export const UNLISTED: Command[] = [
  {
    name: 'remote-control',
    description: 'Control this session from your phone or claude.ai/code',
    aliases: ['rc'],
    argumentHint: '[on|off]',
    builtin: true,
  },
]

export function choices(hint = ''): string[] {
  const s = hint.trim()
  let inner = s.split(' ')[0]
  if ('[<('.includes(s[0])) {
    let depth = 0
    for (let i = 0; i < s.length; i++) {
      if ('[<('.includes(s[i])) depth++
      else if (']>)'.includes(s[i]) && --depth === 0) {
        inner = s.slice(1, i)
        break
      }
    }
  }
  const parts: string[] = []
  let depth = 0
  let current = ''
  for (const ch of inner) {
    if ('[<('.includes(ch)) depth++
    if (depth === 0 && ch === '|') {
      parts.push(current)
      current = ''
      continue
    }
    if (depth === 0 && ch === ' ') break
    current += ch
    if (']>)'.includes(ch)) depth--
  }
  parts.push(current)
  return parts.length > 1 ? parts.map((p) => p.trim()).filter((p) => p && !/[[<(]/.test(p)) : []
}

export function options(command: Command, catalog?: Catalog): Choice[] {
  if (command.name === 'model' && catalog?.models.length)
    return catalog.models.map((m) => ({ value: m.value, label: m.displayName, detail: m.description }))
  if (command.name === 'output-style' && catalog?.styles.length) return catalog.styles.map((value) => ({ value }))
  return choices(command.argumentHint).map((value) => ({ value }))
}

export function find(commands: Command[], name: string) {
  return commands.find((c) => c.name === name || c.aliases?.includes(name))
}

export function parse(text: string) {
  const match = /^\/(\S+)(?:\s+([\s\S]*))?$/.exec(text.trim())
  return match ? { name: match[1], args: (match[2] ?? '').trim() } : null
}

export function highlight(text: string, names: Set<string>) {
  const parts: { text: string; command: boolean }[] = []
  let last = 0
  for (const m of text.matchAll(/(^|\s)(\/[\w:.-]+)/g)) {
    const start = m.index + m[1].length
    if (!names.has(m[2].slice(1))) continue
    if (start > last) parts.push({ text: text.slice(last, start), command: false })
    parts.push({ text: m[2], command: true })
    last = start + m[2].length
  }
  if (last < text.length) parts.push({ text: text.slice(last), command: false })
  return parts
}
