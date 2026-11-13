const SHELL = /[;&|`$<>(){}\\\n]/

export function ruleFor(tool: string, input: Record<string, any>): string {
  if (tool === 'Bash') {
    const command = String(input.command ?? '').trim()
    if (SHELL.test(command)) return `Bash(${command})`
    return `Bash(${command.split(/\s+/).slice(0, 2).join(' ')}:*)`
  }
  if (tool === 'WebFetch') {
    try {
      return `WebFetch(domain:${new URL(String(input.url)).hostname})`
    } catch {
      return 'WebFetch'
    }
  }
  return tool
}

export function matches(rule: string, tool: string, input: Record<string, any>): boolean {
  const parsed = /^([^()]+)(?:\((.*)\))?$/s.exec(rule)
  if (!parsed || parsed[1] !== tool) return false
  const arg = parsed[2]
  if (arg === undefined) return tool !== 'Bash'
  if (tool === 'Bash') {
    const command = String(input.command ?? '').trim()
    if (!arg.endsWith(':*')) return command === arg
    if (SHELL.test(command)) return false
    const prefix = arg.slice(0, -2)
    return command === prefix || command.startsWith(prefix + ' ')
  }
  if (tool === 'WebFetch' && arg.startsWith('domain:')) {
    try {
      return new URL(String(input.url)).hostname === arg.slice('domain:'.length)
    } catch {
      return false
    }
  }
  return false
}

export function ruleLabel(rule: string) {
  return /^Bash\((.*)\)$/s.exec(rule)?.[1] ?? rule
}

const GUARDED = ['.git', '.claude']

export function editable(dir: string, path: unknown) {
  if (!dir || typeof path !== 'string') return false
  const root = dir.replaceAll('\\', '/').replace(/\/+$/, '')
  const file = path.replaceAll('\\', '/')
  if (!file.startsWith(`${root}/`)) return false
  const parts = file.slice(root.length + 1).split('/')
  return !parts.some((p) => p === '..' || p === '.' || GUARDED.includes(p)) && parts.join('/') !== '.mcp.json'
}
