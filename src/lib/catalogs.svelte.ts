import { invoke } from '@tauri-apps/api/core'
import { UNLISTED, type AgentType, type Catalog, type Command } from './commands'
import { known, type Agent } from './stream'

const KEY = 'squadron.catalogs'
const TERMINAL = ['doctor', 'color', 'focus', 'reload-plugins']

let stored: Record<string, Catalog> = {}
try {
  stored = JSON.parse(localStorage.getItem(KEY) ?? '{}')
} catch {}

export const catalogs = $state<Record<string, Catalog>>(stored)

export type Account = { email?: string; organization?: string; subscriptionType?: string }

let signedIn: Account | null = null
try {
  signedIn = JSON.parse(localStorage.getItem('squadron.account') ?? 'null')
} catch {}

export const account = $state({ value: signedIn })

export function remember(project: string, response: Record<string, any>) {
  if (response.account) {
    account.value = response.account
    try {
      localStorage.setItem('squadron.account', JSON.stringify(response.account))
    } catch {}
  }
  delete catalogs[project]
  catalogs[project] = {
    commands: response.commands ?? [],
    models: response.models ?? [],
    styles: response.available_output_styles ?? [],
    agents: response.agents ?? [],
  }
  try {
    localStorage.setItem(KEY, JSON.stringify($state.snapshot(catalogs)))
  } catch {}
}

export const models = () => Object.values(catalogs).reverse().find((c) => c.models?.length)?.models ?? []

export const agentTypes = (project: string): AgentType[] =>
  catalogs[project]?.agents ?? Object.values(catalogs).reverse().find((c) => c.agents?.length)?.agents ?? []

export type Definition = {
  path: string
  source: 'project' | 'user' | 'plugin'
  plugin: string | null
  description: string
  model: string | null
  effort: string | null
  tools: string[] | null
  skills: string[]
  body: string
}

export const definition = (project: string, name: string) => invoke<Definition | null>('agent_definition', { project, name }).catch(() => null)

export function catalogOf(agent: Agent): Catalog | undefined {
  return catalogs[agent.project] ?? Object.values(catalogs)[0]
}

export function commandsOf(agent: Agent): Command[] {
  const terminal = known.terminal.length ? known.terminal : TERMINAL
  const base = catalogOf(agent)?.commands ?? agent.commands.map((name) => ({ name, description: '' }))
  return [...base.filter((c) => !terminal.includes(c.name)), ...UNLISTED.filter((c) => !base.some((b) => b.name === c.name))]
}

export function namesOf(agent: Agent) {
  return new Set(commandsOf(agent).flatMap((c) => [c.name, ...(c.aliases ?? [])]))
}
