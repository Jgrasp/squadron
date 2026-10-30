import { invoke } from '@tauri-apps/api/core'
import type { Key } from './i18n/index.svelte'
import type { Project } from './projects.svelte'
import { settings } from './settings.svelte'

export type Connection = { id: string; name: string; site: string; email: string }
export type Source = { connection: string; key: string; name: string }
export type Person = { name: string; avatar: string }
export type Summary = {
  connection: string
  key: string
  title: string
  status: string
  category: string
  kind: string
  priority: string
  urgency: number
  epic: string
  assignee: Person | null
  mine: boolean
}
export type Comment = { id: string; parent: string | null; author: Person; created: string; body: string }
export type Link = { relation: string; key: string; title: string; status: string }
export type Ticket = {
  connection: string
  key: string
  url: string
  title: string
  kind: string
  status: string
  category: string
  priority: string
  urgency: number
  assignee: Person | null
  reporter: Person | null
  labels: string[]
  components: string[]
  versions: string[]
  epic: Link | null
  sprint: string
  updated: string
  description: string
  subtasks: Link[]
  links: Link[]
  attachments: string[]
  comments: Comment[]
  custom: { name: string; value: string }[]
}
export type TicketLink = { connection: string; key: string; fields?: string[] }
type Sprint = { tickets: Summary[]; sprints: string[]; errors: string[] }

export const FIELDS: [string, Key][] = [
  ['description', 'tickets.fieldDescription'],
  ['comments', 'tickets.fieldComments'],
  ['subtasks', 'tickets.fieldSubtasks'],
  ['links', 'tickets.fieldLinks'],
  ['attachments', 'tickets.fieldAttachments'],
  ['epic', 'tickets.fieldEpic'],
  ['type', 'tickets.fieldType'],
  ['status', 'tickets.fieldStatus'],
  ['labels', 'tickets.fieldLabels'],
  ['components', 'tickets.fieldComponents'],
  ['versions', 'tickets.fieldVersions'],
  ['people', 'tickets.fieldPeople'],
  ['custom', 'tickets.fieldCustom'],
]

export const defaultFields = () => FIELDS.map(([f]) => f).filter((f) => !settings.ticket_fields_off.includes(f))

export type JiraProject = { key: string; name: string }

export const tickets = $state({
  connections: null as Connection[] | null,
  projects: {} as Record<string, JiraProject[] | undefined>,
  sprints: {} as Record<string, (Sprint & { loading: boolean }) | undefined>,
})

export async function loadConnections() {
  tickets.connections = await invoke<Connection[]>('ticket_connections').catch(() => [])
  return tickets.connections
}

const asked = new Set<string>()

export function loadProjects(connection: string) {
  if (asked.has(connection)) return
  asked.add(connection)
  invoke<JiraProject[]>('ticket_projects', { connection })
    .then((found) => (tickets.projects[connection] = found))
    .catch(() => (tickets.projects[connection] = []))
}

const sourcesKey = (project: Project) => project.tickets.map((s) => `${s.connection}:${s.key}`).join(',')
const loadedFor: Record<string, string> = {}

export async function loadSprint(project: Project, force = false) {
  const path = project.path
  const key = sourcesKey(project)
  if (!project.tickets.length) {
    tickets.sprints[path] = undefined
    return
  }
  if (!force && loadedFor[path] === key && tickets.sprints[path]) return
  loadedFor[path] = key
  const before = tickets.sprints[path]
  tickets.sprints[path] = { tickets: before?.tickets ?? [], sprints: before?.sprints ?? [], errors: [], loading: true }
  const found = await invoke<Sprint>('ticket_sprint', { sources: $state.snapshot(project.tickets) }).catch((e) => ({
    tickets: [],
    sprints: [],
    errors: [String(e)],
  }))
  if (loadedFor[path] === key) tickets.sprints[path] = { ...found, loading: false }
}

export const getTicket = (connection: string, key: string) => invoke<Ticket>('ticket_get', { connection, key })

export const ticketContext = (link: TicketLink) =>
  invoke<string>('ticket_context', { connection: link.connection, key: link.key, fields: link.fields ?? defaultFields() })

export const ticketBrief = (path: string, link: TicketLink) =>
  invoke<{ prompt: string; goal: string }>('ticket_brief', { path, connection: link.connection, key: link.key, fields: link.fields ?? defaultFields() })

const BLOCK = /\n*<ticket source="[^"]*" key="([^"]+)"[^>]*>\n?([\s\S]*?)<\/ticket>/g

export function ticketBlocks(text: string) {
  const blocks = [...text.matchAll(BLOCK)].map((m) => ({ key: m[1], body: m[2].trim() }))
  return { text: blocks.length ? text.replace(BLOCK, '').trimEnd() : text, blocks }
}

export const sameTicket = (a: { connection: string; key: string } | undefined | null, b: { connection: string; key: string }) =>
  !!a && a.connection === b.connection && a.key === b.key
