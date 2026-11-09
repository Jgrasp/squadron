import { describe, expect, it } from 'vitest'
import { uses } from './pluginuse'
import { apply, newAgent, userMessage } from './stream'

const tool = (id: string, name: string, input: object) => ({
  type: 'assistant',
  message: { id: `m-${id}`, content: [{ type: 'tool_use', id, name, input }] },
})
const result = (id: string, text: string) => ({ type: 'user', message: { content: [{ type: 'tool_result', tool_use_id: id, content: text }] } })

function session() {
  const agent = newAgent('a', '/project', 'Test', { model: 'haiku', effort: 'low', mode: 'default' })
  userMessage(agent, 'hello')
  apply(agent, {
    type: 'system',
    subtype: 'init',
    session_id: 's',
    slash_commands: [],
    plugins: [
      { name: 'humanizer', path: '/cache/humanizer/3.1.0', source: 'humanizer@humanizer' },
      { name: 'codegraph', path: '/dev/codegraph', source: 'codegraph@inline' },
      { name: 'cc-plugin-telemetry', path: 'builtin', source: 'cc-plugin-telemetry@builtin' },
    ],
  })
  return agent
}

describe('plugin use', () => {
  it('keeps the plugins an agent loaded, without the built-in ones', () => {
    expect(session().plugins?.map((p) => p.id)).toEqual(['humanizer@humanizer', 'codegraph@inline'])
  })

  it('charges a plugin skill with the instructions it loaded', () => {
    const agent = session()
    apply(agent, tool('t1', 'Skill', { skill: 'humanizer:humanizer', args: 'text' }))
    apply(agent, result('t1', 'Launching skill: humanizer:humanizer'))
    apply(agent, { type: 'user', isSynthetic: true, message: { content: [{ type: 'text', text: `Base directory for this skill: /cache/humanizer\n${'x'.repeat(4000)}` }] } })
    const [use] = uses(agent)
    expect(use).toMatchObject({ plugin: 'humanizer', kind: 'skill', label: 'humanizer text' })
    expect(use.tokens).toBeGreaterThan(1000)
  })

  it('finds plugin MCP tools, plugin agents and files read in a plugin', () => {
    const agent = session()
    apply(agent, tool('t1', 'mcp__plugin_codegraph_codegraph__codegraph_explore', { query: 'ProductForm' }))
    apply(agent, result('t1', 'y'.repeat(800)))
    apply(agent, tool('t2', 'Agent', { subagent_type: 'humanizer:reviewer', prompt: 'p'.repeat(40) }))
    apply(agent, tool('t3', 'Read', { file_path: '/cache/humanizer/3.1.0/reference/tone.md' }))
    apply(agent, tool('t4', 'Read', { file_path: '/project/src/app.ts' }))
    apply(agent, tool('t5', 'Skill', { skill: 'my-own-skill' }))
    expect(uses(agent).map((u) => [u.plugin, u.kind, u.label])).toEqual([
      ['codegraph', 'mcp', 'codegraph_explore'],
      ['humanizer', 'agent', 'reviewer'],
      ['humanizer', 'skill', 'reference/tone.md'],
    ])
    expect(uses(agent)[0].tokens).toBe(200 + Math.ceil(JSON.stringify({ query: 'ProductForm' }).length / 4))
  })
})
