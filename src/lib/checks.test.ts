import { describe, expect, it } from 'vitest'
import { isCheck, pullOf } from './checks'
import type { Run } from './pipelines.svelte'
import type { Pull } from './pulls.svelte'

const run = (over: Partial<Run>) => ({ event: 'push', branch: 'main', pull: null, ...over }) as Run
const pulls = [{ number: 12, branch: 'fix/csv', title: 'Fix CSV' }, { number: 13, branch: 'feat/x', title: 'X' }] as Pull[]

describe('checks', () => {
  it('finds the pull request by number, then by head branch', () => {
    expect(pullOf(run({ pull: 13, branch: 'fix/csv' }), pulls)?.title).toBe('X')
    expect(pullOf(run({ branch: 'fix/csv' }), pulls)?.number).toBe(12)
    expect(pullOf(run({ branch: '' }), [{ number: 1, branch: '' } as Pull])).toBeNull()
  })

  it('tells a pull request check from a pipeline', () => {
    expect(isCheck(run({}), pulls)).toBe(false)
    expect(isCheck(run({ pull: 99 }), pulls)).toBe(true)
    expect(isCheck(run({ event: 'merge_request_event' }), pulls)).toBe(true)
    expect(isCheck(run({ branch: 'feat/x' }), pulls)).toBe(true)
  })
})
