import { describe, expect, it } from 'vitest'
import { instanceOf } from './app.mjs'

describe('dev instance names', () => {
  it('names an instance after its worktree, or after its checkout folder', () => {
    expect(instanceOf('/home/q/Projects/squadron/.claude/worktrees/pulls-review')).toBe('pulls-review')
    expect(instanceOf('/home/q/Projects/squadron-browser')).toBe('squadron-browser')
    expect(instanceOf('/home/q/Projects/Squadron Fix_2')).toBe('squadron-fix-2')
  })
})
