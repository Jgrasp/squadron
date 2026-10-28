import { describe, expect, it } from 'vitest'
import { fileName } from './artifacts'

describe('fileName', () => {
  const artifact = { id: 'a', content: '', version: 1, at: 0 }

  it('turns the title into a file name with the extension of its kind', () => {
    expect(fileName({ ...artifact, title: 'Plan de migration : étape 2', kind: 'markdown' })).toBe('plan-de-migration-etape-2.md')
    expect(fileName({ ...artifact, title: 'Schéma', kind: 'svg' })).toBe('schema.svg')
  })

  it('falls back to artifact when nothing is left of the title', () => {
    expect(fileName({ ...artifact, title: '« »', kind: 'html' })).toBe('artifact.html')
  })
})
