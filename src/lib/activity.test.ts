import { describe, expect, it } from 'vitest'
import { activityOf, agentChanges, apiSignal, noted, projectChanges, projectOf, usageOf, type ClaudeApi } from './activity'
import { newAgent, type Agent } from './stream'
import type { Project } from './projects.svelte'

const agent = (patch: Partial<Agent> = {}): Agent => ({ ...newAgent('a1', '/home/me/ring', 'RING-1 Card payment', {} as Agent['options']), status: 'done', since: 10, ...patch })

const call = (name: string, result?: unknown) => ({ kind: 'agent', blocks: [{ type: 'tools', calls: [{ id: 'c', name, input: { command: 'rm -rf /' }, at: 0, result }] }] }) as unknown as Agent['entries'][number]

describe('activity', () => {
  it('tells what an agent does without its content', () => {
    const working = activityOf(agent({ status: 'working', entries: [call('Bash')], startedBy: 'a0' }))
    expect(working).toEqual({ id: 'a1', project: '/home/me/ring', name: 'RING-1 Card payment', status: 'working', waiting: null, tool: 'Bash', error: null, by: 'a0', archived: false, since: 10, unread: false })
    expect(JSON.stringify(working)).not.toContain('rm -rf')
    expect(activityOf(agent({ status: 'working', entries: [call('Bash', { text: 'ok' })] })).tool).toBeNull()
    expect(activityOf(agent({ status: 'working', entries: [call('TaskCreate')] })).tool).toBeNull()
    const ask = (tool_name: string) => [{ id: 'q', agent: 'a1', tool_name, input: { questions: ['secret?'] }, tool_use_id: null }]
    expect(activityOf(agent({ status: 'waiting', asks: ask('AskUserQuestion') })).waiting).toBe('question')
    expect(activityOf(agent({ status: 'waiting', asks: ask('ExitPlanMode') })).waiting).toBe('question')
    expect(activityOf(agent({ status: 'waiting', asks: ask('Bash') })).waiting).toBe('permission')
    expect(activityOf(agent({ status: 'error', alive: false })).error).toBe('crash')
    expect(activityOf(agent({ status: 'error', alive: true })).error).toBe('api')
    expect(activityOf(agent({ status: 'done', mcpFailed: true })).error).toBe('mcp')
    expect(activityOf(agent({ autoName: true, name: 'fix the login of' })).name).toBe('')
    expect(activityOf(agent({ status: 'error', mcpFailed: true, unread: true }), true)).toMatchObject({ archived: true, error: null, waiting: null, tool: null, unread: false })
    expect(activityOf(agent({ unread: true })).unread).toBe(true)
  })

  it('counts the agents of a project over the last week, archived ones included', () => {
    const now = 100 * 86_400_000
    const project = { path: '/home/me/ring', name: 'Ring', color: '', group: 'g1', archived: false } as Project
    const all = [activityOf(agent({ since: now - 86_400_000 })), activityOf(agent({ id: 'a2', since: now - 9 * 86_400_000 }), true), activityOf(agent({ id: 'a3', project: '/other', since: now }))]
    expect(projectOf(project, all, now)).toEqual({ id: '/home/me/ring', name: 'Ring', color: null, group: 'g1', archived: false, active: now - 86_400_000, week_agents: 1, agents: 2 })
  })

  it('reads the state of Claude’s API from the stream', () => {
    expect(apiSignal({ type: 'system', subtype: 'api_retry', attempt: 3, error_status: 529, error: 'overloaded' })).toEqual({ state: 'overloaded', retries: 3 })
    expect(apiSignal({ type: 'system', subtype: 'api_retry', attempt: 1, error_status: 503, error: 'server_error' })).toEqual({ state: 'outage' })
    expect(apiSignal({ type: 'system', subtype: 'api_retry', attempt: 1, error_status: 408, error: 'server_error' })).toBeNull()
    expect(apiSignal({ type: 'system', subtype: 'api_retry', attempt: 1, error_status: null, error: 'unknown' })).toEqual({ state: 'offline' })
    expect(apiSignal({ type: 'system', subtype: 'api_retry', attempt: 1, error_status: 429, error: 'rate_limit' })).toBeNull()
    expect(apiSignal({ type: 'assistant', error: 'authentication_failed' })).toEqual({ state: 'signed_out' })
    expect(apiSignal({ type: 'result', is_error: true, result: 'Not logged in · Please run /login' })).toEqual({ state: 'signed_out' })
    expect(apiSignal({ type: 'result', is_error: true, result: 'x', api_error_status: 500 })).toEqual({ state: 'outage' })
    expect(apiSignal({ type: 'result', is_error: false })).toEqual({ state: 'ok' })
    expect(apiSignal({ type: 'rate_limit_event' })).toEqual({ state: 'ok' })
    expect(apiSignal({ type: 'assistant' })).toBeNull()
    const api: ClaudeApi = { state: 'ok', retries: 0, since: 1 }
    const busy = noted(api, { state: 'overloaded', retries: 1 }, 5)
    expect(busy).toEqual({ state: 'overloaded', retries: 1, since: 5 })
    expect(noted(busy, { state: 'overloaded', retries: 2 }, 9)).toEqual({ state: 'overloaded', retries: 2, since: 5 })
    expect(noted(busy, null)).toBe(busy)
    expect(usageOf({ five_hour: { utilization: 0.78, resetsAt: 1_700_000_000 } }, busy)).toEqual({ five_hour: { utilization: 0.78, resets_at: 1_700_000_000_000 }, seven_day: null, api: 'overloaded', retries: 1, since: 5 })
  })

  it('turns changes into events', () => {
    const seen = new Map<string, string>()
    const one = activityOf(agent())
    expect(agentChanges(seen, [one], new Set(), true).events).toEqual([])
    expect(agentChanges(seen, [one], new Set(), false).events).toEqual([])
    const two = activityOf(agent({ id: 'a2', startedBy: 'a1' }))
    const busy = { ...one, status: 'working' as const }
    expect(agentChanges(seen, [busy, two], new Set(), false).events).toEqual([
      { name: 'agent.status', data: busy },
      { name: 'agent.started', data: two },
    ])
    expect(agentChanges(seen, [two], new Set(), false)).toEqual({ events: [], gone: ['a1'] })
    expect(agentChanges(seen, [two, one], new Set(['a1']), false).events).toEqual([{ name: 'agent.status', data: one }])

    const projects = new Map<string, string>()
    const ring = projectOf({ path: '/r', name: 'Ring', color: '#fff', group: '', archived: false } as Project, [])
    expect(projectChanges(projects, [ring], true)).toEqual([])
    expect(projectChanges(projects, [{ ...ring, agents: 4, active: 9 }], false)).toEqual([])
    expect(projectChanges(projects, [{ ...ring, color: '#000' }], false).map((e) => e.name)).toEqual(['project.changed'])
    expect(projectChanges(projects, [{ ...ring, color: '#000', archived: true }], false).map((e) => e.name)).toEqual(['project.archived'])
    const other = { ...ring, id: '/o' }
    expect(projectChanges(projects, [{ ...ring, color: '#000', archived: true }, other], false).map((e) => e.name)).toEqual(['project.added'])
    expect(projectChanges(projects, [other], false).map((e) => [e.name, (e.data as { id: string }).id])).toEqual([['project.removed', '/r']])
  })
})
