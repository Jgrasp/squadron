import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { EDITS } from './diff'
import { addRule, projects } from './projects.svelte'
import { matches, ruleFor, ruleLabel } from './rules'
import {
  apply,
  argument,
  exited,
  fail,
  newAgent,
  reloaded,
  userMessage,
  type Agent,
  type Ask,
  type Entry,
  type Options,
} from './stream'

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
  if (!agent) return
  exited(agent, payload.stderr)
  invoke('cancel_permissions', { agent: agent.id })
})

listen<Ask>('permission-request', ({ payload: ask }) => {
  const agent = find(ask.agent)
  if (!agent) return decide(ask.id, { behavior: 'deny', message: 'Agent inconnu de Squadron.' })
  const rules = projects.list.find((p) => p.path === agent.project)?.rules ?? []
  if ((agent.allowEdits && EDITS.includes(ask.tool_name)) || rules.some((r) => matches(r, ask.tool_name, ask.input)))
    return decide(ask.id, { behavior: 'allow', updatedInput: ask.input })
  agent.asks.push(ask)
  waitFor(agent)
})

export function toolLabel(name: string) {
  const mcp = /^mcp__(.+?)__(.+)$/.exec(name)
  return mcp ? `${mcp[1]} · ${mcp[2]}` : name
}

function waitFor(agent: Agent) {
  const ask = agent.asks[0]
  if (!ask) {
    agent.status = 'working'
    agent.action = 'Reprend…'
    return
  }
  agent.status = 'waiting'
  agent.action =
    ask.tool_name === 'AskUserQuestion'
      ? 'Attend ta réponse · Question'
      : ask.tool_name === 'ExitPlanMode'
        ? 'Attend ton accord · Plan'
        : `Attend ton accord · ${toolLabel(ask.tool_name).split(' · ')[0]}`
}

export function waitingDetail(agent: Agent) {
  const ask = agent.asks[0]
  if (!ask) return ''
  if (ask.tool_name === 'AskUserQuestion') return String(ask.input.questions?.[0]?.question ?? '')
  if (ask.tool_name === 'ExitPlanMode')
    return (String(ask.input.plan ?? '').split('\n').find((l) => l.trim()) ?? '').replace(/^#+\s*/, '')
  if (ask.tool_name.startsWith('mcp__')) return toolLabel(ask.tool_name)
  return argument(agent, { name: ask.tool_name, input: ask.input }) || toolLabel(ask.tool_name)
}

function decide(id: string, decision: Record<string, unknown>) {
  return invoke('answer_permission', { id, decision })
}

function settle(agent: Agent, ask: Ask, decision: Record<string, unknown>, entry: Omit<Extract<Entry, { kind: 'decision' }>, 'kind' | 'at'>) {
  agent.asks = agent.asks.filter((a) => a.id !== ask.id)
  agent.entries.push({ kind: 'decision', at: Date.now(), ...entry })
  waitFor(agent)
  return decide(ask.id, decision).catch((e) => fail(agent, String(e)))
}

const detailOf = (agent: Agent, ask: Ask) =>
  `${toolLabel(ask.tool_name)} · ${argument(agent, { name: ask.tool_name, input: ask.input })}`.replace(/ · $/, '')

export async function allow(agent: Agent, ask: Ask, always = false) {
  let note: string | undefined
  if (always && EDITS.includes(ask.tool_name)) {
    agent.allowEdits = true
    note = `${agent.name} modifie les fichiers sans demander jusqu’à la fin de sa session`
  } else if (always) {
    const rule = ruleFor(ask.tool_name, ask.input)
    await addRule(agent.project, rule)
    note = `Règle ajoutée pour ${projects.list.find((p) => p.path === agent.project)?.name} : ${ruleLabel(rule)}`
  }
  return settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: always ? 'Toujours autorisé' : 'Autorisé une fois',
    detail: detailOf(agent, ask),
    note,
  })
}

export function deny(agent: Agent, ask: Ask, message = '') {
  return settle(
    agent,
    ask,
    { behavior: 'deny', message: message || 'L’utilisateur a refusé.' },
    { tone: 'bad', title: 'Refusé', detail: detailOf(agent, ask), note: message ? `Avec ta consigne : « ${message} »` : undefined },
  )
}

export function answer(agent: Agent, ask: Ask, answers: Record<string, string>) {
  return settle(
    agent,
    ask,
    { behavior: 'allow', updatedInput: { ...ask.input, answers } },
    {
      tone: 'ok',
      title: 'Répondu',
      detail: Object.entries(answers)
        .map(([q, a]) => `${q} → ${a.replace(/\s*\((recommended|recommandé)\)/gi, '')}`)
        .join(' · '),
    },
  )
}

export function plan(agent: Agent, ask: Ask, choice: 'edits' | 'ask' | 'continue', remark = '') {
  if (choice === 'continue')
    return settle(
      agent,
      ask,
      { behavior: 'deny', message: remark || 'Continue à planifier, ne modifie rien pour l’instant.' },
      { tone: 'bad', title: 'Plan à revoir', detail: 'Continuer à planifier', note: remark ? `Avec ta remarque : « ${remark} »` : undefined },
    )
  if (choice === 'edits') agent.allowEdits = true
  return settle(agent, ask, { behavior: 'allow', updatedInput: ask.input }, {
    tone: 'ok',
    title: 'Plan validé',
    detail: choice === 'edits' ? 'Lancé en « accepter les modifications »' : 'Lancé en demandant à chaque modification',
  })
}

export function reply(agent: Agent, text: string) {
  const ask = agent.asks[0]
  if (!ask) return false
  if (ask.tool_name === 'AskUserQuestion') {
    const answers: Record<string, string> = {}
    for (const q of ask.input.questions ?? []) answers[q.question] = text
    answer(agent, ask, answers)
  } else if (ask.tool_name === 'ExitPlanMode') plan(agent, ask, 'continue', text)
  else deny(agent, ask, text)
  return true
}

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
  agent.asks = []
  await invoke('interrupt_agent', { id })
  await invoke('cancel_permissions', { agent: id })
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
  await invoke('cancel_permissions', { agent: id })
  agents.list = agents.list.filter((a) => a.id !== id)
}

export async function archiveProject(project: string) {
  await Promise.all(ofProject(project).map((a) => archive(a.id)))
}
