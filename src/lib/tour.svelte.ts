import { agents, find } from './agents.svelte'
import { DEMO_PATH, clearDemoAgents, demo, scripted, startDemo, stopDemo } from './demo.svelte'
import { isDemo } from './demoId'
import { t, type Key } from './i18n/index.svelte'
import { projects } from './projects.svelte'
import { patchSettings, settings } from './settings.svelte'
import type { Action } from './shortcuts'
import { newAgent, type Agent } from './stream'
import { END, EVENTS, GATE, SEGMENTS, type Director } from './tourScript'

export const CHAPTERS = ['projects', 'agents', 'turn', 'tools', 'kanban', 'pulls', 'find', 'usage', 'settings', 'extensions'] as const

export type Chapter = (typeof CHAPTERS)[number]

export type Stop = { id: string; ch: Chapter; el?: string; key?: Action; you?: true; seconds: number }

export const STOPS: Stop[] = [
  { id: 'tabs', ch: 'projects', el: `nav [data-item="${DEMO_PATH}"]`, seconds: 12 },
  { id: 'panel', ch: 'projects', el: '[data-projects-toggle]', key: 'projects', seconds: 10 },
  { id: 'overview', ch: 'projects', el: 'nav .tab.overview', seconds: 12 },
  { id: 'add', ch: 'projects', el: 'nav .add', seconds: 10 },
  { id: 'list', ch: 'agents', el: 'aside .list[role="tabpanel"]', seconds: 12 },
  { id: 'new', ch: 'agents', el: 'aside button.new', key: 'newAgent', seconds: 15 },
  { id: 'stream', ch: 'agents', el: 'section.conversation', seconds: 15 },
  { id: 'reviews', ch: 'agents', el: 'aside .tabs[role="tablist"] [role="tab"]:last-child', seconds: 12 },
  { id: 'fork', ch: 'agents', el: '[data-item="demo:a2"]', seconds: 10 },
  { id: 'copper', ch: 'turn', el: '[data-item="demo:a3"]', key: 'jump', seconds: 10 },
  { id: 'perm', ch: 'turn', el: 'section.conversation section.card', you: true, seconds: 15 },
  { id: 'question', ch: 'turn', el: 'section.conversation', seconds: 12 },
  { id: 'secrets', ch: 'turn', el: 'section.conversation', seconds: 10 },
  { id: 'composer', ch: 'turn', el: '.bar:has(.composer-field)', seconds: 12 },
  { id: 'options', ch: 'tools', el: 'button.settings', seconds: 10 },
  { id: 'context', ch: 'tools', el: 'button.gauge', seconds: 10 },
  { id: 'diff', ch: 'tools', el: '.tools [data-item="diff"]', key: 'diff', seconds: 15 },
  { id: 'editor', ch: 'tools', el: '.tools [data-item="editor"]', key: 'editor', seconds: 10 },
  { id: 'terminal', ch: 'tools', el: '.tools [data-item="terminal"]', key: 'terminal', seconds: 10 },
  { id: 'browser', ch: 'tools', el: '.tools [data-item="browser"]', seconds: 12 },
  { id: 'artifacts', ch: 'tools', el: '[data-item="artifacts"]', seconds: 10 },
  { id: 'subagents', ch: 'tools', el: '[data-item="subagents"]', seconds: 10 },
  { id: 'pipelines', ch: 'tools', el: '.tools [data-item="runs"]', seconds: 10 },
  { id: 'chips', ch: 'tools', el: '[data-item="branch"]', seconds: 10 },
  { id: 'board', ch: 'kanban', el: '.tools [data-item="kanban"]', key: 'kanban', seconds: 12 },
  { id: 'card', ch: 'kanban', el: '.tools [data-item="kanban"]', seconds: 10 },
  { id: 'prbtn', ch: 'pulls', el: '.prbtn', seconds: 10 },
  { id: 'prpage', ch: 'pulls', el: '.prbtn', seconds: 12 },
  { id: 'review', ch: 'pulls', el: '.prbtn', seconds: 15 },
  { id: 'openpr', ch: 'pulls', el: '[data-item="pull"]', seconds: 10 },
  { id: 'merge', ch: 'pulls', el: '.prbtn', seconds: 8 },
  { id: 'spotlight', ch: 'find', key: 'search', seconds: 10 },
  { id: 'keys', ch: 'find', seconds: 8 },
  { id: 'limits', ch: 'usage', el: '.limits', seconds: 8 },
  { id: 'usage', ch: 'usage', el: '.usebtn', seconds: 10 },
  { id: 'sprojects', ch: 'settings', seconds: 12 },
  { id: 'sagents', ch: 'settings', seconds: 10 },
  { id: 'sclaude', ch: 'settings', seconds: 10 },
  { id: 'splugins', ch: 'settings', seconds: 12 },
  { id: 'shosting', ch: 'settings', seconds: 8 },
  { id: 'snotify', ch: 'settings', seconds: 10 },
  { id: 'slayout', ch: 'settings', seconds: 10 },
  { id: 'sagain', ch: 'settings', seconds: 8 },
  { id: 'discover', ch: 'extensions', seconds: 10 },
  { id: 'inapp', ch: 'extensions', el: 'aside .list[role="tabpanel"]', seconds: 10 },
]

