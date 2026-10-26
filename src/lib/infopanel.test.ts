import { describe, expect, it } from 'vitest'
import { panelDocument, panelHeight, strip } from './infopanel'

describe('info panel', () => {
  it('removes scripts and the tags that could reach out or override the policy', () => {
    const html = strip(
      '<p>60/80</p><script>alert(1)</script><SCRIPT src="x.js"></SCRIPT><meta http-equiv="refresh" content="0;url=https://x"><base href="https://x/"><link rel="stylesheet" href="https://x/a.css"><iframe src="https://x"></iframe><object data="x"></object><embed src="x"><script>never closed',
    )
    expect(html).toBe('<p>60/80</p>')
  })

  it('puts the content security policy before the agent html', () => {
    const doc = panelDocument('<meta http-equiv="Content-Security-Policy" content="default-src *"><b>ok</b>', { '--text': '#eee', '--evil': 'x' })
    expect(doc.indexOf("default-src 'none'")).toBeLessThan(doc.indexOf('<b>ok</b>'))
    expect(doc).not.toContain('default-src *')
    expect(doc).toContain('--text:#eee')
    expect(doc).not.toContain('--evil')
  })

  it('bounds the height', () => {
    expect(panelHeight(undefined)).toBe(64)
    expect(panelHeight('nope')).toBe(64)
    expect(panelHeight(4)).toBe(24)
    expect(panelHeight(5000)).toBe(240)
    expect(panelHeight(99.6)).toBe(100)
  })
})
