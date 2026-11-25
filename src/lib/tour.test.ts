import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

const store = vi.hoisted(() => {
  const data = new Map<string, string>()
  const localStorage = {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, String(v)),
    removeItem: (k: string) => void data.delete(k),
    clear: () => data.clear(),
    key: (i: number) => [...data.keys()][i] ?? null,
    get length() {
      return data.size
    },
  }
  return { data, localStorage }
})

vi.stubGlobal('localStorage', store.localStorage)

const backend = vi.hoisted(() => ({ calls: [] as { cmd: string; args: unknown }[] }))

vi.mock('@tauri-apps/api', () => ({
  core: {
    invoke: async (cmd: string, args: unknown) => {
      backend.calls.push({ cmd, args })
      return null
    },
  },
}))

vi.mock('./agents.svelte', () => {
  const agents = { list: [] as { id: string; project: string }[], current: {} as Record<string, string> }
  return { agents, find: (id: string) => agents.list.find((a) => a.id === id) }
})
vi.mock('./projects.svelte', () => ({ projects: { list: [], current: '', overview: false, usage: false, board: false, pulls: false }, sandbox: { project: null } }))
vi.mock('./settings.svelte', () => ({ settings: { tour_offered: false }, patchSettings: vi.fn(async () => {}) }))

const { agents } = await import('./agents.svelte')
const { projects } = await import('./projects.svelte')
const { STOPS, back, closeEnd, finish, go, leave, next, startTour, tour } = await import('./tour.svelte')
const { DEMO_PATH } = await import('./demo.svelte')

const { invoke } = await import('@tauri-apps/api/core')

const leaked = () => backend.calls.filter((c) => JSON.stringify(c.args ?? {}).includes('"demo:'))

describe('guided tour sandbox', () => {
  beforeAll(() => {
    vi.useFakeTimers()
    projects.list = [{ name: 'webshop', path: '/home/me/webshop' } as never]
    projects.current = '/home/me/webshop'
    store.localStorage.setItem('squadron.project', '/home/me/webshop')
  })

  afterEach(() => {
    backend.calls = []
  })

  it('plays every stop without a demo call reaching Rust', async () => {
    startTour(0)
    expect(projects.list.some((p) => p.path === DEMO_PATH)).toBe(true)
    expect(projects.current).toBe(DEMO_PATH)
    for (let i = 0; i < STOPS.length; i++) {
      go(i)
      await vi.advanceTimersByTimeAsync(8000)
      for (const a of agents.list) await invoke('save_agent', { id: a.id, data: '{}' })
    }
    await invoke('set_cards', { path: DEMO_PATH, cards: [] })
    await invoke('git_changes', { project: DEMO_PATH }).catch(() => {})
    await invoke('list_projects', {})
    expect(leaked()).toEqual([])
    expect(backend.calls.map((c) => c.cmd)).toEqual(['list_projects'])
  })

  it('waits for Allow, then lets the agent go on', async () => {
    go(STOPS.findIndex((s) => s.id === 'perm'))
    await vi.advanceTimersByTimeAsync(3000)
    const stripe = agents.list.find((a) => a.id === 'demo:a3') as unknown as { asks: unknown[]; status: string }
    expect(stripe.status).toBe('waiting')
    stripe.asks = []
    await invoke('answer_permission', { id: 'demo:ask-stripe', decision: { behavior: 'allow' } })
    expect(tour.allowed).toBe(true)
    await vi.advanceTimersByTimeAsync(5000)
    expect(STOPS[tour.i].id).toBe('question')
    back()
    expect(STOPS[tour.i].id).toBe('perm')
    expect(leaked()).toEqual([])
  })

  it('leaves no trace once the tour is left', async () => {
    next()
    leave()
    expect(projects.list.map((p) => p.path)).toEqual(['/home/me/webshop'])
    expect(projects.current).toBe('/home/me/webshop')
    expect(agents.list).toEqual([])
    expect([...store.data.entries()].filter(([k, v]) => k.includes('demo:') || v.includes('demo:'))).toEqual([])
    await invoke('save_agent', { id: 'demo:a1', data: '{}' })
    expect(backend.calls.at(-1)?.cmd).toBe('save_agent')
  })

  it('finishes on the end card, then tears down', () => {
    startTour(0)
    finish()
    expect(tour.end).toBe(true)
    expect(tour.on).toBe(false)
    closeEnd()
    expect(tour.end).toBe(false)
    expect(projects.list.map((p) => p.path)).toEqual(['/home/me/webshop'])
    expect(tour.toast).not.toBe('')
  })
})
