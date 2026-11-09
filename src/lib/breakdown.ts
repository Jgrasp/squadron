import type { PluginRef, Use } from './pluginuse'

type Json = any

export type ContextUsage = {
  categories: { id: string; tokens: number }[]
  skills: { name: string; source: string; plugin?: string; tokens: number }[]
  agents: { name: string; source: string; plugin?: string; tokens: number }[]
  memory: { path: string; tokens: number }[]
  tools: { name: string; server: string; tokens: number }[]
  instructions: { server: string; tokens: number }[]
  messages: { tools: { name: string; tokens: number }[]; replies: number; user: number; added: { name: string; tokens: number }[] }
}

export function trimUsage(r: Json): ContextUsage {
  const n = (v: unknown) => (typeof v === 'number' ? v : 0)
  const m = r.messageBreakdown ?? {}
  return {
    categories: (r.categories ?? []).map((c: Json) => ({ id: String(c.id), tokens: n(c.tokens) })),
    skills: (r.skills?.skillFrontmatter ?? []).map((s: Json) => ({ name: String(s.name), source: String(s.source), plugin: s.pluginName, tokens: n(s.tokens) })),
    agents: (r.agents ?? []).map((a: Json) => ({ name: String(a.agentType), source: String(a.source), plugin: a.pluginId ? String(a.pluginId).split('@')[0] : undefined, tokens: n(a.tokens) })),
    memory: (r.memoryFiles ?? []).map((f: Json) => ({ path: String(f.path), tokens: n(f.tokens) })),
    tools: (r.mcpTools ?? []).filter((t: Json) => t.isLoaded).map((t: Json) => ({ name: String(t.name), server: String(t.serverName), tokens: n(t.tokens) })),
    instructions: (r.mcpInstructions ?? []).map((i: Json) => ({ server: String(i.serverName), tokens: n(i.tokens) })),
    messages: {
      tools: (m.toolCallsByType ?? []).map((t: Json) => ({ name: String(t.name), tokens: n(t.callTokens) + n(t.resultTokens) })),
      replies: n(m.assistantMessageTokens),
      user: n(m.userMessageTokens),
      added: (m.attachmentsByType ?? []).map((a: Json) => ({ name: String(a.name), tokens: n(a.tokens) })).filter((a: { name: string }) => a.name !== 'prompt_snapshot'),
    },
  }
}

export type LineKind =
  | 'prompt'
  | 'squadronPrompt'
  | 'builtinTools'
  | 'builtinSkills'
  | 'syncedSkills'
  | 'other'
  | 'server'
  | 'memory'
  | 'skills'
  | 'agents'
  | 'mcpTools'
  | 'mcpInstructions'
  | 'tool'
  | 'call'
  | 'hook'
  | 'added'

export type Line = { kind: LineKind; tokens: number; name?: string; count?: number; plugin?: string; use?: string; estimate?: boolean }

export type GroupId = 'claude' | 'plugins' | 'squadron' | 'you' | 'mcp' | 'tools' | 'calls' | 'replies' | 'messages' | 'added'

export type Group = { id: GroupId; tokens: number; lines: Line[] }

export type Share = { name: string; tokens: number; lines: Line[] }

export type Breakdown = { total: number; start: Group[]; talk: Group[]; plugins: Share[] }

const STARTUP = ['system_prompt', 'system_tools', 'mcp_tools', 'mcp_instructions', 'custom_agents', 'memory_files', 'skills']
const OWN = ['built-in', 'syncedSkills', 'plugin']
const SPAWNS = ['Agent', 'Task']

const total = (lines: { tokens: number }[]) => lines.reduce((s, l) => s + l.tokens, 0)
const tail = (path: string) => path.split('/').slice(-2).join('/')

function pluginOfServer(refs: PluginRef[], server: string) {
  const colon = /^plugin:([^:]+):/.exec(server)?.[1]
  if (colon) return colon
  if (!server.startsWith('plugin_')) return undefined
  return refs.map((r) => r.name).filter((name) => server.startsWith(`plugin_${name}_`)).sort((a, b) => b.length - a.length)[0] ?? server.slice(7).split('_')[0]
}

const toolServer = (name: string) => /^mcp__(.+?)__/.exec(name)?.[1]

