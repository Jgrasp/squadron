import { describe, expect, it } from 'vitest'
import type { Run } from './pipelines.svelte'
import { NO_CHECKS_AFTER, pullOutcome } from './pullwait'
import type { PullWait } from './stream'

const checks: PullWait = { number: 42, until: 'checks', since: 1_000 }
const merged: PullWait = { ...checks, until: 'merged' }
const pull = (state: string, c: string | null) => ({ state, checks: c, url: 'https://forge/pr/42', branch: 'feat' }) as Parameters<typeof pullOutcome>[1]
const run = (fields: Partial<Run>) => ({ status: 'failure', name: 'test', url: 'https://forge/run/7', pull: 42, branch: 'feat', ...fields }) as Run

describe('waiting for a pull request', () => {
  it('tells when the checks end', () => {
    expect(pullOutcome(checks, pull('open', 'pending'), 2_000)).toBeNull()
    expect(pullOutcome(checks, pull('open', 'passing'), 2_000)).toBe('PR #42: all checks green')
    expect(pullOutcome(checks, pull('open', 'failing'), 2_000, [run({ pull: 41, name: 'other' }), run({})])).toBe('PR #42: check test failed: https://forge/run/7')
    expect(pullOutcome(checks, pull('open', 'failing'), 2_000, [run({ pull: null, name: 'lint' })])).toBe('PR #42: check lint failed: https://forge/run/7')
    expect(pullOutcome(checks, pull('open', 'failing'), 2_000)).toBe('PR #42: check failed: https://forge/pr/42')
  })

  it('gives the checks time to appear before saying there are none', () => {
    expect(pullOutcome(checks, pull('open', null), 1_000 + NO_CHECKS_AFTER - 1)).toBeNull()
    expect(pullOutcome(checks, pull('open', null), 1_000 + NO_CHECKS_AFTER)).toBe('PR #42: no checks')
  })

  it('waits for the merge past the checks, and tells a merge or a close in both modes', () => {
    expect(pullOutcome(merged, pull('open', 'passing'), 2_000)).toBeNull()
    expect(pullOutcome(merged, pull('open', null), 10_000_000)).toBeNull()
    expect(pullOutcome(merged, pull('merged', 'passing'), 2_000)).toBe('PR #42 merged')
    expect(pullOutcome(checks, pull('merged', 'pending'), 2_000)).toBe('PR #42 merged')
    expect(pullOutcome(merged, pull('declined', null), 2_000)).toBe('PR #42 closed without merge')
    expect(pullOutcome(checks, pull('closed', 'failing'), 2_000)).toBe('PR #42 closed without merge')
  })
})
