import { describe, expect, it } from 'vitest'
import { chains, chats, match, projectsFor, projectsNamed, pullActs, pullsIn, pullsNamed, VERBS, verbsFor, verbsOf, waits, words, type Listed, type World } from './spotlight'
import type { Pull } from './pulls.svelte'
import type { Project } from './projects.svelte'
import type { Agent } from './stream'

const project = (name: string, path = `/p/${name.toLowerCase()}`, extra: Partial<Project> = {}) => ({ name, path, color: '#fff', exists: true, ...extra }) as Project

const agent = (id: string, name: string, at: Project, extra: Partial<Agent> = {}) =>
  ({ id, name, project: at.path, pinned: false, sessionId: 's', status: 'done', ...extra }) as Agent

const squadron = project('Squadron')
const uh = project('UH')
const bmc = project('BMC - TMC')
const gone = project('Gone', '/p/gone', { exists: false })
const login = agent('1', 'Fix the login redirect', squadron)
const export_ = agent('2', 'Export script', uh)

const world = (extra: Partial<World> = {}): World => ({ projects: [squadron, uh, bmc, gone], current: squadron, agents: [login, export_], shown: login, ...extra })

const ids = (query: string, extra?: Partial<World>) => match(query, world(extra)).map((r) => r.key)

describe('spotlight actions', () => {
  it('reads a verb and a project together', () => {
    expect(ids('pr uh')).toEqual(['pulls:/p/uh'])
    expect(ids('PR UH')).toEqual(['pulls:/p/uh'])
    expect(ids('new bmc')).toEqual(['new:/p/bmc - tmc'])
    expect(ids('pull requests uh')).toEqual(['pulls:/p/uh'])
  })

  it('offers every project for a verb alone, the current one first, then by priority', () => {
    expect(ids('pr')).toEqual(['pulls:/p/squadron', 'pulls:/p/uh', 'pulls:/p/bmc - tmc', 'pulls:/p/gone'])
    expect(ids('pr', { priority: (p) => (p === bmc ? 3 : 0) })).toEqual(['pulls:/p/squadron', 'pulls:/p/bmc - tmc', 'pulls:/p/uh', 'pulls:/p/gone'])
  })

  it('leaves out the projects a verb cannot act on', () => {
    expect(ids('new')).not.toContain('new:/p/gone')
    expect(ids('open')).toContain('open:/p/gone')
  })

  it('targets the current agent and project when none is named', () => {
    expect(ids('arc')).toEqual(['archive:1', 'archiveProject:/p/squadron'])
    expect(ids('arc', { shown: undefined })).toEqual(['archiveProject:/p/squadron'])
    expect(ids('archive uh')).toEqual(['archive:2', 'archiveProject:/p/uh'])
  })

  it('finds an agent by its name', () => {
    expect(ids('arc export')).toEqual(['archive:2'])
    expect(ids('fork login')).toEqual(['fork:1'])
  })

  it('only offers an agent action the agent can take', () => {
    expect(ids('stop')).toEqual([])
    expect(ids('stop', { shown: { ...login, status: 'working' } as Agent })).toEqual(['interrupt:1'])
    expect(ids('pin')).toEqual(['pin:1'])
    expect(ids('pin', { shown: { ...login, pinned: true } as Agent })).toEqual(['unpin:1'])
    expect(ids('fork', { shown: { ...login, sessionId: null } as Agent })).toEqual([])
  })

  it('opens a settings tab', () => {
    expect(ids('set lay')).toEqual(['settings:layout'])
    expect(ids('settings hosting')).toEqual(['settings:hosting'])
    expect(ids('set')).toHaveLength(6)
  })

  it('does not take a plain search for an action', () => {
    expect(ids('')).toEqual([])
    expect(ids('p')).toEqual([])
    expect(ids('new login')).toEqual([])
    expect(ids('refactor the pricing')).toEqual([])
    expect(ids('pr zzz')).toEqual([])
  })

  it('ignores accents and case', () => {
    expect(words('Éditeur  BMC - TMC')).toEqual(['editeur', 'bmc', 'tmc'])
    expect(ids('Edit uh')).toEqual(['editor:/p/uh'])
  })

  it('completes to the canonical words', () => {
    expect(match('pr u', world())[0].complete).toBe('pr uh')
    expect(match('arc exp', world())[0].complete).toBe('arc export script')
    expect(match('set lay', world())[0].complete).toBe('set layout')
  })
})

