import { describe, expect, it } from 'vitest'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import tasks from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'
import twoturns from '../../src-tauri/tests/fixtures/twoturns.jsonl?raw'
import subagents from '../../src-tauri/tests/fixtures/subagents.jsonl?raw'
import background from '../../src-tauri/tests/fixtures/background.jsonl?raw'
import backgroundInterrupt from '../../src-tauri/tests/fixtures/background_interrupt.jsonl?raw'
import thinking from '../../src-tauri/tests/fixtures/thinking.jsonl?raw'
import goalPlan from '../../src-tauri/tests/fixtures/goal_plan.jsonl?raw'
import goalClear from '../../src-tauri/tests/fixtures/goal_clear.jsonl?raw'
import worktreeEnter from '../../src-tauri/tests/fixtures/worktree_enter.jsonl?raw'
import worktreeFlag from '../../src-tauri/tests/fixtures/worktree_flag.jsonl?raw'
import { apply, calls, exited, fail, newAgent, prompt, replayLog, subagentOf, userMessage, type Agent, type Entry } from './stream'

function replay(agent: Agent, stream: string) {
  for (const line of stream.trim().split('\n')) apply(agent, JSON.parse(line))
}

function fresh() {
  const agent = newAgent('a', '/project', 'Test', { model: 'haiku', effort: 'low', mode: 'default' })
  userMessage(agent, 'hello')
  return agent
}

const kinds = (entries: Entry[]) => entries.map((e) => e.kind)

describe('notices', () => {
  it('shows a task notification from Claude Code as a note, not as a message of yours', () => {
    const agent = fresh()
    apply(agent, {
      type: 'user',
      isReplay: true,
      message: { role: 'user', content: '<task-notification>\nponytail: its hook was skipped.\n</task-notification>' },
    })
    expect(agent.entries.at(-1)).toMatchObject({ kind: 'note', command: 'notice', text: 'ponytail: its hook was skipped.' })
    expect(agent.entries.filter((e) => e.kind === 'user')).toHaveLength(1)
  })
})

describe('worktree', () => {
  it('follows an agent that enters a worktree with EnterWorktree', () => {
    const agent = fresh()
    replay(agent, worktreeEnter)
    expect(agent.worktree).toEqual({ path: '/project/.claude/worktrees/demo', branch: 'worktree-demo' })
  })

  it('reads the worktree of a session started with --worktree, until ExitWorktree', () => {
    const agent = fresh()
    const lines = worktreeFlag.trim().split('\n')
    replay(agent, lines[0])
    expect(agent.worktree).toEqual({ path: '/project/.claude/worktrees/demo2', branch: '' })
    replay(agent, lines.slice(1).join('\n'))
    expect(agent.worktree).toBeNull()
  })
})

