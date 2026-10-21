import { describe, expect, it } from 'vitest'
import { choices, highlight, options, parse } from './commands'

describe('commands', () => {
  it('reads the choices of an argument hint', () => {
    expect(choices('[on|off]')).toEqual(['on', 'off'])
    expect(choices('<low|medium|high|xhigh|max|auto|ultracode [on|off]>')).toEqual(['low', 'medium', 'high', 'xhigh', 'max', 'auto', 'ultracode'])
    expect(choices('[reconnect|enable|disable [<server>|all]]')).toEqual(['reconnect', 'enable', 'disable'])
    expect(choices('[auto|<tokens>]')).toEqual(['auto'])
    expect(choices('lite|full|ultra')).toEqual(['lite', 'full', 'ultra'])
    expect(choices('<model>')).toEqual([])
    expect(choices('[interval] [prompt]')).toEqual([])
    expect(choices('<optional custom summarization instructions>')).toEqual([])
    expect(choices('key=value')).toEqual([])
    expect(choices()).toEqual([])
  })

  it('offers the models of the session for /model', () => {
    const catalog = { commands: [], models: [{ value: 'sonnet', displayName: 'Sonnet 5.5', description: 'Fast' }], styles: [] }
    expect(options({ name: 'model', description: '', argumentHint: '<model>' }, catalog)).toEqual([{ value: 'sonnet', label: 'Sonnet 5.5', detail: 'Fast' }])
    expect(options({ name: 'fast', description: '', argumentHint: '[on|off]' })).toEqual([{ value: 'on' }, { value: 'off' }])
  })

  it('parses and highlights commands', () => {
    expect(parse('/model  sonnet ')).toEqual({ name: 'model', args: 'sonnet' })
    expect(parse('/compact')).toEqual({ name: 'compact', args: '' })
    expect(parse('hello /model')).toBeNull()
    expect(highlight('try /model then /nope or /fast', new Set(['model', 'fast']))).toEqual([
      { text: 'try ', command: false },
      { text: '/model', command: true },
      { text: ' then /nope or ', command: false },
      { text: '/fast', command: true },
    ])
  })
})
