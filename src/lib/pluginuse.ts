import type { Agent, ToolCall } from './stream'

export type PluginRef = { name: string; id: string; path: string }

export type Kind = 'skill' | 'agent' | 'mcp'

export type Part = { key: 'skillBody' | 'agentPrompt' | 'agentReport' | 'request' | 'answer' | 'file'; label?: string; tokens: number }

export type Use = { id: string; plugin: string; kind: Kind; label: string; tokens: number; at?: number; parts: Part[]; excerpt?: string }

export const KINDS: Kind[] = ['skill', 'agent', 'mcp']

const SPAWNS = ['Task', 'Agent']

export const estimate = (text: string | number | undefined) => Math.ceil((typeof text === 'number' ? text : (text ?? '').length) / 4)

const prefixed = (refs: PluginRef[], value: unknown) => {
  const [plugin, ...rest] = String(value ?? '').split(':')
  return rest.length && refs.some((r) => r.name === plugin) ? { plugin, label: rest.join(':') } : null
}

export function owner(refs: PluginRef[], call: ToolCall): { plugin: string; kind: Kind; label: string } | null {
  if (call.name === 'Skill') {
    const hit = prefixed(refs, call.input.skill ?? call.input.command)
    const args = String(call.input.args ?? '').split('\n')[0].trim()
    return hit && { ...hit, label: args ? `${hit.label} ${args}` : hit.label, kind: 'skill' }
  }
  if (SPAWNS.includes(call.name)) {
    const hit = prefixed(refs, call.input.subagent_type)
    return hit && { ...hit, kind: 'agent' }
  }
  if (call.name.startsWith('mcp__plugin_')) {
    const ref = refs.filter((r) => call.name.startsWith(`mcp__plugin_${r.name}_`)).sort((a, b) => b.name.length - a.name.length)[0]
    return ref ? { plugin: ref.name, kind: 'mcp', label: call.name.split('__').at(-1) ?? call.name } : null
  }
  if (call.name === 'Read' && typeof call.input.file_path === 'string') {
    const file = call.input.file_path
    const ref = refs.find((r) => r.path.startsWith('/') && file.startsWith(`${r.path}/`))
    return ref ? { plugin: ref.name, kind: 'skill', label: file.slice(ref.path.length + 1) } : null
  }
  return null
}

const cut = (text: string | undefined) => {
  const clean = (text ?? '').trim()
  return clean ? (clean.length > 400 ? `${clean.slice(0, 400)}…` : clean) : undefined
}

export function useOf(refs: PluginRef[], call: ToolCall): Use | null {
  const who = owner(refs, call)
  if (!who) return null
  const result = call.result?.text ?? ''
  let parts: Part[]
  let excerpt: string | undefined
  if (call.name === 'Read') parts = [{ key: 'file', label: who.label, tokens: estimate(result) }]
  else if (who.kind === 'skill') parts = [{ key: 'skillBody', tokens: estimate(call.loaded ?? 0) + estimate(result) }]
  else if (who.kind === 'agent') {
    parts = [
      { key: 'agentPrompt', tokens: estimate(String(call.input.prompt ?? '')) },
      { key: 'agentReport', tokens: estimate(result) },
    ]
    excerpt = cut(result)
  } else {
    parts = [
      { key: 'request', tokens: estimate(JSON.stringify(call.input)) },
      { key: 'answer', tokens: estimate(result) },
    ]
    excerpt = cut(result)
  }
  return { id: call.id, ...who, tokens: parts.reduce((s, p) => s + p.tokens, 0), at: call.at, parts, excerpt }
}

function commandUse(refs: PluginRef[], entry: { text: string; at: number }, invoked: (plugin: string, skill: string) => number): Use | null {
  const typed = /^\/(\S+)[ \t]*(.*)/.exec(entry.text.trim())
  const hit = typed && prefixed(refs, typed[1])
  if (!hit) return null
  const tokens = invoked(hit.plugin, hit.label)
  return { id: `command-${entry.at}`, ...hit, kind: 'skill', label: typed[2] ? `${hit.label} ${typed[2]}` : hit.label, tokens, at: entry.at, parts: [{ key: 'skillBody', tokens }] }
}

export function uses(agent: Agent, invoked: (plugin: string, skill: string) => number = () => 0): Use[] {
  const refs = agent.plugins ?? []
  if (!refs.length) return []
  const out: Use[] = []
  for (const entry of agent.entries) {
    const typed = entry.kind === 'user' && commandUse(refs, entry, invoked)
    if (typed) out.push(typed)
    if (entry.kind !== 'agent') continue
    for (const block of entry.blocks) {
      if (block.type !== 'tools') continue
      for (const call of block.calls) {
        const use = useOf(refs, call)
        if (use) out.push(use)
      }
    }
  }
  return out
}

export type Share = { ref: PluginRef; start: number; kinds: Record<Kind, number>; total: number; uses: Use[] }

export function shares(refs: PluginRef[], list: Use[], start: (ref: PluginRef) => number): Share[] {
  return refs
    .map((ref) => {
      const own = list.filter((u) => u.plugin === ref.name)
      const kinds = Object.fromEntries(KINDS.map((k) => [k, own.filter((u) => u.kind === k).reduce((s, u) => s + u.tokens, 0)])) as Record<Kind, number>
      const first = start(ref)
      return { ref, start: first, kinds, total: first + KINDS.reduce((s, k) => s + kinds[k], 0), uses: own }
    })
    .sort((a, b) => b.total - a.total)
}

export function server(call: ToolCall, plugin: string) {
  const middle = call.name.split('__')[1] ?? ''
  return middle.startsWith(`plugin_${plugin}_`) ? middle.slice(`plugin_${plugin}_`.length) : middle
}
