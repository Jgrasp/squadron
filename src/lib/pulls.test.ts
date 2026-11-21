import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Lists, Pull } from './pulls.svelte'
import type { Agent } from './stream'

const forge = vi.hoisted(() => ({ lists: {} as Omit<Lists, 'error'>, pull: null as unknown, calls: [] as string[], offline: false }))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string) => {
    forge.calls.push(command)
    if (command === 'pull_lists') return structuredClone(forge.lists)
    if (forge.offline) throw new Error('offline')
    if (command === 'numbered_pull') return structuredClone(forge.pull)
    return null
  }),
}))

vi.mock('./agents.svelte', () => ({ agents: { list: [], current: {} }, create: vi.fn(), FOR_INFO: '', persist: vi.fn(), send: vi.fn() }))
vi.mock('./projects.svelte', () => ({ projects: { list: [] }, defaultsFor: vi.fn() }))
vi.mock('./layout.svelte', () => ({ layout: { review: null }, requestPanel: vi.fn(), setPanel: vi.fn() }))
vi.mock('./settings.svelte', () => ({ settings: {} }))
vi.mock('./forges.svelte', () => ({ forgeName: (host: string) => host, providedLinks: [], coverage: { '/p': { by: 'github' } }, can: (host: string, call: string) => host === 'github' && call === 'create' }))
const pushed = vi.hoisted(() => ({ ref: null as { branch: string } | { url: string } | null }))
vi.mock('./pushed', () => ({ pushedTo: () => pushed.ref }))

const { pullToOpen, awaitsChecks, ignore, ignored, ignoredPulls, lists, lookUpLists, lookUpNumbered, numbered, openReview, owned, pullWait, refreshPull, tidyMerged, unignore, waitsFor } = await import('./pulls.svelte')

const pull = (number: number, more: Partial<Pull> = {}): Pull => ({
  host: 'github',
  number,
  title: 'Release',
  description: '',
  url: `https://github.com/o/r/pull/${number}`,
  state: 'open',
  draft: false,
  approvals: 0,
  changes_requested: false,
  comments: 0,
  checks: 'pending',
  branch: 'release',
  base: 'main',
  head: 'abc',
  author: 'me',
  author_avatar: '',
  updated: '',
  reviewed: null,
  reviewed_head: null,
  reviewed_at: null,
  head_at: null,
  reviewers: [],
  conflicts: false,
  mergeable: false,
  ...more,
})

const settle = () => new Promise((resolve) => setTimeout(resolve))

describe('pull request freshness', () => {
  beforeEach(() => {
    forge.calls = []
  })

  it('opening a review fetches the lists again, even right after they were fetched', async () => {
    forge.lists = { review: [], mine: [], others: [] }
    await lookUpLists('/p')
    expect(lists['/p'].mine).toEqual([])
    forge.lists = { review: [], mine: [pull(7, { checks: 'passing', mergeable: true })], others: [] }
    openReview('/p', 7)
    await settle()
    expect(lists['/p'].mine[0]).toMatchObject({ number: 7, mergeable: true })
  })

  it('opening a review fetches a pull request outside the lists again instead of keeping it forever', async () => {
    forge.pull = pull(8)
    await lookUpNumbered('/p', 8)
    forge.pull = pull(8, { checks: 'passing', mergeable: true })
    await lookUpNumbered('/p', 8)
    expect(numbered['/p#8']?.mergeable).toBe(false)
    openReview('/p', 8)
    await settle()
    expect(numbered['/p#8']?.mergeable).toBe(true)
  })

  it('keeps the pull request it has when the forge fails to answer', async () => {
    forge.offline = true
    await refreshPull('/p', 8)
    forge.offline = false
    expect(forge.calls).toContain('numbered_pull')
    expect(numbered['/p#8']?.mergeable).toBe(true)
  })

  it('keeps asking while checks of your own pull request still run', () => {
    forge.lists = { review: [], mine: [], others: [] }
    lists['/p'] = { review: [], mine: [pull(9)], others: [], error: '' }
    expect(awaitsChecks('/p', pull(9))).toBe(true)
    expect(awaitsChecks('/p', pull(9, { checks: 'passing', mergeable: true }))).toBe(false)
    expect(awaitsChecks('/p', pull(9, { checks: 'failing' }))).toBe(false)
    expect(awaitsChecks('/p', pull(10))).toBe(false)
  })
})

describe('merged pull requests', () => {
  it('drops the worktree and branch of an asleep agent once its pull request is merged, only once', async () => {
    forge.calls = []
    const agent = { id: 'a1', project: '/p', alive: false, pinned: true, worktree: { path: '/p/.claude/worktrees/feat', branch: 'feat' } } as Agent
    const dropped = () => forge.calls.filter((c) => c === 'drop_merged_worktree').length
    owned.a1 = pull(9, { branch: 'feat' })
    await tidyMerged(agent)
    owned.a1 = pull(9, { branch: 'other', state: 'merged' })
    await tidyMerged(agent)
    owned.a1 = pull(9, { branch: 'feat', state: 'merged' })
    agent.alive = true
    await tidyMerged(agent)
    expect(dropped()).toBe(0)

    agent.alive = false
    await tidyMerged(agent)
    await tidyMerged(agent)
    expect(dropped()).toBe(1)
    expect(agent.worktree).toBeNull()
    expect(agent.pinned).toBe(false)
    expect(forge.calls).toContain('set_workdir')
  })
})

