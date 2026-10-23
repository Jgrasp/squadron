import { readdirSync } from 'node:fs'
import { join } from 'node:path'
import { expect, it } from 'vitest'

it('has no two source files whose names differ only by case, which macOS cannot tell apart', () => {
  const walk = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true }).flatMap((entry) => (entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)]))
  const names = walk('src').map((file) => file.replace(/\.ts$/, '').toLowerCase())
  expect(names.filter((name, i) => names.indexOf(name) !== i)).toEqual([])
})
