import { expect, it } from 'vitest'
import { projectFile } from './filelink'

it('takes a link to a file of the project and leaves the rest', () => {
  expect(projectFile('a-trier/T52-01-commandes-cms-cassees-dettes.md')).toBe('a-trier/T52-01-commandes-cms-cassees-dettes.md')
  expect(projectFile('./src/App.svelte')).toBe('src/App.svelte')
  expect(projectFile('docs/mon%20plan.md')).toBe('docs/mon plan.md')
  expect(projectFile('README.md')).toBe('README.md')
  for (const href of ['https://github.com/a/b/pull/1', 'mailto:a@b.fr', 'squadron:task/1', '/home/q/x.md', '#titre', '../secret.md', 'a/../../b.md', 'docs/', 'docs'])
    expect(projectFile(href)).toBe('')
})
