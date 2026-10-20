import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { apply, exited, fail, newAgent, reloaded, userMessage, type Agent, type Options } from './stream'

export const agents = $state({ list: [] as Agent[], current: {} as Record<string, string> })

export const clock = $state({ now: Date.now() })
setInterval(() => (clock.now = Date.now()), 1000)

export function find(id: string) {
  return agents.list.find((a) => a.id === id)
}

export function ofProject(project: string) {
  return agents.list.filter((a) => a.project === project)
}

export function waiting(project: string) {
  return agents.list.filter((a) => a.project === project && a.status === 'waiting').length
}

export function selected(project: string) {
  const list = ofProject(project)
  return list.find((a) => a.id === agents.current[project]) ?? list[0]
}

listen<{ id: string; line: string }>('agent-line', ({ payload }) => {
  const agent = find(payload.id)
  if (agent) apply(agent, JSON.parse(payload.line))
})

listen<{ id: string; stderr: string }>('agent-exit', ({ payload }) => {
  const agent = find(payload.id)
  if (agent) exited(agent, payload.stderr)
})

export async function create(project: string, name: string, options: Options, text: string) {
  agents.list.push(newAgent(crypto.randomUUID(), project, name, options))
  const agent = agents.list.at(-1)!
  agents.current[project] = agent.id
  userMessage(agent, text)
  try {
    await invoke('create_agent', { id: agent.id, project, options, text })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function send(id: string, text: string) {
  const agent = find(id)
  if (!agent) return
  userMessage(agent, text)
  try {
    await invoke('send_message', { id, text })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function retry(id: string) {
  const agent = find(id)
  const last = agent?.entries.findLast((e) => e.kind === 'user')
  if (!agent || last?.kind !== 'user') return
  agent.status = 'working'
  agent.action = 'Démarre…'
  try {
    await invoke('send_message', { id, text: last.text })
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function interrupt(id: string) {
  const agent = find(id)
  if (!agent || (agent.status !== 'working' && agent.status !== 'waiting')) return
  agent.interrupting = true
  await invoke('interrupt_agent', { id })
}

export async function reload(id: string) {
  const agent = find(id)
  if (!agent) return
  try {
    await invoke('reload_agent', { id })
    reloaded(agent)
  } catch (e) {
    fail(agent, String(e))
  }
}

export async function archive(id: string) {
  await invoke('archive_agent', { id })
  agents.list = agents.list.filter((a) => a.id !== id)
}

export async function archiveProject(project: string) {
  await Promise.all(ofProject(project).map((a) => archive(a.id)))
}
