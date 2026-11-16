import { getCurrentWindow } from '@tauri-apps/api/window'
import { isPermissionGranted, onAction, requestPermission, sendNotification } from '@tauri-apps/plugin-notification'
import { untrack } from 'svelte'
import { ui, type SettingsTab } from './actions.svelte'
import { agents, find, selected, show, SSH_KEY, SUDO, VALUES, waitingDetail, waitingFor } from './agents.svelte'
import { asked } from './askpass.svelte'
import { chime } from './chime'
import { t, type Key } from './i18n/index.svelte'
import { showPulls } from './linked.svelte'
import { projects } from './projects.svelte'
import { lists, waitsFor } from './pulls.svelte'
import { settings } from './settings.svelte'
import type { Agent, Status } from './stream'

export const AGENT_EVENTS = ['question', 'permission', 'plan', 'password', 'turn', 'goal', 'error'] as const
export const OTHER_EVENTS = ['pulls', 'updates', 'extensions'] as const
export type NotifyEvent = (typeof AGENT_EVENTS)[number] | (typeof OTHER_EVENTS)[number]

const WAITING: NotifyEvent[] = ['question', 'permission', 'plan', 'password']

export type Target = { agent?: string; pulls?: string; settings?: SettingsTab }

const main = () => getCurrentWindow().label === 'main'

export const wanted = (event: NotifyEvent) => settings.notify_when !== 'never' && !settings.notify_off.includes(event)

async function away() {
  const window = getCurrentWindow()
  return !(await window.isFocused()) || (await window.isMinimized())
}

async function allowed() {
  return (await isPermissionGranted()) || (await requestPermission()) === 'granted'
}

async function deliver(title: string, body: string, target: Target, sound: boolean) {
  if (!(await allowed())) return
  sendNotification({ title, body, extra: target })
  if (sound) chime()
}

async function timely(agent?: Agent) {
  if (!main() || settings.notify_when === 'never') return false
  const hidden = await away()
  if (settings.notify_when === 'background' && !hidden) return false
  if (agent) {
    if (!hidden && projects.current === agent.project && !projects.overview && !projects.board && !projects.pulls && selected(agent.project)?.id === agent.id) return false
    if (agent.remote && (settings.notify_remote === 'skip' || (settings.notify_remote === 'background' && hidden))) return false
  }
  return true
}

export async function notify(event: NotifyEvent, title: string, body: string, target: Target = {}, agent?: Agent) {
  if (!wanted(event) || !(await timely(agent))) return
  const ringing = WAITING.includes(event) && settings.tab_ring === 'sound'
  await deliver(title, body, target, settings.notify_sound.includes(event) && !ringing)
}

export async function notifyRun(title: string, body: string, target: Target = {}) {
  if (await timely()) await deliver(title, body, target, false)
}

export function testNotification() {
  return deliver(t('notify.testTitle'), t('notify.testBody'), {}, false)
}

async function open(target: Target) {
  const window = getCurrentWindow()
  await window.unminimize().catch(() => {})
  await window.setFocus().catch(() => {})
  const agent = target.agent ? find(target.agent) : undefined
  if (agent) show(agent)
  if (target.pulls) showPulls(target.pulls)
  if (target.settings) ui.settings = target.settings
}

type Seen = { status: Status; ask: string; goal: string; password: boolean; started: number }

const seen = new Map<string, Seen>()
const busy = (status: Status) => status === 'working' || status === 'waiting'

function snapshot(agent: Agent, before?: Seen): Seen {
  return {
    status: agent.status,
    ask: agent.asks[0]?.id ?? '',
    goal: agent.goal?.state ?? '',
    password: asked.list.some((q) => q.agent === agent.id),
    started: busy(agent.status) ? (before && busy(before.status) ? before.started : Date.now()) : 0,
  }
}

const projectOf = (agent: Agent) => projects.list.find((p) => p.path === agent.project)?.name ?? ''

const line = (text: string) => (text.split('\n').find((l) => l.trim()) ?? '').replace(/^[#>*\-\s]+/, '').slice(0, 180)

function lastWords(agent: Agent) {
  for (let i = agent.entries.length - 1; i >= 0; i--) {
    const entry = agent.entries[i]
    if (entry.kind !== 'agent') continue
    const text = entry.blocks.findLast((b) => b.type === 'text')
    if (text?.type === 'text' && text.text.trim()) return line(text.text)
  }
  return ''
}

function kindOf(tool: string): NotifyEvent {
  if (tool === 'AskUserQuestion') return 'question'
  if (tool === 'ExitPlanMode') return 'plan'
  if (tool === SUDO || tool === SSH_KEY || tool === VALUES) return 'password'
  return 'permission'
}

function tell(agent: Agent, event: NotifyEvent, detail: string) {
  const title = t(`notify.${event}` as Key, { agent: agent.name })
  const body = [projectOf(agent), detail].filter(Boolean).join(' · ')
  notify(event, title, body, { agent: agent.id }, agent)
}

function compare(agent: Agent, before: Seen, now: Seen) {
  const ask = agent.asks[0]
  if (ask && now.ask !== before.ask) tell(agent, kindOf(ask.tool_name), waitingDetail(agent) || waitingFor(ask))
  if (now.password && !before.password) tell(agent, 'password', asked.list.findLast((q) => q.agent === agent.id)?.prompt ?? '')
  if (now.goal === 'met' && before.goal !== 'met') return tell(agent, 'goal', agent.goal?.condition ?? '')
  if (now.status === 'error' && before.status !== 'error') return tell(agent, 'error', agent.attention)
  if (now.status === 'done' && busy(before.status) && Date.now() - before.started >= settings.notify_turn_secs * 1000) tell(agent, 'turn', lastWords(agent))
}

type PullKey = string

const waiting = new Map<string, Set<PullKey>>()

function pullsOf(project: string) {
  const found = lists[project]
  if (!found || found.error) return null
  return [
    ...found.review.filter((p) => waitsFor(project, p)).map((p) => ({ key: `r${p.number}`, pull: p, mine: false })),
    ...found.mine.filter((p) => p.changes_requested).map((p) => ({ key: `c${p.number}`, pull: p, mine: true })),
  ]
}

function comparePulls() {
  for (const project of projects.list) {
    const now = pullsOf(project.path)
    if (!now) continue
    const before = waiting.get(project.path)
    waiting.set(project.path, new Set(now.map((n) => n.key)))
    if (!before) continue
    for (const { key, pull, mine } of now)
      if (!before.has(key))
        notify('pulls', t(mine ? 'notify.pullChanges' : 'notify.pullReview', { project: project.name }), `#${pull.number} ${pull.title}`, { pulls: project.path })
  }
}

export function watchNotifications() {
  if (!main()) return
  onAction((event) => open((event.notification?.extra ?? {}) as Target)).catch(() => {})
  $effect.root(() => {
    $effect(() => {
      const now = agents.list.map((a) => [a, a.status, a.asks[0]?.id, a.goal?.state, asked.list.length] as const)
      untrack(() => {
        const alive = new Set<string>()
        for (const [agent] of now) {
          alive.add(agent.id)
          const before = seen.get(agent.id)
          const next = snapshot(agent, before)
          seen.set(agent.id, next)
          if (before) compare(agent, before, next)
        }
        for (const id of seen.keys()) if (!alive.has(id)) seen.delete(id)
      })
    })
    $effect(() => {
      for (const project of projects.list) void lists[project.path]
      untrack(comparePulls)
    })
  })
}