export function breakdown(u: ContextUsage, refs: PluginRef[], prompt: number, uses: Use[] = [], hooks: { text: string; tokens: number }[] = []): Breakdown {
  const category = (id: string) => u.categories.find((c) => c.id === id)?.tokens ?? 0
  const shares = new Map<string, Line[]>(refs.map((r) => [r.name, []]))
  const servers = new Map<string, number>()
  const own = (plugin: string) => shares.get(plugin) ?? shares.set(plugin, []).get(plugin)!

  for (const kind of ['skills', 'agents'] as const) {
    const per = new Map<string, Json[]>()
    for (const item of u[kind]) if (item.plugin) per.set(item.plugin, [...(per.get(item.plugin) ?? []), item])
    for (const [plugin, items] of per) own(plugin).push({ kind, count: items.length, tokens: total(items) })
  }
  for (const tool of u.tools) {
    const plugin = pluginOfServer(refs, tool.server)
    if (plugin) {
      const lines = own(plugin)
      const line = lines.find((l) => l.kind === 'mcpTools')
      if (line) (line.count = (line.count ?? 0) + 1), (line.tokens += tool.tokens)
      else lines.push({ kind: 'mcpTools', count: 1, tokens: tool.tokens })
    } else servers.set(tool.server, (servers.get(tool.server) ?? 0) + tool.tokens)
  }
  for (const item of u.instructions) {
    const plugin = pluginOfServer(refs, item.server)
    if (plugin) own(plugin).push({ kind: 'mcpInstructions', tokens: item.tokens })
    else servers.set(item.server, (servers.get(item.server) ?? 0) + item.tokens)
  }
  const plugins: Share[] = [...shares].map(([name, lines]) => ({ name, lines, tokens: total(lines) })).sort((a, b) => b.tokens - a.tokens)

  const squadronServers = [...servers].filter(([name]) => name.startsWith('squadron'))
  const otherServers = [...servers].filter(([name]) => !name.startsWith('squadron'))
  const ours = Math.min(prompt, category('system_prompt'))
  const builtin = u.skills.filter((s) => s.source === 'built-in')
  const synced = u.skills.filter((s) => s.source === 'syncedSkills')
  const userSkills = u.skills.filter((s) => !OWN.includes(s.source))
  const userAgents = u.agents.filter((a) => !a.plugin)

  const claude: Line[] = [
    { kind: 'prompt', tokens: category('system_prompt') - ours },
    { kind: 'builtinTools', tokens: category('system_tools') },
    { kind: 'builtinSkills', count: builtin.length, tokens: total(builtin) },
    { kind: 'syncedSkills', count: synced.length, tokens: total(synced) },
  ]
  const squadron: Line[] = [{ kind: 'squadronPrompt', tokens: ours }, ...squadronServers.map(([name, tokens]) => ({ kind: 'server' as const, name, tokens }))]
  const you: Line[] = [
    ...u.memory.map((f) => ({ kind: 'memory' as const, name: tail(f.path), tokens: f.tokens })),
    { kind: 'skills', count: userSkills.length, tokens: total(userSkills) },
    { kind: 'agents', count: userAgents.length, tokens: total(userAgents) },
  ]
  const mcp: Line[] = otherServers.map(([name, tokens]): Line => ({ kind: 'server', name, tokens })).sort((a, b) => b.tokens - a.tokens)
  const counted = total(claude) + total(squadron) + total(you) + total(mcp) + total(plugins)
  const missing = STARTUP.reduce((s, id) => s + category(id), 0) - counted
  if (missing > 0) claude.push({ kind: 'other', tokens: missing })

  const start: Group[] = [
    { id: 'claude' as const, lines: claude },
    { id: 'plugins' as const, lines: [] },
    { id: 'squadron' as const, lines: squadron },
    { id: 'you' as const, lines: you },
    { id: 'mcp' as const, lines: mcp },
  ]
    .map((g) => ({ ...g, lines: g.lines.filter((l) => l.tokens > 0), tokens: g.id === 'plugins' ? total(plugins) : total(g.lines) }))
    .sort((a, b) => b.tokens - a.tokens)

  const tools = new Map(u.messages.tools.map((t) => [t.name, t.tokens]))
  const calls: Line[] = []
  for (const [name, tokens] of tools) {
    const plugin = pluginOfServer(refs, toolServer(name) ?? '')
    if (!plugin) continue
    calls.push({ kind: 'call', name: name.split('__').at(-1), plugin, tokens })
    tools.delete(name)
  }
  let user = u.messages.user
  let budget = Math.max(0, category('messages') - total([...tools].map(([, tokens]) => ({ tokens }))) - total(calls) - u.messages.replies - user)
  for (const use of uses) {
    if (use.kind === 'mcp') continue
    const row = use.parts[0]?.key === 'file' ? 'Read' : use.kind === 'agent' ? SPAWNS.find((s) => tools.has(s)) : 'Skill'
    const fromRow = row && tools.has(row) ? Math.min(tools.get(row)!, use.tokens) : 0
    if (row && fromRow) tools.set(row, tools.get(row)! - fromRow)
    const fromUser = use.kind === 'skill' && row === 'Skill' ? Math.min(user, use.tokens - fromRow) : 0
    user -= fromUser
    const fromRest = Math.min(budget, use.tokens - fromRow - fromUser)
    budget -= fromRest
    calls.push({ kind: 'call', name: use.label, plugin: use.plugin, use: use.id, tokens: fromRow + fromUser + fromRest, estimate: true })
  }
  const talk: Group[] = [
    { id: 'tools' as const, lines: [...tools].map(([name, tokens]): Line => ({ kind: 'tool', name, tokens })).sort((a, b) => b.tokens - a.tokens) },
    { id: 'calls' as const, lines: calls },
    { id: 'replies' as const, lines: [] as Line[], tokens: u.messages.replies },
    { id: 'messages' as const, lines: [] as Line[], tokens: user },
    {
      id: 'added' as const,
      lines: [
        ...hooks.map((h): Line => ({ kind: 'hook', name: h.text, tokens: h.tokens, estimate: true })),
        ...u.messages.added.filter((a) => !hooks.length && a.name === 'hook_success').map((a): Line => ({ kind: 'added', name: a.name, tokens: a.tokens, estimate: true })),
      ],
      tokens: budget,
    },
  ].map((g) => {
    const lines = g.lines.filter((l) => l.tokens > 0)
    return { ...g, lines, tokens: 'tokens' in g && g.tokens !== undefined ? g.tokens : total(lines) }
  })

  return { total: total(start) + total(talk), start, talk, plugins }
}
