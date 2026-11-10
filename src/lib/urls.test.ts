import { describe, expect, it } from 'vitest'
import { splitUrls, urlsIn } from './urls'

describe('urlsIn', () => {
  it('leaves trailing punctuation and an unopened parenthesis out of the link', () => {
    expect(urlsIn('voir https://github.com/nektos/act. Et (https://a.dev/x) puis https://en.wikipedia.org/wiki/Act_(film)!').map((u) => u.url)).toEqual([
      'https://github.com/nektos/act',
      'https://a.dev/x',
      'https://en.wikipedia.org/wiki/Act_(film)',
    ])
  })

  it('ignores a bare scheme', () => {
    expect(urlsIn('https:// rien')).toEqual([])
  })
})

describe('splitUrls', () => {
  it('cuts the text around each link', () => {
    expect(splitUrls('Je pensais à ça :\nhttps://github.com/nektos/act')).toEqual([
      { text: 'Je pensais à ça :\n' },
      { text: 'https://github.com/nektos/act', url: 'https://github.com/nektos/act' },
    ])
  })
})