describe('goal', () => {
  const decisions = (agent: Agent) => agent.entries.flatMap((e) => (e.kind === 'decision' ? [e.title] : []))

  it('marks the goal met when the turn ends without a relaunch', () => {
    const agent = fresh()
    replay(agent, goalPlan)
    expect(agent.goal).toMatchObject({ condition: 'running python3 count.py prints start, the numbers 1 to 5 one per line, then done', state: 'met', relaunches: 0 })
    expect(decisions(agent)).toEqual(['Goal met'])
  })

  it('keeps the agent working while its goal waits for the end of the first turn', () => {
    const agent = fresh()
    agent.goal = { condition: 'tests pass', state: 'sent', relaunches: 0, reason: '' }
    replay(agent, hello)
    expect(agent.status).toBe('working')
    apply(agent, { type: 'assistant', message: { id: 'g', model: '<synthetic>', content: [{ type: 'text', text: 'Goal set: tests pass' }] } })
    expect(agent.goal).toMatchObject({ state: 'active', condition: 'tests pass' })
    apply(agent, { type: 'result', is_error: false, num_turns: 2, total_cost_usd: 0.01, usage: {} })
    expect(agent.status).toBe('done')
    expect(agent.goal?.state).toBe('met')
  })

  it('counts the relaunches with their reason, and stops at the cap of Claude Code', () => {
    const agent = fresh()
    const lines = goalClear.trim().split('\n')
    const capped = lines.findIndex((l) => l.includes('stop-hook-block-cap'))
    replay(agent, lines.slice(0, capped + 1).join('\n'))
    expect(agent.goal).toMatchObject({ state: 'stopping', relaunches: 9 })
    expect(agent.goal?.reason).toMatch(/^\S/)
    expect(agent.goal?.reason).not.toContain('Stop hook feedback')
    expect(decisions(agent)).toEqual(Array(9).fill('Goal not met yet'))
    replay(agent, lines.slice(capped + 1).join('\n'))
    expect(agent.goal?.state).toBe('cleared')
    expect(decisions(agent)).not.toContain('Goal met')
  })
})

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
    const expired = fresh()
    fail(expired, 'Failed to authenticate: OAuth session expired and could not be refreshed.')
    const last = expired.entries.at(-1)
    expect(last?.kind === 'error' && last.login).toBe(true)
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

  it('keeps the agent waiting for you after a turn that ends on a question, until you reply', () => {
    const agent = fresh()
    replay(agent, hello)
    expect(agent.attention).toBe('')
    apply(agent, { type: 'system', subtype: 'post_turn_summary', status_category: 'blocked', needs_action: 'Pick a colour' })
    apply(agent, { type: 'result', is_error: false, total_cost_usd: 0.01, usage: {} })
    expect(agent.attention).toBe('Pick a colour')
    userMessage(agent, 'Red')
    expect(agent.attention).toBe('')
  })

  it('puts diff comments before the message sent to Claude', () => {
    const comments = [
      { path: 'src/a.php', line: 12, removed: false, code: '    return [];', note: 'Use the repository' },
      { path: 'b.txt', line: 3, removed: true, code: 'old', note: 'Why?' },
    ]
    expect(prompt('Go on', comments)).toBe('src/a.php:12\n> return [];\nUse the repository\n\nb.txt:3 (removed line)\n> old\nWhy?\n\nGo on')
    expect(prompt('', comments.slice(1))).toBe('b.txt:3 (removed line)\n> old\nWhy?')
    expect(prompt('Hi')).toBe('Hi')
  })

  it('follows slash commands, compaction, clear and remote control', () => {
    const agent = newAgent('a', '/project', 'a', { model: 'haiku', effort: 'low', mode: 'default' })
    apply(agent, { type: 'system', subtype: 'init', session_id: 's1', slash_commands: ['compact', 'clear', 'doctor', 'ponytail:ponytail'], terminal_slash_commands: ['doctor'], skills: ['ponytail:ponytail'] })
    expect(agent.commands).toEqual(['compact', 'clear', 'ponytail:ponytail'])
    expect(agent.skills).toEqual(['ponytail:ponytail'])
    userMessage(agent, '/context')
    apply(agent, { type: 'assistant', message: { id: 'syn-1', model: '<synthetic>', content: [{ type: 'text', text: '## Context Usage' }] } })
    apply(agent, { type: 'result', subtype: 'success', is_error: false, num_turns: 0, total_cost_usd: 0, usage: {} })
    expect(agent.turns).toBe(0)
    expect(agent.status).toBe('done')
    expect(agent.entries.some((e) => e.kind === 'agent' && e.blocks.some((b) => b.type === 'text' && b.text === '## Context Usage'))).toBe(true)
    apply(agent, { type: 'system', subtype: 'compact_boundary', compact_metadata: { pre_tokens: 46530, post_tokens: 14200 } })
    expect(agent.entries.at(-1)).toMatchObject({ kind: 'compacted', before: 46530, after: 14200 })
    expect(agent.tokens).toBe(14200)
    apply(agent, { type: 'conversation_reset', trigger: 'clear' })
    expect(agent.entries.at(-1)?.kind).toBe('cleared')
    apply(agent, { type: 'system', subtype: 'bridge_state', state: 'ready' })
    apply(agent, { type: 'control_response', response: { subtype: 'success', response: { session_url: 'https://claude.ai/code/session_1' } } })
    apply(agent, { type: 'system', subtype: 'bridge_state', state: 'connected' })
    expect(agent.remote).toEqual({ state: 'connected', url: 'https://claude.ai/code/session_1', since: expect.any(Number) })
  })

  it('keeps subagents apart from the conversation', () => {
    const agent = fresh()
    const lines = subagents.trim().split('\n')
    const asked = lines.findIndex((l) => l.includes('"parent_tool_use_id":"toolu_') && l.includes('"name":"Bash"'))
    for (const line of lines.slice(0, asked + 1)) apply(agent, JSON.parse(line))
    const [first, second] = agent.subagents
    expect(agent.subagents.map((s) => [s.num, s.type, s.status])).toEqual([
      [1, 'general-purpose', 'running'],
      [2, 'general-purpose', 'running'],
    ])
    expect(first.prompt).toContain('first.txt')
    const bash = first.log.flatMap((b) => (b.type === 'tools' ? b.calls : []))[0]
    expect(bash.name).toBe('Bash')
    expect(subagentOf(agent, bash.id)).toBe(first)
    expect(subagentOf(agent, 'toolu_unknown')).toBeUndefined()

    for (const line of lines.slice(asked + 1)) apply(agent, JSON.parse(line))
    const main = agent.entries.flatMap((e) => (e.kind === 'agent' ? e.blocks.flatMap((b) => (b.type === 'tools' ? b.calls : [])) : []))
    expect(main.map((c) => c.name).filter((n) => n !== 'ToolSearch')).toEqual(['Agent', 'Agent'])
    expect(agent.subagents.map((s) => s.status)).toEqual(['done', 'done'])
    expect(first.result).toBeTruthy()
    expect(first.tokens).toBeGreaterThan(0)
    expect(first.model).toContain('haiku')
    expect(second.log.some((b) => b.type === 'tools' && b.calls[0].result)).toBe(true)
    expect(calls(agent).map((c) => c.id)).toContain(bash.id)
    expect(agent.status).toBe('done')
  })

  it('keeps the agent working while a background subagent runs after its turn', () => {
    const agent = fresh()
    const lines = background.trim().split('\n')
    const turn = lines.findIndex((l) => JSON.parse(l).type === 'result')
    for (const line of lines.slice(0, turn + 1)) apply(agent, JSON.parse(line))
    expect(agent.subagents.map((s) => [s.background, s.status])).toEqual([[true, 'running']])
    expect(agent.status).toBe('working')
    expect(agent.action).toBe('Waiting for 1 subagent…')

    const settled = lines.findIndex((l) => l.includes('"subtype":"task_updated"'))
    for (const line of lines.slice(turn + 1, settled + 1)) apply(agent, JSON.parse(line))
    expect(agent.subagents[0].status).toBe('done')
    expect(agent.status).toBe('working')

    for (const line of lines.slice(settled + 1)) apply(agent, JSON.parse(line))
    expect(agent.status).toBe('done')
    expect(agent.subagents[0].result).toBe('DONE')
    expect(agent.entries.filter((e) => e.kind === 'user' && e.text.includes('Another Claude session'))).toHaveLength(0)
  })

  it('interrupts an agent that only waits for its background subagents', () => {
    const agent = fresh()
    const lines = backgroundInterrupt.trim().split('\n')
    const turn = lines.findIndex((l) => JSON.parse(l).type === 'result')
    for (const line of lines.slice(0, turn + 1)) apply(agent, JSON.parse(line))
    expect(agent.status).toBe('working')
    agent.interrupting = true
    for (const line of lines.slice(turn + 1)) apply(agent, JSON.parse(line))
    expect(agent.subagents[0].status).toBe('stopped')
    expect(agent.status).toBe('interrupted')
    expect(agent.interrupting).toBe(false)
  })

  it('keeps the summary of the thinking', () => {
    const agent = fresh()
    const lines = thinking.trim().split('\n')
    const streamed = lines.findIndex((l) => l.includes('"content_block_stop"'))
    for (const line of lines.slice(0, streamed)) apply(agent, JSON.parse(line))
    const live = agent.entries.find((e) => e.kind === 'agent')
    expect(live?.kind === 'agent' && live.blocks[0]).toEqual(expect.objectContaining({ type: 'thinking', text: expect.stringContaining('9.9') }))
    for (const line of lines.slice(streamed)) apply(agent, JSON.parse(line))
    const reply = agent.entries.find((e) => e.kind === 'agent')
    const block = reply?.kind === 'agent' ? reply.blocks[0] : undefined
    expect(block?.type === 'thinking' && block.ms).toBeGreaterThan(0)
    expect(block?.type === 'thinking' && block.text).toMatch(/^9\.9/)
  })

  it('shows messages sent from claude.ai', () => {
    const agent = fresh()
    agent.remote = { state: 'connected', url: 'https://claude.ai/code/session_1' }
    apply(agent, { type: 'user', isReplay: true, parent_tool_use_id: null, timestamp: '2026-10-22T09:00:00.000Z', message: { role: 'user', content: 'Ship it' } })
    expect(agent.entries.at(-1)).toEqual(expect.objectContaining({ kind: 'user', text: 'Ship it', remote: true, at: Date.parse('2026-10-22T09:00:00.000Z') }))
    expect(agent.status).toBe('working')
    expect(agent.remote.last).toBeGreaterThan(0)
    apply(agent, { type: 'user', message: { content: [{ type: 'tool_result', tool_use_id: 'x', content: 'ok' }] } })
    expect(agent.entries.filter((e) => e.kind === 'user')).toHaveLength(2)
  })

  it('keeps the waiting label while the agent keeps streaming', () => {
    const agent = fresh()
    agent.status = 'waiting'
    agent.action = 'Waiting for your approval · Bash'
    apply(agent, { type: 'assistant', message: { id: 'm', content: [{ type: 'tool_use', id: 'toolu_more', name: 'Agent', input: { description: 'More' } }] } })
    expect(agent.action).toBe('Waiting for your approval · Bash')
  })

  it('stops running subagents when the process exits', () => {
    const agent = fresh()
    const lines = subagents.trim().split('\n')
    for (const line of lines.slice(0, lines.findIndex((l) => l.includes('task_started')) + 1)) apply(agent, JSON.parse(line))
    exited(agent, '')
    expect(agent.subagents[0].status).toBe('stopped')
  })

  it('rebuilds a subagent log from its transcript', () => {
    const agent = fresh()
    replay(agent, subagents)
    const sub = agent.subagents[0]
    const live = sub.log.flatMap((b) => (b.type === 'tools' ? b.calls.map((c) => c.name) : []))
    const transcript = subagents
      .trim()
      .split('\n')
      .map((l) => JSON.parse(l))
      .filter((d) => d.parent_tool_use_id === sub.id)
      .map((d) => JSON.stringify({ ...d, parent_tool_use_id: null, timestamp: '2026-10-21T14:02:41.000Z', toolUseResult: d.tool_use_result }))
      .join('\n')
    replayLog(agent, sub, `${transcript}\nnot json`)
    expect(sub.log.flatMap((b) => (b.type === 'tools' ? b.calls.map((c) => c.name) : []))).toEqual(live)
    expect(sub.log[0].at).toBe(Date.parse('2026-10-21T14:02:41.000Z'))
    expect(sub.log.some((b) => b.type === 'tools' && b.calls[0].result)).toBe(true)
  })
})
