import { expect, it } from 'vitest'

it('has no two source files whose names differ only by case, which macOS cannot tell apart', () => {
  const names = Object.keys(import.meta.glob('/src/**/*')).map((file) => file.replace(/\.ts$/, '').toLowerCase())
  expect(names.length).toBeGreaterThan(50)
  expect(names.filter((name, i) => names.indexOf(name) !== i)).toEqual([])
})
