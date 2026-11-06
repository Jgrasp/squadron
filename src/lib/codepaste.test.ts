import { describe, expect, it } from 'vitest'
import { codeLanguage, fenceAt, fences, guessLanguage } from './codepaste'

describe('codepaste', () => {
  it('guesses common languages', () => {
    expect(guessLanguage("function refreshImports() {\n  const ss = SpreadsheetApp.getActive();\n  IMPORTS.forEach(({ sheet }) => {});\n}")).toBe('javascript')
    expect(guessLanguage('const a: string = x\ninterface Foo {\n  b: number\n}')).toBe('typescript')
    expect(guessLanguage('<?php\nnamespace App\\Entity;\n\nfinal class A {\n  public function b() { return $this->c(); }\n}')).toBe('php')
    expect(guessLanguage('def main():\n    import os\n    print(self.x)')).toBe('python')
    expect(guessLanguage('SELECT id, name\nFROM users u\nLEFT JOIN orders o ON o.user_id = u.id\nWHERE o.total > 10')).toBe('sql')
    expect(guessLanguage('{\n  "a": 1\n}')).toBe('json')
    expect(guessLanguage('pub fn main() {\n    let mut v = Vec::new();\n}')).toBe('rust')
  })

  it('leaves prose alone', () => {
    expect(guessLanguage("Salut,\nest-ce que tu peux regarder le bug d'hier ?\nMerci")).toBe('')
    expect(codeLanguage('const a = 1; const b = () => a;', '')).toBe('')
  })

  it('trusts the editor language first', () => {
    expect(codeLanguage('echo a\necho b', '{"version":1,"mode":"shellscript"}')).toBe('bash')
    expect(codeLanguage('hello\nworld', '{"mode":"plaintext"}')).toBe('')
  })

  it('fences on its own lines, never inside a fence', () => {
    expect(fenceAt('Voici', 5, 5, 'a\nb\n', 'js')).toBe('\n```js\na\nb\n```\n')
    expect(fenceAt('Voici\n\nfin', 6, 6, 'a\nb', 'js')).toBe('```js\na\nb\n```')
    expect(fenceAt('```js\n', 6, 6, 'a\nb', 'js')).toBe('')
  })

  it('splits prose and code', () => {
    expect(fences('Avant\n```js\nx()\n```\nAprès')).toEqual([
      { code: false, text: 'Avant' },
      { code: true, text: '```js\nx()\n```' },
      { code: false, text: 'Après' },
    ])
    expect(fences('  rien\n')).toEqual([{ code: false, text: '  rien\n' }])
  })
})
