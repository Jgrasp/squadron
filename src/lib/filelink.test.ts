import { expect, it } from 'vitest'
import { fenced, projectFile, within, windowLabel } from './filelink'

it('takes a link to a file of the project and leaves the rest', () => {
  expect(projectFile('a-trier/T52-01-commandes-cms-cassees-dettes.md')).toBe('a-trier/T52-01-commandes-cms-cassees-dettes.md')
  expect(projectFile('./src/App.svelte')).toBe('src/App.svelte')
  expect(projectFile('docs/mon%20plan.md')).toBe('docs/mon plan.md')
  expect(projectFile('README.md')).toBe('README.md')
  for (const href of ['https://github.com/a/b/pull/1', 'mailto:a@b.fr', 'squadron:task/1', '/home/q/x.md', '#titre', '../secret.md', 'a/../../b.md', 'docs/', 'docs'])
    expect(projectFile(href)).toBe('')
})

it('fences a file with more backticks than it holds', () => {
  expect(fenced('a', 'php')).toBe('```php\na\n```')
  expect(fenced('x\n````md\ny\n````', 'markdown')).toBe('`````markdown\nx\n````md\ny\n````\n`````')
})

it('gives one window label per file', () => {
  expect(windowLabel('/p', 'a.md')).toMatch(/^file-[0-9a-z]+$/)
  expect(windowLabel('/p', 'a.md')).toBe(windowLabel('/p', 'a.md'))
  expect(windowLabel('/p', 'a.md')).not.toBe(windowLabel('/p', 'b.md'))
})

it('takes an absolute path back to the project, and leaves the files outside it', () => {
  expect(within('/home/q/mercure2/.ai-specs/BAT-809/a-trier/T52.md', '/home/q/mercure2')).toBe('.ai-specs/BAT-809/a-trier/T52.md')
  expect(within('a-trier/T52.md', '/home/q/mercure2')).toBe('a-trier/T52.md')
  expect(within('/home/q/mercure2-old/x.md', '/home/q/mercure2')).toBe('')
  expect(within('/tmp/x.md', '')).toBe('')
})
