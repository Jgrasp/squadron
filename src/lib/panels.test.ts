import { describe, expect, it } from 'vitest'
import { covers } from './panels'

const at = (mode: 'slide' | 'full' | 'bottom', width = 760, height = 40) => ({ mode, width, height })

describe('covers', () => {
  it('lets a panel opening the same way and at least as big cover the open one', () => {
    expect(covers(at('bottom', 760, 55), at('bottom', 760, 40))).toBe(true)
    expect(covers(at('slide', 800), at('slide', 760))).toBe(true)
    expect(covers(at('full'), at('full'))).toBe(true)
  })

  it('does not cover a bigger panel or one opening another way', () => {
    expect(covers(at('bottom', 760, 40), at('bottom', 760, 55))).toBe(false)
    expect(covers(at('slide', 600), at('slide', 760))).toBe(false)
    expect(covers(at('full'), at('slide'))).toBe(false)
    expect(covers(at('slide'), at('bottom'))).toBe(false)
  })
})
