import { appendFileSync, mkdtempSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { instanceOf } from './app.mjs'
import { answer, since, wrap } from './drive.mjs'

describe('dev instance names', () => {
  it('names an instance after its worktree, or after its checkout folder', () => {
    expect(instanceOf('/home/q/Projects/squadron/.claude/worktrees/pulls-review')).toBe('pulls-review')
    expect(instanceOf('/home/q/Projects/squadron-browser')).toBe('squadron-browser')
    expect(instanceOf('/home/q/Projects/Squadron Fix_2')).toBe('squadron-fix-2')
  })
})

describe('drive scripts', () => {
  it('leave their value or their error under their id', async () => {
    const window: { __drive?: object[] } = {}
    await new Function('window', `return ${wrap('return 1 + 1', 'a1').slice(1)}`)(window)
    await new Function('window', `return ${wrap('throw new Error("no button")', 'a2').slice(1)}`)(window)
    const lines = `${JSON.stringify(window.__drive)}\n`
    expect(answer(lines, 'a1')).toEqual({ id: 'a1', value: 2 })
    expect(answer(lines, 'a2')?.error).toMatch(/^Error: no button/)
    expect(answer(lines, 'a3')).toBeUndefined()
  })

  it('find their answer after an earlier answer with multi-byte text', () => {
    const file = join(mkdtempSync(join(tmpdir(), 'drive-')), 'answers')
    writeFileSync(file, `${JSON.stringify([{ id: 'b1', value: 'dev· ünïcødé 🚀' }])}\n`)
    const from = statSync(file).size
    appendFileSync(file, `${JSON.stringify([{ id: 'b2', value: 'ok' }])}\n`)
    expect(answer(since(file, from), 'b2')).toEqual({ id: 'b2', value: 'ok' })
  })
})
