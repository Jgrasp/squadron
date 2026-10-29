import { describe, expect, it } from 'vitest'
import { match, words, type World } from './spotlight'
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
