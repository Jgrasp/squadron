const SHELL = /[;&|`$<>(){}\\\n]/

const WRAPPERS = ['env', 'timeout', 'nice', 'nohup', 'time', 'xargs', 'stdbuf', 'ionice', 'command', 'exec', 'eval', 'builtin', 'sudo', 'doas', 'su', 'watch', 'setsid', 'unbuffer', 'strace', 'flock', 'chroot', 'find', 'bash', 'sh', 'zsh', 'dash', 'ksh', 'fish', 'npx', 'pnpx', 'bunx', 'uvx', 'pipx']

const general = (words: string[]) => words.length > 0 && !WRAPPERS.includes(words[0].split('/').pop()!) && !words[0].includes('=') && !words[1]?.startsWith('-')

export function ruleFor(tool: string, input: Record<string, any>): string {
  if (tool === 'Bash') {
    const command = String(input.command ?? '').trim()
    const words = command.split(/\s+/)
    if (SHELL.test(command) || !general(words)) return `Bash(${command})`
    return `Bash(${words.slice(0, 2).join(' ')}:*)`
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
    if (!general(prefix.split(/\s+/))) return false
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
