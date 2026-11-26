import { describe, expect, it } from 'vitest'
import { settle } from './autoLaunch'
import type { Model } from './commands'
import type { Options } from './stream'

const options = (model: string, effort: Options['effort']): Options => ({ model, effort, mode: 'default' })
type Pick = { model: string; effort: string }
const catalog = [{ value: 'haiku', supportedEffortLevels: ['low', 'medium', 'high'] }] as Model[]

describe('settle', () => {
  it('takes Haiku’s pick for the auto sides only', () => {
    expect(settle(options('auto', 'auto'), { model: 'sonnet', effort: 'medium' }, catalog)).toMatchObject({ model: 'sonnet', effort: 'medium' })
    expect(settle(options('opus', 'auto'), { model: 'sonnet', effort: 'low' }, catalog)).toMatchObject({ model: 'opus', effort: 'low' })
    expect(settle(options('auto', 'max'), { model: 'sonnet', effort: 'low' }, catalog)).toMatchObject({ model: 'sonnet', effort: 'max' })
  })

  it('falls back to Opus · High without a pick', () => {
    expect(settle(options('auto', 'auto'), null, catalog)).toMatchObject({ model: 'opus', effort: 'high' })
  })

  it('pairs an auto advisor with the model it ends up with', () => {
    const advised = (model: string, pick: Pick) => settle({ ...options(model, 'auto'), advisor: 'auto' }, pick, catalog).advisor
    expect(advised('auto', { model: 'haiku', effort: 'low' })).toBe('sonnet')
    expect(advised('auto', { model: 'sonnet', effort: 'medium' })).toBe('opus')
    expect(advised('auto', { model: 'opus', effort: 'high' })).toBe('none')
    expect(advised('sonnet', { model: 'opus', effort: 'high' })).toBe('opus')
    expect(settle({ ...options('auto', 'auto'), advisor: 'fable' }, { model: 'sonnet', effort: 'low' }, catalog).advisor).toBe('fable')
  })

  it('keeps the effort within what the model supports', () => {
    expect(settle(options('auto', 'max'), { model: 'haiku', effort: 'low' }, catalog)).toMatchObject({ model: 'haiku', effort: 'high' })
  })
})
