import { describe, expect, it } from 'vitest'
import { citedFiles } from './mention.svelte'

const files = new Set(['DESIGN.md', 'src/', 'src/lib/', 'src/lib/app.css', 'my notes.md'])

describe('citedFiles', () => {
  it('finds files and folders the project has, and only those', () => {
    const text = 'look at @DESIGN.md, @src/lib and @desgin.md'
    expect(citedFiles(text, files)).toEqual([
      { from: 8, to: 18, path: 'DESIGN.md' },
      { from: 20, to: 28, path: 'src/lib/' },
    ])
  })

  it('reads quoted paths and ignores an @ inside a word', () => {
    expect(citedFiles('@"my notes.md" me@DESIGN.md', files)).toEqual([{ from: 0, to: 14, path: 'my notes.md' }])
  })
})
