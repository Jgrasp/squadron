import { describe, expect, it } from 'vitest'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import tasks from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'
import twoturns from '../../src-tauri/tests/fixtures/twoturns.jsonl?raw'
import { apply, exited, newAgent, userMessage, type Agent, type Entry } from './stream'

function replay(agent: Agent, stream: string) {
  for (const line of stream.trim().split('\n')) apply(agent, JSON.parse(line))
}

function fresh() {
  const agent = newAgent('a', '/project', 'Test', { model: 'haiku', effort: 'low', mode: 'default' })
  userMessage(agent, 'hello')
  return agent
}

const kinds = (entries: Entry[]) => entries.map((e) => e.kind)

describe('stream', () => {
  it('streams a text reply and closes the turn', () => {
    const agent = fresh()
    replay(agent, hello)
    expect(kinds(agent.entries)).toEqual(['user', 'agent', 'turn'])
    const reply = agent.entries[1]
    expect(reply.kind === 'agent' && reply.blocks).toEqual([
      expect.objectContaining({ type: 'text', text: 'Bonjour' }),
    ])
    expect(agent.status).toBe('done')
    expect(agent.cost).toBeCloseTo(0.00207658)
  })

  it('pairs tool calls with their results', () => {
    const agent = fresh()
    replay(agent, tools)
    const reply = agent.entries.find((e) => e.kind === 'agent')
    const calls = reply?.kind === 'agent' ? reply.blocks.flatMap((b) => (b.type === 'tools' ? b.calls : [])) : []
    expect(calls.map((c) => c.name)).toEqual(['Read', 'Bash', 'Write'])
    expect(calls.every((c) => c.result && !c.result.error)).toBe(true)
    expect(calls[2].input.file_path).toBe('/project/resume.txt')
    expect(calls[2].result?.data?.type).toBe('create')
    expect(reply?.kind === 'agent' && reply.blocks.filter((b) => b.type === 'thinking').every((b) => b.type === 'thinking' && b.ms !== undefined)).toBe(true)
    expect(agent.status).toBe('done')
  })

  it('splits the cumulative cost of a process into turns', () => {
    const agent = fresh()
    replay(agent, twoturns)
    const turns = agent.entries.filter((e) => e.kind === 'turn')
    expect(turns.map((t) => t.kind === 'turn' && t.index)).toEqual([1, 2])
    expect(turns[0].kind === 'turn' && turns[0].cost).toBeCloseTo(0.00160846)
    expect(turns[1].kind === 'turn' && turns[1].cost).toBeCloseTo(0.00315546 - 0.00160846)
    expect(agent.cost).toBeCloseTo(0.00315546)
  })

  it('marks an interrupted turn as interrupted, not as an error', () => {
    const agent = fresh()
    agent.interrupting = true
    replay(agent, sigint)
    exited(agent, '')
    expect(agent.status).toBe('interrupted')
    expect(kinds(agent.entries).filter((k) => k === 'interrupted')).toHaveLength(1)
    expect(agent.alive).toBe(false)
  })

  it('flags a logged-out Claude Code', () => {
    const agent = fresh()
    replay(agent, nologin)
    const error = agent.entries.at(-1)
    expect(agent.status).toBe('error')
    expect(error?.kind === 'error' && error.login).toBe(true)
  })

  it('turns an unexpected exit into an error', () => {
    const agent = fresh()
    exited(agent, 'Segmentation fault\n')
    expect(agent.status).toBe('error')
    expect(agent.action).toBe('Segmentation fault')
  })

  it('follows the agent task list', () => {
    const agent = fresh()
    replay(agent, tasks)
    expect(agent.tasks.map((t) => [t.id, t.subject, t.status])).toEqual([
      ['1', 'Write a.txt', 'completed'],
      ['2', 'Write b.txt', 'completed'],
      ['3', 'Write c.txt', 'completed'],
    ])
    const reply = agent.entries.find((e) => e.kind === 'agent')
    const names = reply?.kind === 'agent' ? reply.blocks.flatMap((b) => (b.type === 'tools' ? b.calls.map((c) => c.name) : [])) : []
    expect(names.filter((n) => n === 'Write')).toHaveLength(3)
  })

  it('keeps the session id and forgets the fork once the new session starts', () => {
    const agent = fresh()
    agent.sessionId = 'parent'
    agent.forkOf = 'Parent agent'
    replay(agent, hello)
    expect(agent.sessionId).not.toBe('parent')
    expect(agent.forkOf).toBeNull()
  })
})
