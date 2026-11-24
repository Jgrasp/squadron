import { describe, expect, it } from 'vitest'
import { docsOf, fileName, fillShare, freeId, insideArtifacts, sharePrompt, type ArtifactInfo } from './artifacts'
import { newAgent, type ToolCall } from './stream'

describe('fileName', () => {
  it('turns the title into a file name with the extension of its kind', () => {
    expect(fileName({ title: 'Plan de migration : étape 2', kind: 'markdown' })).toBe('plan-de-migration-etape-2.md')
    expect(fileName({ title: 'Schéma', kind: 'svg' })).toBe('schema.svg')
    expect(fileName({ title: '« »', kind: 'html' })).toBe('artifact.html')
  })
})

describe('freeId', () => {
  it('names the folder after the title, numbered when taken', () => {
    expect(freeId('Maquette accueil', [])).toBe('maquette-accueil')
    expect(freeId('Maquette accueil', ['maquette-accueil', 'maquette-accueil-2'])).toBe('maquette-accueil-3')
  })
})

describe('insideArtifacts', () => {
  const root = '/data/artifacts'

  it('lets an agent write only inside its own artifacts folder', () => {
    expect(insideArtifacts(root, 'a1', '/data/artifacts/a1/page/index.html')).toBe(true)
    expect(insideArtifacts(root, 'a1', '/data/artifacts/a2/page/index.html')).toBe(false)
    expect(insideArtifacts(root, 'a1', '/data/artifacts/a1/../a2/index.html')).toBe(false)
    expect(insideArtifacts(root, 'a1', '/data/artifacts/a1')).toBe(false)
    expect(insideArtifacts('', 'a1', '/a1/page/index.html')).toBe(false)
    expect(insideArtifacts(root, 'a1', undefined)).toBe(false)
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
    expect(docsOf(agentWith([birth(link)]), [link])).toEqual([])
  })

  it('lists what the Artifact tool created, not its updates', () => {
    const deck = 'https://claude.ai/artifact/61qyoH18ecsnyRYCxSSjgx'
    const tool = (input: Record<string, unknown>, text: string): ToolCall => ({ id: crypto.randomUUID(), name: 'Artifact', input, result: { text, error: false } })
    const created = tool({ title: 'Deck' }, `Created a new Artifact at ${deck} (version 1)`)
    const updated = tool({ url: deck }, `Updated the Artifact at ${deck} (Version 2)`)
    expect(docsOf(agentWith([created, updated]))).toEqual([{ id: deck, title: 'Deck', kind: 'doc', link: deck }])
  })
})

describe('sharePrompt', () => {
  const artifact = (kind: ArtifactInfo['kind']) => ({ id: 'plan', title: 'Plan', kind, entry: kind === 'html' ? 'index.html' : 'index.md' }) as ArtifactInfo

  it('publishes markdown as a Claude Doc, html as a page and asks for a presentation on demand', () => {
    expect(sharePrompt(artifact('markdown'), '/a/plan')).toContain('Claude Docs connector')
    expect(sharePrompt(artifact('html'), '/a/plan')).toMatch(/as a page with your Artifact tool, from its files in \/a\/plan with index\.html/)
    expect(sharePrompt(artifact('markdown'), '/a/plan', true)).toMatch(/Slides presentation.*destination slides/)
  })
})

describe('fillShare', () => {
  it('fills the known names and leaves the others', () => {
    expect(fillShare('Share {artifact.id} at {artifact.dir} from {extension.data} for {agent} ({agent.id}), {other}', { 'agent.id': 'a1', 'artifact.id': 'plan', 'artifact.dir': '/a/plan', 'extension.data': '/x/team', agent: 'Atlas' })).toBe('Share plan at /a/plan from /x/team for Atlas (a1), {other}')
  })
})
