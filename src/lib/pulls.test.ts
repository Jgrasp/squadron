import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Lists, Pull } from './pulls.svelte'

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
vi.mock('./forges.svelte', () => ({ forgeName: (host: string) => host, providedLinks: [] }))

const { awaitsChecks, lists, lookUpLists, lookUpNumbered, numbered, openReview, refreshPull } = await import('./pulls.svelte')

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
