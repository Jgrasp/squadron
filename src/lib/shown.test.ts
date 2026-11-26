import { describe, expect, it, vi } from 'vitest'

vi.stubGlobal('localStorage', { getItem: () => null, setItem: () => {}, removeItem: () => {} })
vi.stubGlobal('addEventListener', () => {})
vi.mock('@tauri-apps/api/core', () => ({ invoke: async () => null }))
vi.mock('@tauri-apps/api/event', () => ({ listen: async () => () => {} }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ label: 'main', listen: async () => () => {} }) }))
vi.mock('./projects.svelte', () => ({ projects: { list: [], current: '/p', overview: false, usage: false, board: false, pulls: false }, addRule: vi.fn(), defaultsFor: vi.fn(), remove: vi.fn(), setCards: vi.fn() }))

const { agents, archivable, shownAgent } = await import('./agents.svelte')

const agent = (id: string) => ({ id, project: '/p', status: 'done', asks: [] }) as never

describe('the agent on screen', () => {
  it('is none once every agent of the project is archived, even with a stale current id', () => {
    agents.list = []
    agents.current['/p'] = 'archived'
    expect(shownAgent()).toBeUndefined()
  })

  it('is the first agent when the current id is stale, and that agent is not archivable', () => {
    agents.list = [agent('a'), agent('b')]
    agents.current['/p'] = 'archived'
    expect(shownAgent()).toBe('a')
    expect(archivable(agents.list[0])).toBe(false)
    expect(archivable(agents.list[1])).toBe(true)
  })
})
