import { describe, expect, it } from 'vitest'
import { clip, plain, said } from './shell'

describe('shell output', () => {
  it('keeps the text a terminal shows', () => {
    expect(plain('\x1b[32m✓\x1b[0m ok\r\n10%\r50%\r100%\r\n\x1b]0;title\x07done\r\n')).toBe('✓ ok\n100%\ndone')
  })

  it('cuts the middle of a long output', () => {
    const long = 'a'.repeat(20_000) + 'b'.repeat(20_000)
    const cut = clip(long)
    expect(cut.length).toBeLessThan(31_000)
    expect(cut.startsWith('a')).toBe(true)
    expect(cut.endsWith('b')).toBe(true)
  })

  it('tells Claude what ran and how it ended', () => {
    expect(said([{ command: 'pnpm test', output: '1 failed', code: 1, state: 'done' }])).toBe(
      '<bash-input>pnpm test</bash-input>\n<bash-stdout>1 failed</bash-stdout><bash-stderr>Exit code 1</bash-stderr>',
    )
    expect(said([{ command: 'sleep 9', output: '', code: null, state: 'stopped' }])).toContain('Stopped by the user')
  })
})
