import { describe, expect, it } from 'vitest'
import { copiedFiles } from './paste'

describe('copiedFiles', () => {
  it('turns file URIs copied from a file manager into paths', () => {
    expect(copiedFiles('file:///home/me/My%20doc.pdf\r\nfile:///tmp/a.png\r\n')).toEqual(['/home/me/My doc.pdf', '/tmp/a.png'])
  })

  it('leaves web links and empty clipboards to the text paste', () => {
    expect(copiedFiles('https://example.com/a.png')).toEqual([])
    expect(copiedFiles('')).toEqual([])
  })
})
