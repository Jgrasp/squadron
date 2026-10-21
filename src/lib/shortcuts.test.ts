import { describe, expect, it } from 'vitest'
import { binding, combo, pressed, usable } from './shortcuts'

const key = (key: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
})

describe('shortcuts', () => {
  it('reads a key combination', () => {
    expect(combo(key('e', { ctrlKey: true }))).toBe('Ctrl+E')
    expect(combo(key('K', { ctrlKey: true, shiftKey: true }))).toBe('Ctrl+Shift+K')
    expect(combo(key('Escape'))).toBe('Escape')
    expect(combo(key('Control', { ctrlKey: true }))).toBe('')
  })

  it('finds the action, with overrides first', () => {
    expect(pressed(key('d', { ctrlKey: true }), {})).toBe('diff')
    expect(pressed(key('d', { ctrlKey: true }), { diff: 'Alt+D' })).toBeUndefined()
    expect(pressed(key('d', { altKey: true }), { diff: 'Alt+D' })).toBe('diff')
    expect(pressed(key('k', { ctrlKey: true, shiftKey: true }), {})).toBeUndefined()
    expect(binding({}, 'interrupt')).toBe('Escape')
  })

  it('refuses keys that would fire while typing', () => {
    expect(usable('Ctrl+E')).toBe(true)
    expect(usable('F5')).toBe(true)
    expect(usable('E')).toBe(false)
    expect(usable('Shift+E')).toBe(false)
  })
})
