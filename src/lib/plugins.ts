export type Installed = {
  id: string
  name: string | null
  version: string
  scope: 'user' | 'project' | 'local' | 'managed'
  enabled: boolean
  installPath: string
  projectPath?: string
  description: string | null
}

export type Entry = { id: string; name: string; description: string; marketplace: string; installs: number }

export type Marketplace = { name: string; source: string; url?: string; repo?: string; path?: string }

export type Flags = { user: Record<string, boolean>; projects: Record<string, Record<string, boolean>> }

export type Component = { kind: 'skill' | 'agent' | 'mcp'; name: string; always: number; invoke: number }

export type Details = { always: number; components: Component[] }

export type Step = { args: string[]; dir?: string }

export type Target = 'all' | 'off' | string[]

export const OFFICIAL = 'claude-plugins-official'

const COLORS = ['#e08a5b', '#4fa3e0', '#a78bfa', '#7cc47c', '#e3b341', '#6fa0d0', '#3cc39a', '#e07a9b']

export const pluginColor = (id: string) => COLORS[[...id].reduce((sum, c) => sum + c.charCodeAt(0), 0) % COLORS.length]

export const ALL = '*'

export const SHOWS = [
  ['off', 'plugins.showOff', 'plugins.showOffHint'],
  ['tag', 'plugins.showTag', 'plugins.showTagHint'],
  ['card', 'plugins.showCard', 'plugins.showCardHint'],
  ['gutter', 'plugins.showGutter', 'plugins.showGutterHint'],
] as const

export const marketplaceOf = (id: string) => id.slice(id.lastIndexOf('@') + 1)

export const nameOf = (id: string) => id.slice(0, id.lastIndexOf('@'))

export const effective = (flags: Flags, id: string, path: string) => flags.projects[path]?.[id] ?? flags.user[id] ?? false

export const activeIn = (flags: Flags, id: string, paths: string[]) => paths.filter((p) => effective(flags, id, p))

export const everywhere = (flags: Flags, id: string, paths: string[]) => flags.user[id] === true && paths.every((p) => effective(flags, id, p))

export function plan(flags: Flags, id: string, paths: string[], target: Target): Step[] {
  const steps: Step[] = []
  const user = (verb: 'enable' | 'disable') => steps.push({ args: [verb, id, '--scope', 'user'] })
  const local = (verb: 'enable' | 'disable', dir: string) => steps.push({ args: [verb, id, '--scope', 'local'], dir })
  const chosen = Array.isArray(target) ? target.filter((p) => paths.includes(p)) : target
  if (chosen === 'all' || (Array.isArray(chosen) && paths.length > 0 && chosen.length === paths.length)) {
    if (flags.user[id] !== true) user('enable')
    for (const p of paths) if (flags.projects[p]?.[id] === false) local('enable', p)
    return steps
  }
  if (flags.user[id] !== false) user('disable')
  const want = new Set(chosen === 'off' ? [] : chosen)
  for (const p of paths) {
    const now = flags.projects[p]?.[id] ?? false
    if (now !== want.has(p)) local(want.has(p) ? 'enable' : 'disable', p)
  }
  return steps
}

export function after(flags: Flags, steps: Step[]): Flags {
  const next: Flags = { user: { ...flags.user }, projects: Object.fromEntries(Object.entries(flags.projects).map(([p, f]) => [p, { ...f }])) }
  for (const { args, dir } of steps) {
    const [verb, id, , scope] = args
    const on = verb === 'enable'
    if (scope === 'user') next.user[id] = on
    else if (dir) next.projects[dir] = { ...next.projects[dir], [id]: on }
  }
  return next
}