export const titleOf = (stop: Stop) => t(`tour.${stop.id}Title` as Key)
export const bodyOf = (stop: Stop) => t(`tour.${stop.id}Body` as Key)
export const chapterName = (ch: Chapter) => t(`tour.chapter_${ch}` as Key)

export const minutes = (stops: Stop[] = STOPS) => Math.max(1, Math.round((stops.reduce((a, s) => a + s.seconds, 0) + 20) / 60))

const SEEN = 'squadron.tour.seen'

function readSeen(): string[] {
  try {
    const list = JSON.parse(localStorage.getItem(SEEN) ?? '[]')
    return Array.isArray(list) ? list : []
  } catch {
    return []
  }
}

export const tour = $state({
  on: false,
  i: 0,
  confirm: false,
  end: false,
  allowed: false,
  denied: false,
  welcome: false,
  toast: '',
  seen: readSeen(),
})

function remember(id: string) {
  if (tour.seen.includes(id)) return
  tour.seen = [...tour.seen, id]
  try {
    localStorage.setItem(SEEN, JSON.stringify(tour.seen))
  } catch {}
}

export function forgetSeen() {
  tour.seen = []
  try {
    localStorage.removeItem(SEEN)
  } catch {}
}

export const current = () => STOPS[tour.i]

const still = () => typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches

let timers: ReturnType<typeof setTimeout>[] = []
let typers: ReturnType<typeof setInterval>[] = []
let live = false

function clearTimers() {
  timers.forEach(clearTimeout)
  typers.forEach(clearInterval)
  timers = []
  typers = []
}

function typeInto(full: string, ms: number | undefined, set: (text: string) => void) {
  if (!live || !ms || still()) return set(full)
  let shown = 0
  const step = Math.max(1, Math.ceil(full.length / (ms / 30)))
  set('')
  const timer = setInterval(() => {
    shown = Math.min(full.length, shown + step)
    set(full.slice(0, shown))
    if (shown >= full.length) clearInterval(timer)
  }, 30)
  typers.push(timer)
}

function lastAgentEntry(agent: Agent) {
  const last = agent.entries.at(-1)
  if (last?.kind === 'agent') return last
  agent.entries.push({ kind: 'agent', blocks: [] })
  return agent.entries.at(-1) as Extract<Agent['entries'][number], { kind: 'agent' }>
}

const director: Director = {
  agent(id, name, status, options) {
    if (find(id)) return
    const agent = newAgent(id, DEMO_PATH, name, { model: 'opus', effort: 'high', mode: 'default', ...options })
    agent.status = status
    agent.action = ''
    agent.context = { max: 200_000, threshold: 160_000 }
    agent.tokens = 18_000
    agents.list = [...agents.list, agent]
  },
  user(id, text) {
    find(id)?.entries.push({ kind: 'user', text, at: Date.now() })
  },
  say(id, key, text, typing) {
    const agent = find(id)
    if (!agent) return
    const entry = lastAgentEntry(agent)
    entry.blocks.push({ type: 'text', text: '', key })
    const block = entry.blocks.at(-1) as { text: string }
    typeInto(text, typing, (shown) => (block.text = shown))
  },
  call(id, call, name, input) {
    const agent = find(id)
    if (!agent) return
    const entry = lastAgentEntry(agent)
    const last = entry.blocks.at(-1)
    if (last?.type === 'tools') last.calls.push({ id: call, name, input })
    else entry.blocks.push({ type: 'tools', calls: [{ id: call, name, input }] })
    agent.status = 'working'
  },
  result(id, call, text) {
    const agent = find(id)
    if (!agent) return
    for (const entry of agent.entries)
      if (entry.kind === 'agent')
        for (const block of entry.blocks) if (block.type === 'tools') for (const c of block.calls) if (c.id === call) c.result = { text, error: false }
  },
  ask(id, ask, tool, input, call) {
    const agent = find(id)
    if (!agent || agent.asks.some((a) => a.id === ask)) return
    agent.asks = [...agent.asks, { id: ask, agent: id, tool_name: tool, input, tool_use_id: call }]
    agent.status = 'waiting'
    agent.action = agent.attention = t('shell.waitingApprovalTool', { tool })
  },
  allow(id, ask) {
    const agent = find(id)
    const found = agent?.asks.find((a) => a.id === ask)
    if (!agent || !found) return
    agent.asks = agent.asks.filter((a) => a.id !== ask)
    agent.entries.push({ kind: 'decision', at: Date.now(), tone: 'ok', title: t('shell.allowedOnce'), detail: `Bash · ${String(found.input.command)}` })
    agent.status = 'working'
    agent.attention = ''
  },
  status(id, status, action) {
    const agent = find(id)
    if (!agent) return
    agent.status = status
    if (action !== undefined) agent.action = action
  },
  select(id) {
    agents.current[DEMO_PATH] = id
  },
  type(text, typing) {
    scripted.agent = agents.current[DEMO_PATH] ?? ''
    typeInto(text, typing, (shown) => (scripted.text = shown))
  },
  mode(id, mode) {
    const agent = find(id)
    if (agent) agent.options = { ...agent.options, mode }
  },
  tokens(id, tokens) {
    const agent = find(id)
    if (agent) agent.tokens = tokens
  },
}

