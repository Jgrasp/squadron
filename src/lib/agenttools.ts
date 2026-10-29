export type Ability = 'read' | 'write' | 'run' | 'web' | 'delegate'

export type ToolGroup = { kind: Ability | 'mcp' | 'other'; server?: string; tools: string[] }

const ABILITIES: Record<Ability, string[]> = {
  read: ['Read', 'Glob', 'Grep', 'LS', 'NotebookRead'],
  write: ['Write', 'Edit', 'MultiEdit', 'NotebookEdit'],
  run: ['Bash', 'BashOutput', 'KillShell', 'KillBash', 'PowerShell', 'Monitor'],
  web: ['WebFetch', 'WebSearch'],
  delegate: ['Agent', 'Task', 'SendMessage'],
}

export const ABILITY_ORDER = Object.keys(ABILITIES) as Ability[]

function kindOf(tool: string): Ability | undefined {
  return ABILITY_ORDER.find((ability) => ABILITIES[ability].includes(tool))
}

export function groups(tools: string[]): ToolGroup[] {
  const found: ToolGroup[] = []
  const add = (kind: ToolGroup['kind'], tool: string, server?: string) => {
    const group = found.find((g) => g.kind === kind && g.server === server)
    if (group) group.tools.push(tool)
    else found.push({ kind, server, tools: [tool] })
  }
  for (const tool of tools) {
    const mcp = tool.match(/^mcp__(.+?)__(.+)$/)
    if (mcp) add('mcp', mcp[2], mcp[1].replace(/^plugin_[^_]+_/, ''))
    else add(kindOf(tool) ?? 'other', tool)
  }
  const rank = (g: ToolGroup) => (g.kind === 'mcp' ? 5 : g.kind === 'other' ? 6 : ABILITY_ORDER.indexOf(g.kind))
  return found.sort((a, b) => rank(a) - rank(b) || (a.server ?? '').localeCompare(b.server ?? ''))
}

export function abilities(tools: string[] | null): Record<Ability, boolean> {
  return Object.fromEntries(ABILITY_ORDER.map((ability) => [ability, !tools || tools.some((tool) => kindOf(tool) === ability)])) as Record<Ability, boolean>
}

export const shortName = (name: string) => name.slice(name.lastIndexOf(':') + 1)

export const namespace = (name: string) => (name.includes(':') ? name.slice(0, name.lastIndexOf(':') + 1) : '')
