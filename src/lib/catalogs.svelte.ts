import { UNLISTED, type Catalog, type Command } from './commands'
import { known, type Agent } from './stream'

const KEY = 'squadron.catalogs'
const TERMINAL = ['doctor', 'color', 'focus', 'reload-plugins']

let stored: Record<string, Catalog> = {}
try {
  stored = JSON.parse(localStorage.getItem(KEY) ?? '{}')
} catch {}

export const catalogs = $state<Record<string, Catalog>>(stored)

export function remember(project: string, response: Record<string, any>) {
  catalogs[project] = {
    commands: response.commands ?? [],
    models: response.models ?? [],
    styles: response.available_output_styles ?? [],
  }
  try {
    localStorage.setItem(KEY, JSON.stringify($state.snapshot(catalogs)))
  } catch {}
}

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
