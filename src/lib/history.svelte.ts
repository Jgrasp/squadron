import { agents, selected } from './agents.svelte'
import { projects } from './projects.svelte'

type View = { project: string; agent?: string; overview: boolean; board: boolean; pulls: boolean }

const view = (): View => ({
  project: projects.current,
  agent: selected(projects.current)?.id,
  overview: projects.overview,
  board: projects.board,
  pulls: projects.pulls,
})

const same = (a: View, b: View | null) => JSON.stringify(a) === JSON.stringify(b)

function restore(state: View) {
  if (projects.list.some((p) => p.path === state.project)) projects.current = state.project
  projects.overview = state.overview
  projects.board = state.board
  projects.pulls = state.pulls
  if (agents.list.some((a) => a.id === state.agent && a.project === state.project)) agents.current[state.project] = state.agent!
  history.replaceState(view(), '')
}

export function trackHistory() {
  history.replaceState(view(), '')
  addEventListener('popstate', ({ state }) => state && restore(state))
  $effect.root(() => {
    $effect(() => {
      const next = view()
      if (!same(next, history.state)) history.pushState(next, '')
    })
  })
}
