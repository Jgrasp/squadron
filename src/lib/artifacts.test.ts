import { describe, expect, it } from 'vitest'
import { docsOf, fileName } from './artifacts'
import { newAgent, type ToolCall } from './stream'

describe('fileName', () => {
  const artifact = { id: 'a', content: '', version: 1, at: 0 }

  it('turns the title into a file name with the extension of its kind', () => {
    expect(fileName({ ...artifact, title: 'Plan de migration : étape 2', kind: 'markdown' })).toBe('plan-de-migration-etape-2.md')
    expect(fileName({ ...artifact, title: 'Schéma', kind: 'svg' })).toBe('schema.svg')
  })

  it('falls back to artifact when nothing is left of the title', () => {
    expect(fileName({ ...artifact, title: '« »', kind: 'html' })).toBe('artifact.html')
  })
})

describe('docsOf', () => {
  const link = 'https://claude.ai/artifact/plan-de-migration-3f2a9c1e-5b7d-4e8a-9c21-7d4e5f6a8b90'
  const birth = (text: string, error = false): ToolCall => ({
    id: crypto.randomUUID(),
    name: 'mcp__claude_ai_Claude_Docs__batch',
    input: { container: { kind: 'project', create: { name: 'Plan de migration' } }, batch: [] },
    result: { text, error },
  })
  const agentWith = (calls: ToolCall[]) => {
    const agent = newAgent('a', '/project', 'Test', { model: 'haiku', effort: 'low', mode: 'default' })
    agent.entries.push({ kind: 'agent', blocks: [{ type: 'tools', calls }] } as never)
    return agent
  }

  it('lists the docs the agent created on claude.ai, once each', () => {
    expect(docsOf(agentWith([birth(`created ${link}\n`), birth(`again ${link}`)]))).toEqual([{ id: link, title: 'Plan de migration', kind: 'doc', link }])
  })

  it('leaves out failed calls, edits, hidden docs and docs an artifact already links to', () => {
    const edit: ToolCall = { id: 'e', name: 'mcp__claude_ai_Claude_Docs__batch', input: { container: { kind: 'project', id: 'x' } }, result: { text: link, error: false } }
    expect(docsOf(agentWith([birth(link, true), edit]))).toEqual([])
    const hidden = agentWith([birth(link)])
    hidden.hiddenDocs = [link]
    expect(docsOf(hidden)).toEqual([])
    const linked = agentWith([birth(link)])
    linked.artifacts = [{ id: 'a', title: 'Plan', kind: 'markdown', content: '#', version: 1, at: 0, link }]
    expect(docsOf(linked)).toEqual([])
  })
})
