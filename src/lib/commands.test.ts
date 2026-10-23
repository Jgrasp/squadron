import { describe, expect, it } from 'vitest'
import { choices, families, familyOf, highlight, modelName, options, parse, versionsOf } from './commands'

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

  it('groups the models of Claude Code by family, latest first', () => {
    const models = [
      { value: 'default', displayName: 'Default (recommended)' },
      { value: 'opus', displayName: 'Opus 5.5' },
      { value: 'haiku', displayName: 'Haiku 5.5' },
      { value: 'claude-haiku-4-5-20251001', displayName: 'Haiku 4.5' },
      { value: 'claude-opus-4-7', displayName: 'Opus 4.7' },
    ]
    expect(families(models)).toEqual(['opus', 'haiku'])
    expect(families([])).toEqual(['opus', 'sonnet', 'haiku'])
    expect(familyOf('claude-opus-4-7', models)).toEqual('opus')
    expect(familyOf('sonnet', [])).toEqual('sonnet')
    expect(versionsOf('haiku', models).map((m) => m.value)).toEqual(['haiku', 'claude-haiku-4-5-20251001'])
    expect(modelName('claude-opus-4-7', models)).toEqual('Opus 4.7')
    expect(modelName('sonnet', [])).toEqual('Sonnet')
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
