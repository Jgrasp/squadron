import { describe, expect, it } from 'vitest'
import { activeIn, after, everywhere, plan, type Flags } from './plugins'

const id = 'shop@m'
const paths = ['/a', '/b', '/c']
const base = (): Flags => ({ user: { [id]: true }, projects: { '/a': {}, '/b': { [id]: false }, '/c': {} } })

describe('plugin projects', () => {
  it('reads where a plugin is active', () => {
    expect(activeIn(base(), id, paths)).toEqual(['/a', '/c'])
    expect(everywhere(base(), id, paths)).toBe(false)
  })

  it('enables everywhere by clearing local opt-outs', () => {
    const steps = plan(base(), id, paths, 'all')
    expect(steps).toEqual([{ args: ['enable', id, '--scope', 'local'], dir: '/b' }])
    expect(everywhere(after(base(), steps), id, paths)).toBe(true)
  })

  it('treats every project checked as everywhere', () => {
    expect(plan(base(), id, paths, ['/a', '/b', '/c'])).toEqual(plan(base(), id, paths, 'all'))
  })

  it('limits a plugin to some projects', () => {
    const steps = plan(base(), id, paths, ['/b'])
    expect(steps).toEqual([
      { args: ['disable', id, '--scope', 'user'] },
      { args: ['enable', id, '--scope', 'local'], dir: '/b' },
    ])
    expect(activeIn(after(base(), steps), id, paths)).toEqual(['/b'])
  })

  it('opts out a project the team enabled', () => {
    const flags: Flags = { user: { [id]: false }, projects: { '/a': { [id]: true }, '/b': {}, '/c': {} } }
    expect(plan(flags, id, paths, ['/c'])).toEqual([
      { args: ['disable', id, '--scope', 'local'], dir: '/a' },
      { args: ['enable', id, '--scope', 'local'], dir: '/c' },
    ])
  })

  it('turns a plugin off everywhere', () => {
    const flags: Flags = { user: { [id]: false }, projects: { '/a': { [id]: true }, '/b': {}, '/c': {} } }
    const steps = plan(flags, id, paths, 'off')
    expect(activeIn(after(flags, steps), id, paths)).toEqual([])
    expect(plan(flags, id, paths, [])).toEqual(steps)
  })
})