describe('spotlight chips', () => {
  const keys = (rows: { key: string }[]) => rows.map((r) => r.key)
  const verb = (id: string) => VERBS.find((v) => v.id === id)!

  it('lists the actions of a project, the named one first', () => {
    expect(keys(verbsOf(uh, 'pr'))).toEqual(['pulls:/p/uh'])
    expect(keys(verbsOf(uh, 'kan'))).toEqual(['board:/p/uh'])
    expect(keys(verbsOf(gone, ''))).toEqual(['open:/p/gone', 'pulls:/p/gone', 'board:/p/gone', 'archiveProject:/p/gone'])
  })

  it('lists the projects of an action, the current one first, then by priority', () => {
    expect(keys(projectsFor(verb('pulls'), '', world({ priority: (p) => (p === bmc ? 3 : 0) })))).toEqual(['pulls:/p/squadron', 'pulls:/p/bmc - tmc', 'pulls:/p/uh', 'pulls:/p/gone'])
    expect(keys(projectsFor(verb('new'), '', world()))).not.toContain('new:/p/gone')
    expect(keys(projectsFor(verb('pulls'), 'bm', world()))).toEqual(['pulls:/p/bmc - tmc'])
  })

  it('lists what an agent can do', () => {
    expect(keys(verbsFor(login, '', world()))).toEqual(['show:1', 'archive:1', 'fork:1', 'pin:1'])
    expect(keys(verbsFor({ ...login, status: 'working' } as Agent, 'stop', world()))).toEqual(['interrupt:1'])
  })

  it('names projects and conversations without a folder', () => {
    expect(projectsNamed('squ', world()).map((p) => p.name)).toEqual(['Squadron'])
    expect(projectsNamed('s', world())).toEqual([])
    expect(chats('chat')).toBe(true)
    expect(chats('new conv')).toBe(false)
    expect(chats('c')).toBe(false)
  })

  it('chains a project action typed alone', () => {
    const [pr] = match('pr', world())
    expect(chains(pr, 'pr')).toBe(true)
    expect(chains(match('pr uh', world())[0], 'pr uh')).toBe(false)
    expect(chains(match('set lay', world())[0], 'set')).toBe(false)
  })
})

describe('spotlight pull requests', () => {
  const pull = (number: number, title: string, extra: Partial<Pull> = {}) => ({ number, title, draft: false, ...extra }) as Pull
  const listed: Listed[] = [
    { project: squadron, pull: pull(412, 'Refactor the checkout steps') },
    { project: uh, pull: pull(31, 'Push notification opt-in') },
    { project: uh, pull: pull(4120, 'Draft checkout', { draft: true }) },
  ]
  const numbers = (found: Listed[]) => found.map((f) => f.pull.number)

  it('finds a pull request by its title or its number, drafts left out', () => {
    expect(numbers(pullsNamed('checkout', listed))).toEqual([412])
    expect(numbers(pullsNamed('#41', listed))).toEqual([412])
    expect(numbers(pullsNamed('31', listed))).toEqual([31])
    expect(pullsNamed('ch', listed)).toEqual([])
  })

  it('narrows the pull requests waiting by project or title', () => {
    expect(numbers(pullsIn('', listed))).toEqual([412, 31, 4120])
    expect(numbers(pullsIn('uh push', listed))).toEqual([31])
  })

  it('reads the waiting list from its own words', () => {
    expect(waits('review')).toBe(true)
    expect(waits('to rev')).toBe(true)
    expect(waits('pr')).toBe(false)
    expect(waits('to')).toBe(false)
  })

  it('offers only what can be done with a pull request', () => {
    const title = (act: string) => act
    const all = { reviewer: false, approve: true, changes: true, merge: true }
    expect(pullActs('', all, title)).toEqual(['review', 'agent', 'host', 'approve', 'changes', 'merge'])
    expect(pullActs('', { reviewer: true, approve: false, changes: false, merge: false }, title)).toEqual(['review', 'reviewer', 'host'])
    expect(pullActs('mer', all, title)).toEqual(['merge'])
    expect(pullActs('app', all, title)).toEqual(['approve'])
  })
})