function rebuild(at: number, gateOpen: boolean) {
  live = false
  clearTimers()
  clearDemoAgents()
  scripted.text = ''
  for (const [time, run, gated] of EVENTS) if (time <= at && (!gated || gateOpen)) run(director)
  projects.current = DEMO_PATH
}

function play(from: number, to: number, gated: boolean) {
  live = true
  for (const [time, run, g] of EVENTS) {
    if (time <= from || time > to || !!g !== gated) continue
    timers.push(setTimeout(() => run(director), (time - from) * 0.9))
  }
}

function pointOf(i: number): [from: number, to: number] | null {
  return SEGMENTS[STOPS[i].id] ?? null
}

function restAt(i: number) {
  for (let j = i - 1; j >= 0; j--) {
    const seg = pointOf(j)
    if (seg) return seg[1]
  }
  return 2700
}

function enter(i: number) {
  tour.i = Math.max(0, Math.min(STOPS.length - 1, i))
  tour.confirm = false
  tour.allowed = tour.denied = false
  const stop = STOPS[tour.i]
  remember(stop.id)
  const seg = pointOf(tour.i)
  if (seg) {
    rebuild(seg[0], seg[0] > GATE)
    play(seg[0], seg[1], false)
  } else rebuild(Math.max(2700, restAt(tour.i)), true)
}

function onCommand(cmd: string, args: Record<string, unknown>) {
  if (cmd === 'answer_permission' && args.id === 'demo:ask-stripe' && tour.on && current().id === 'perm' && !tour.allowed) {
    tour.allowed = true
    tour.denied = (args.decision as { behavior?: string } | undefined)?.behavior === 'deny'
    const [, to] = SEGMENTS.perm
    play(GATE, to, true)
    timers.push(setTimeout(() => tour.on && current().id === 'perm' && !tour.confirm && next(), 4600))
  }
  return null
}

export function startTour(at = 0) {
  tour.welcome = false
  tour.end = false
  tour.toast = ''
  if (!demo.on) startDemo(t('tour.demoProject'), onCommand)
  tour.on = true
  if (!settings.tour_offered) patchSettings({ tour_offered: true }).catch(() => {})
  enter(at)
}

export function go(i: number) {
  if (tour.on) enter(i)
}

export function next() {
  if (tour.i >= STOPS.length - 1) return finish()
  enter(tour.i + 1)
}

export function back() {
  if (tour.i > 0) enter(tour.i - 1)
}

export function skipChapter() {
  const ch = current().ch
  const n = STOPS.findIndex((s, i) => i > tour.i && s.ch !== ch)
  n < 0 ? finish() : enter(n)
}

export function finish() {
  live = false
  clearTimers()
  rebuild(END, true)
  tour.on = false
  tour.confirm = false
  tour.end = true
}

export const hasOwnProjects = () => projects.list.some((p) => !isDemo(p.path))

function teardown(toast: string) {
  live = false
  clearTimers()
  stopDemo()
  tour.on = false
  tour.end = false
  tour.confirm = false
  tour.toast = toast
}

export function closeEnd() {
  teardown(hasOwnProjects() ? t('tour.toastBack') : t('tour.toastFresh'))
}

export function leave() {
  teardown(t('tour.toastLeft', { n: tour.i + 1 }))
}

export function restartTour() {
  forgetSeen()
  startTour(0)
}

export function firstUnseen() {
  const i = STOPS.findIndex((s) => !tour.seen.includes(s.id))
  return i < 0 ? 0 : i
}

export function offered() {
  tour.welcome = false
  if (!settings.tour_offered) patchSettings({ tour_offered: true }).catch(() => {})
}
