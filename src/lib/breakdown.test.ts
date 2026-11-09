import { describe, expect, it } from 'vitest'
import { breakdown, trimUsage } from './breakdown'
import type { Use } from './pluginuse'

const raw = {
  totalTokens: 9700,
  categories: [
    { id: 'system_prompt', tokens: 3100 },
    { id: 'system_tools', tokens: 2000 },
    { id: 'mcp_tools', tokens: 900 },
    { id: 'mcp_instructions', tokens: 700 },
    { id: 'mcp_tools_deferred', tokens: 237000 },
    { id: 'custom_agents', tokens: 300 },
    { id: 'memory_files', tokens: 500 },
    { id: 'skills', tokens: 400 },
    { id: 'messages', tokens: 1800 },
    { id: 'free_space', tokens: 900000 },
  ],
  skills: {
    skillFrontmatter: [
      { name: 'loop', source: 'built-in', tokens: 100 },
      { name: 'pdf', source: 'syncedSkills', tokens: 60 },
      { name: 'chiffrage', source: 'userSettings', tokens: 90 },
      { name: 'humanizer:humanizer', source: 'plugin', pluginName: 'humanizer', tokens: 110 },
      { name: 'codegraph:setup', source: 'plugin', pluginName: 'codegraph', tokens: 40 },
    ],
  },
  agents: [
    { agentType: 'crew:architect', source: 'plugin', pluginId: 'crew@inline', tokens: 180 },
    { agentType: 'visual-review', source: 'userSettings', tokens: 120 },
  ],
  memoryFiles: [{ path: '/home/me/.claude/CLAUDE.md', type: 'User', tokens: 500 }],
  mcpTools: [
    { name: 'mcp__plugin_codegraph_codegraph__codegraph_explore', serverName: 'plugin_codegraph_codegraph', tokens: 600, isLoaded: true },
    { name: 'mcp__squadron__permission', serverName: 'squadron', tokens: 300, isLoaded: true },
    { name: 'mcp__sentry__search', serverName: 'sentry', tokens: 1000, isLoaded: false },
  ],
  mcpInstructions: [
    { serverName: 'plugin:codegraph:codegraph', tokens: 500 },
    { serverName: 'sentry', tokens: 200 },
  ],
  messageBreakdown: {
    toolCallsByType: [
      { name: 'Read', callTokens: 40, resultTokens: 500 },
      { name: 'mcp__plugin_codegraph_codegraph__codegraph_explore', callTokens: 30, resultTokens: 270 },
      { name: 'Skill', callTokens: 20, resultTokens: 10 },
    ],
    assistantMessageTokens: 300,
    userMessageTokens: 100,
    attachmentTokens: 41000,
    attachmentsByType: [
      { name: 'prompt_snapshot', tokens: 17000 },
      { name: 'hook_success', tokens: 2800 },
      { name: 'skill_listing', tokens: 4100 },
    ],
  },
}

const refs = [
  { name: 'humanizer', id: 'humanizer@humanizer', path: '/cache/humanizer' },
  { name: 'codegraph', id: 'codegraph@inline', path: '/dev/codegraph' },
  { name: 'crew', id: 'crew@inline', path: '/dev/crew' },
  { name: 'warp', id: 'warp@warp', path: '/cache/warp' },
]

const skillUse: Use = { id: 't1', plugin: 'humanizer', kind: 'skill', label: 'humanizer', tokens: 400, parts: [{ key: 'skillBody', tokens: 400 }] }

describe('context breakdown', () => {
  const b = breakdown(trimUsage(raw), refs, 800, [skillUse], [{ text: 'PONYTAIL MODE ACTIVE', tokens: 350 }])
  const group = (id: string) => [...b.start, ...b.talk].find((g) => g.id === id)!

  it('adds up to what Claude Code counts in the context', () => {
    expect(b.total).toBe(raw.totalTokens)
    for (const g of [...b.start, ...b.talk].filter((g) => g.id !== 'added' && g.id !== 'plugins')) expect(g.tokens).toBe(g.lines.length ? g.lines.reduce((s, l) => s + l.tokens, 0) : g.tokens)
  })

  it('takes Squadron out of the system prompt and gives it its own servers', () => {
    expect(group('squadron').lines).toEqual([
      { kind: 'squadronPrompt', tokens: 800 },
      { kind: 'server', name: 'squadron', tokens: 300 },
    ])
    expect(group('claude').lines[0]).toEqual({ kind: 'prompt', tokens: 2300 })
  })

  it('charges each plugin for its skills, agents, MCP tools and instructions, inline ones included', () => {
    expect(b.plugins.map((p) => [p.name, p.tokens])).toEqual([
      ['codegraph', 1140],
      ['crew', 180],
      ['humanizer', 110],
      ['warp', 0],
    ])
    expect(group('plugins').tokens).toBe(1430)
    expect(group('mcp').lines).toEqual([{ kind: 'server', name: 'sentry', tokens: 200 }])
  })

  it('moves plugin calls out of the tool results and of the messages that carry skill bodies, without counting them twice', () => {
    expect(group('tools').lines.map((l) => [l.name, l.tokens])).toEqual([['Read', 540]])
    expect(group('calls').lines).toEqual([
      { kind: 'call', name: 'codegraph_explore', plugin: 'codegraph', tokens: 300 },
      { kind: 'call', name: 'humanizer', plugin: 'humanizer', use: 't1', tokens: 400, estimate: true },
    ])
    expect(group('messages').tokens).toBe(0)
    expect(group('added').tokens).toBe(1800 - 540 - 300 - 400 - 300)
    expect(group('added').lines).toEqual([{ kind: 'hook', name: 'PONYTAIL MODE ACTIVE', tokens: 350, estimate: true }])
    expect(breakdown(trimUsage(raw), refs, 0).talk.find((g) => g.id === 'added')!.lines).toEqual([{ kind: 'added', name: 'hook_success', tokens: 2800, estimate: true }])
  })
})