describe('waiting on a pull request', () => {
  const waiting = (...commands: string[]) =>
    ({
      id: 'w1',
      project: '/p',
      held: true,
      subagents: [],
      background: commands.map((_, i) => ({ id: `t${i}`, toolUseId: `c${i}`, name: '', started: 0 })),
      entries: [{ kind: 'agent', blocks: [{ type: 'tools', calls: commands.map((command, i) => ({ id: `c${i}`, input: { command } })) }] }],
    }) as unknown as Agent

  it('tells the checks wait from the merge wait, and lets any other background task show as background', () => {
    owned.w1 = pull(50)
    expect(pullWait(waiting('sleep 15; timeout 1800 gh pr checks 50 --watch --fail-fast'))?.merge).toBe(false)
    expect(pullWait(waiting(`timeout 7200 sh -c 'until [ "$(gh pr view 50 --json state -q .state)" != OPEN ]; do sleep 30; done'`))).toMatchObject({ merge: true, pull: { number: 50 } })
    expect(pullWait(waiting('gh pr view 50 --json state', 'pnpm test'))).toBeNull()
    owned.w1 = pull(50, { state: 'merged' })
    expect(pullWait(waiting('gh pr view 50 --json state'))).toBeNull()
    delete owned.w1
  })
})

describe('ignored pull requests', () => {

  it('an ignored pull request no longer waits for you, until you bring it back', async () => {
    forge.lists = { review: [pull(20)], mine: [], others: [] }
    await lookUpLists('/i', true)
    ignore('/i', pull(20), false)
    expect(waitsFor('/i', pull(20, { head: 'def' }))).toBe(false)
    unignore('/i', pull(20))
    expect(waitsFor('/i', pull(20))).toBe(true)
  })

  it('ignoring until new commits brings it back once the author pushes, and forgets it', async () => {
    ignore('/i', pull(21), true)
    expect(ignored('/i', pull(21))).toBe(true)
    expect(waitsFor('/i', pull(21, { head: 'def' }))).toBe(true)
    forge.lists = { review: [pull(21, { head: 'def' })], mine: [], others: [] }
    await lookUpLists('/i', true)
    expect('/i#21' in ignoredPulls).toBe(false)
  })

  it('forgets an ignored pull request once it is no longer open', async () => {
    ignore('/i', pull(22), false)
    forge.lists = { review: [], mine: [], others: [] }
    await lookUpLists('/i', true)
    expect('/i#22' in ignoredPulls).toBe(false)
  })
})

describe('reviews started on their own', () => {
  it('starts one agent review per refresh and per new head of a pull request waiting for you, only in a project that asked for it', async () => {
    const { projects } = await import('./projects.svelte')
    const { settings } = await import('./settings.svelte')
    const checkouts = () => forge.calls.filter((c) => c === 'review_checkout').length
    Object.assign(settings, { pilot: true })
    forge.calls = []
    forge.lists = { review: [pull(30), pull(31, { draft: true }), pull(32)], mine: [], others: [] }
    projects.list = [{ path: '/a', auto_review: false }] as typeof projects.list
    await lookUpLists('/a', true)
    await settle()
    expect(checkouts()).toBe(0)

    projects.list = [{ path: '/a', auto_review: true }] as typeof projects.list
    await lookUpLists('/a', true)
    await settle()
    expect(checkouts()).toBe(1)
    await lookUpLists('/a', true)
    await lookUpLists('/a', true)
    await settle()
    expect(checkouts()).toBe(2)
  })
})

describe('opening a pull request', () => {
  it('offers it for a pushed branch with no pull request yet, on a forge that can create one', () => {
    const agent = { id: 'o1', project: '/p', worktree: null } as unknown as Agent
    expect(pullToOpen(agent)).toBeNull()
    pushed.ref = { branch: 'feat/xero' }
    expect(pullToOpen(agent)).toEqual({ branch: 'feat/xero', host: 'github' })
    pushed.ref = { branch: '3c1d9e0' }
    expect(pullToOpen(agent)).toBeNull()
    pushed.ref = { url: 'https://github.com/o/r/pull/3' }
    expect(pullToOpen(agent)).toBeNull()
    pushed.ref = { branch: 'feat/xero' }
    expect(pullToOpen({ ...agent, project: '/elsewhere' } as Agent)).toBeNull()
    expect(pullToOpen({ ...agent, review: { project: '/p', number: 1 } } as unknown as Agent)).toBeNull()
    pushed.ref = null
  })
})
