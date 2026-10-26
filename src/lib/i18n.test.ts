import { afterEach, describe, expect, it } from 'vitest'
import { ago, duration } from './format'
import { english, errorText, parts, setLanguage, t } from './i18n/index.svelte'

afterEach(() => setLanguage('system'))

describe('i18n', () => {
  it('falls back to English', () => {
    expect(t('common.save')).toBe('Save')
    setLanguage('fr')
    expect(t('common.save')).toBe('Enregistrer')
    expect(english('common.save')).toBe('Save')
  })

  it('picks plural forms and formats numbers', () => {
    expect(ago(0, 5 * 60_000)).toBe('5 min ago')
    setLanguage('fr')
    expect(ago(0, 5 * 60_000)).toBe('il y a 5 min')
    expect(duration(65_000)).toBe('1 min 05 s')
  })

  it('splits a message around its values', () => {
    const snippet = (() => {}) as never
    expect(parts('agent.messageOf', { n: 1, total: 2, time: snippet })).toEqual(['Message ', '1', ' of ', '2', ' · ', snippet])
  })

  it('ignores an unknown language', () => {
    setLanguage('de' as never)
    expect(t('common.close')).toBe('Close')
  })

  it('keeps unknown errors as they are', () => {
    setLanguage('fr')
    expect(errorText('boom')).toBe('boom')
    expect(errorText('Unknown agent.')).toBe('Agent inconnu.')
    expect(errorText(new Error('This folder is already the Nova project.'))).toBe('Ce dossier est déjà le projet Nova.')
  })
})
