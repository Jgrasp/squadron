import { agents } from './agents.svelte'
import { DEMO, isDemo, touchesDemo } from './demoId'
import { ipc } from './ipc'
import { projects, sandbox, type Project } from './projects.svelte'

export { DEMO, isDemo }

export const DEMO_PATH = 'demo:acme-shop'

type Handler = (cmd: string, args: Record<string, unknown>) => unknown

const ANSWERED = new Set([
  'answer_permission',
  'save_agent',
  'set_agent_options',
  'remote_control',
  'sleep_agent',
  'stop_turn',
  'interrupt_agent',
  'archive_agent',
  'archive_agent_file',
  'delete_agent_file',
  'send_message',
  'context_usage',
])

const LISTS = new Set(['set_cards', 'add_rule', 'remove_rule', 'edit_project'])

let handler: Handler = () => null
let saved: { current: string; overview: boolean; usage: boolean; board: boolean; pulls: boolean } | null = null

function gate() {
  ipc.route = (cmd, args) => {
    if (!touchesDemo(args)) return undefined
    if (LISTS.has(cmd)) return Promise.resolve(projects.list.filter((p) => !isDemo(p.path)))
    if (!ANSWERED.has(cmd)) return Promise.reject(new Error('demo'))
    return Promise.resolve(handler(cmd, args))
  }
}

function ungate() {
  ipc.route = null
}

export const demo = $state({ on: false })

export const scripted = $state({ agent: '', text: '' })

export function demoProject(name: string): Project {
  return {
    name,
    path: DEMO_PATH,
    rules: [],
    cards: [],
    color: '#E0B65B',
    pinned: false,
    tab_closed_at: 0,
    group: '',
    runs: false,
    runs_notify: false,
    runs_hidden: [],
    repository: '',
    review_prompt: '',
    auto_review: false,
    defaults: {},
    exists: true,
    git: true,
    branch: 'main',
  }
}

export function startDemo(name: string, onCommand: Handler) {
  if (demo.on) return
  handler = onCommand
  gate()
  saved = { current: projects.current, overview: projects.overview, usage: projects.usage, board: projects.board, pulls: projects.pulls }
  projects.list = [...projects.list.filter((p) => p.path !== DEMO_PATH), demoProject(name)]
  sandbox.project = projects.list.at(-1) ?? null
  projects.overview = projects.usage = projects.board = projects.pulls = false
  demo.on = true
}

export function clearDemoAgents() {
  agents.list = agents.list.filter((a) => !isDemo(a.id))
  for (const key of Object.keys(agents.current)) if (isDemo(key)) delete agents.current[key]
}

export function stopDemo() {
  if (!demo.on) return
  clearDemoAgents()
  scripted.agent = scripted.text = ''
  sandbox.project = null
  projects.list = projects.list.filter((p) => p.path !== DEMO_PATH)
  if (saved) {
    projects.current = projects.list.some((p) => p.path === saved!.current) ? saved.current : (projects.list[0]?.path ?? '')
    Object.assign(projects, { overview: saved.overview, usage: saved.usage, board: saved.board, pulls: saved.pulls })
  }
  saved = null
  sweepStorage()
  ungate()
  handler = () => null
  demo.on = false
}

function clean(value: unknown): unknown {
  if (Array.isArray(value)) return value.filter((v) => !isDemo(v) && !isDemo((v as { id?: unknown })?.id)).map(clean)
  if (!value || typeof value !== 'object') return value
  return Object.fromEntries(Object.entries(value).filter(([k, v]) => !isDemo(k) && !isDemo(v)).map(([k, v]) => [k, clean(v)]))
}

function sweepStorage() {
  const keys = Array.from({ length: localStorage.length }, (_, i) => localStorage.key(i) ?? '')
  for (const key of keys) {
    try {
      const value = localStorage.getItem(key) ?? ''
      if (key.includes(DEMO) || value.startsWith(DEMO)) {
        localStorage.removeItem(key)
        continue
      }
      if (value.includes(DEMO)) localStorage.setItem(key, JSON.stringify(clean(JSON.parse(value))))
    } catch {}
  }
}
