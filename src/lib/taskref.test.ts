import { describe, expect, it } from 'vitest'
import { codeOf, nameOf, taskId } from './taskref'

describe('taskref', () => {
  it('reads the code at the head of a card title', () => {
    expect(codeOf('T7B — Admin : options produit TMC')).toBe('T7B')
    expect(codeOf('API — sans chiffre')).toBeUndefined()
    expect(nameOf('T7B — Admin : options produit TMC')).toBe('Admin : options produit TMC')
  })

  it('reads the card id of a task link', () => {
    expect(taskId('squadron:task/abb24043-bd0a-4de0-b8a8-8a1247146687')).toBe('abb24043-bd0a-4de0-b8a8-8a1247146687')
  })

})
