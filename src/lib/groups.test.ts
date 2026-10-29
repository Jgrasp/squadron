import { describe, expect, it } from 'vitest'
import { contained, inBar, rollup, rows, within } from './groups'
import type { Group, Project } from './projects.svelte'

const group = (id: string, parent = ''): Group => ({ id, name: id, parent })
const project = (name: string, g = '', pinned = false) => ({ name, path: `/p/${name}`, group: g, pinned }) as Project

const groups = [group('clients'), group('ring', 'clients'), group('tools')]
const list = [project('RingApp', 'ring'), project('RingBack', 'ring'), project('Lunetterie', 'clients'), project('squadron', 'tools'), project('domotique')]

describe('rows', () => {
  it('lists groups before projects, nested, then root projects', () => {
    const shown = rows(groups, list, {}, '').map((r) => (r.kind === 'group' ? `g:${r.id}@${r.depth}` : `p:${r.project.name}@${r.depth}`))
    expect(shown).toEqual(['g:clients@0', 'g:ring@1', 'p:RingApp@2', 'p:RingBack@2', 'p:Lunetterie@1', 'g:tools@0', 'p:squadron@1', 'p:domotique@0'])
  })

  it('hides the content of a folded group', () => {
    const shown = rows(groups, list, { clients: true }, '').map((r) => (r.kind === 'group' ? r.id : r.project.name))
    expect(shown).toEqual(['clients', 'tools', 'squadron', 'domotique'])
  })

  it('treats a project of an unknown group as a root project', () => {
    const shown = rows(groups, [project('lost', 'gone')], {}, '').filter((r) => r.kind === 'project')
    expect(shown.map((r) => r.depth)).toEqual([0])
  })

  it('keeps the ancestors of a match and opens folded groups while searching', () => {
    const shown = rows(groups, list, { clients: true, ring: true }, 'ringb').map((r) => (r.kind === 'group' ? r.id : r.project.name))
    expect(shown).toEqual(['clients', 'ring', 'RingBack'])
  })

  it('shows the whole content of a group whose name matches', () => {
    const shown = rows(groups, list, {}, 'tools').map((r) => (r.kind === 'group' ? r.id : r.project.name))
    expect(shown).toEqual(['tools', 'squadron'])
  })
})

describe('tree helpers', () => {
  it('knows what is inside what', () => {
    expect(within(groups, 'ring', 'clients')).toBe(true)
    expect(within(groups, 'clients', 'ring')).toBe(false)
    expect(within(groups, 'clients', 'clients')).toBe(true)
    expect(contained(groups, list, 'clients').map((p) => p.name)).toEqual(['RingApp', 'RingBack', 'Lunetterie'])
  })

  it('adds up what happens inside a group, sub-groups included', () => {
    const stat = (path: string) => ({ working: path === '/p/RingApp' ? 1 : 0, waiting: path === '/p/Lunetterie' ? 2 : 0, failed: path === '/p/RingBack' ? 1 : 0 })
    expect(rollup(groups, list, 'clients', stat)).toEqual({ working: 1, waiting: 2, failed: 1 })
    expect(rollup(groups, list, 'tools', stat)).toEqual({ working: 0, waiting: 0, failed: 0 })
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
