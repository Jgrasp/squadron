import { describe, expect, it } from 'vitest'
import { abilities, groups, namespace, shortName } from './agenttools'

describe('groups', () => {
  it('sorts the tools of an agent by what they let it do, and its MCP tools by server', () => {
    expect(groups(['Read', 'mcp__plugin_codegraph_codegraph__codegraph_explore', 'Write', 'Glob', 'mcp__chrome-devtools__click', 'Skill', 'Bash'])).toEqual([
      { kind: 'read', tools: ['Read', 'Glob'] },
      { kind: 'write', tools: ['Write'] },
      { kind: 'run', tools: ['Bash'] },
      { kind: 'mcp', server: 'chrome-devtools', tools: ['click'] },
      { kind: 'mcp', server: 'codegraph', tools: ['codegraph_explore'] },
      { kind: 'other', tools: ['Skill'] },
    ])
  })
})

describe('abilities', () => {
  it('tells what an agent can do from its tools', () => {
    expect(abilities(['Read', 'Edit'])).toEqual({ read: true, write: true, run: false, web: false, delegate: false })
  })

  it('gives every ability to an agent without a tool list', () => {
    expect(Object.values(abilities(null)).every(Boolean)).toBe(true)
  })
})

describe('names', () => {
  it('splits the plugin of an agent from its name', () => {
    expect([namespace('crew:architect'), shortName('crew:architect')]).toEqual(['crew:', 'architect'])
    expect([namespace('Explore'), shortName('Explore')]).toEqual(['', 'Explore'])
  })
})
