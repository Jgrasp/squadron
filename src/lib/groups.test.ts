import { describe, expect, it } from 'vitest'
import { descendants, EMPTY, inBar, inTab, matching, sortProjects, total, within, type Tally } from './groups'
import type { Group, Project } from './projects.svelte'

const group = (id: string, parent = ''): Group => ({ id, name: id, parent })
const project = (name: string, g = '', pinned = false) => ({ name, path: `/p/${name}`, group: g, pinned }) as Project

const groups = [group('clients'), group('ring', 'clients'), group('tools')]
const list = [project('RingApp', 'ring'), project('RingBack', 'ring'), project('Lunetterie', 'clients'), project('squadron', 'tools'), project('domotique')]

const tally = (data: Record<string, Partial<Tally>>) => (path: string) => ({ ...EMPTY, ...data[path] })
const names = (projects: Project[]) => projects.map((p) => p.name)

describe('tabs', () => {
  it('keeps a group, its sub-groups, or the projects without a group', () => {
    expect(names(list.filter((p) => inTab(groups, p, 'clients')))).toEqual(['RingApp', 'RingBack', 'Lunetterie'])
    expect(names(list.filter((p) => inTab(groups, p, 'ring')))).toEqual(['RingApp', 'RingBack'])
    expect(names(list.filter((p) => inTab(groups, p, '-')))).toEqual(['domotique'])
    expect(list.filter((p) => inTab(groups, p, ''))).toHaveLength(5)
  })

  it('treats a project of an unknown group as one without a group', () => {
    expect(inTab(groups, project('lost', 'gone'), '-')).toBe(true)
  })

  it('lists every sub-group with its path under the tab', () => {
    const deep = [...groups, group('ringapp', 'ring')]
    expect(descendants(deep, 'clients')).toEqual([
      { id: 'ring', path: ['ring'] },
      { id: 'ringapp', path: ['ring', 'ringapp'] },
    ])
    expect(descendants(deep, 'tools')).toEqual([])
  })

  it('finds a project by its name or the name of one of its groups', () => {
    expect(names(matching(groups, list, 'ringb'))).toEqual(['RingBack'])
    expect(names(matching(groups, list, 'CLIENTS'))).toEqual(['RingApp', 'RingBack', 'Lunetterie'])
    expect(matching(groups, list, '  ')).toBe(list)
  })

  it('adds up the agents of a list of projects', () => {
    const sum = total(list, tally({ '/p/RingApp': { working: 1, agents: 2, last: 5 }, '/p/squadron': { waiting: 2, agents: 2, last: 9 } }))
    expect(sum).toEqual({ ...EMPTY, working: 1, waiting: 2, agents: 4, last: 9 })
  })
})

describe('sortProjects', () => {
  const pinned = [project('b', '', true), project('a'), project('c', '', true)]
  const data = tally({ '/p/a': { last: 30, agents: 1 }, '/p/b': { last: 10, failed: 1, agents: 3 }, '/p/c': { last: 20, working: 1, agents: 1 } })

  it('sorts by name, activity, attention, open agents or keeps the manual order', () => {
    expect(names(sortProjects(pinned, 'name', data))).toEqual(['a', 'b', 'c'])
    expect(names(sortProjects(pinned, 'activity', data))).toEqual(['a', 'c', 'b'])
    expect(names(sortProjects(pinned, 'attention', data))).toEqual(['b', 'c', 'a'])
    expect(names(sortProjects(pinned, 'agents', data))).toEqual(['b', 'a', 'c'])
    expect(names(sortProjects(pinned, 'manual', data))).toEqual(['b', 'a', 'c'])
  })

  it('reverses the order, then puts the pinned projects first', () => {
    expect(names(sortProjects(pinned, 'name', data, true))).toEqual(['c', 'b', 'a'])
    expect(names(sortProjects(pinned, 'name', data, false, true))).toEqual(['b', 'c', 'a'])
    expect(names(sortProjects(pinned, 'name', data, true, true))).toEqual(['c', 'b', 'a'])
  })
})

describe('tree helpers', () => {
  it('knows what is inside what', () => {
    expect(within(groups, 'ring', 'clients')).toBe(true)
    expect(within(groups, 'clients', 'ring')).toBe(false)
    expect(within(groups, 'clients', 'clients')).toBe(true)
  })
})

describe('inBar', () => {
  const now = 100 * 3_600_000
  const idle = { busy: false, last: 0 }

  it('keeps pinned and current projects', () => {
    expect(inBar({ path: '/a', pinned: true }, '', idle, now, 4)).toBe(true)
    expect(inBar({ path: '/a', pinned: false }, '/a', idle, now, 4)).toBe(true)
  })

  it('brings in a project with a busy agent or recent activity, drops it after the delay', () => {
    expect(inBar({ path: '/a', pinned: false }, '', { busy: true, last: 0 }, now, 4)).toBe(true)
    expect(inBar({ path: '/a', pinned: false }, '', { busy: false, last: now - 3 * 3_600_000 }, now, 4)).toBe(true)
    expect(inBar({ path: '/a', pinned: false }, '', { busy: false, last: now - 5 * 3_600_000 }, now, 4)).toBe(false)
    expect(inBar({ path: '/a', pinned: false }, '', idle, now, 4)).toBe(false)
  })

  it('keeps pinned and current only when the delay is zero', () => {
    expect(inBar({ path: '/a', pinned: false }, '', { busy: true, last: now }, now, 0)).toBe(false)
    expect(inBar({ path: '/a', pinned: true }, '', idle, now, 0)).toBe(true)
  })
})
