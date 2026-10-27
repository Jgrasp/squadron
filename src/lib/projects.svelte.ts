import { invoke } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { Card } from './board'
import { ofProject } from './agents.svelte'
import { settings } from './settings.svelte'

export type Project = {
  name: string
  path: string
  rules: string[]
  cards: Card[]
  color: string
  pinned: boolean
  archived: boolean
  runs: boolean
  runs_hidden: string[]
  repository: string
  exists: boolean
  git: boolean
  branch: string | null
}

export type Inspection = Project & { taken_by: string | null; taken_archived: boolean; near: string | null }

export const projects = $state({ list: [] as Project[], archived: [] as Project[], current: '', overview: false, board: false, pulls: false })

export function current(): Project | undefined {
  return projects.list.find((p) => p.path === projects.current)
}

export function set(all: Project[]) {
  const list = all.filter((p) => !p.archived)
  projects.list = list
  projects.archived = all.filter((p) => p.archived)
  if (!list.some((p) => p.path === projects.current)) projects.current = list[0]?.path ?? ''
}

export async function load() {
  set(await invoke<Project[]>('list_projects'))
}

export async function add(path: string, name: string, color = '') {
  set(await invoke<Project[]>('add_project', { path, name, color: color || null }))
  projects.current = path
}

export async function scratch(name: string) {
  const [path, all] = await invoke<[string, Project[]]>('scratch_project', { name })
  set(all)
  projects.overview = false
  projects.board = false
  projects.pulls = false
  projects.current = path
}

export async function remove(path: string) {
  set(await invoke<Project[]>('remove_project', { path }))
}

export async function move(path: string, to: string) {
  set(await invoke<Project[]>('move_project', { path, to }))
  projects.current = to
}

export async function editProject(path: string, changes: {
    name?: string
    color?: string
    pinned?: boolean
    archived?: boolean
    runs?: boolean
    runs_hidden?: string[]
    repository?: string
  },) {
  set(await invoke<Project[]>('edit_project', { path, changes }))
}

export async function orderProjects(paths: string[]) {
  const rank = (p: Project) => (paths.includes(p.path) ? paths.indexOf(p.path) : paths.length)
  projects.list = [...projects.list].sort((a, b) => rank(a) - rank(b))
  set(await invoke<Project[]>('order_projects', { paths }))
}

const lastActive = (path: string) => Math.max(0, ...ofProject(path).map((a) => a.since))

export function ordered() {
  const list = [...projects.list]
  if (settings.tab_order === 'name') list.sort((a, b) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' }))
  else if (settings.tab_order === 'activity') list.sort((a, b) => lastActive(b.path) - lastActive(a.path))
  return [...list.filter((p) => p.pinned), ...list.filter((p) => !p.pinned)]
}

export async function addRule(path: string, rule: string) {
  set(await invoke<Project[]>('add_rule', { path, rule }))
}

export async function removeRule(path: string, rule: string) {
  set(await invoke<Project[]>('remove_rule', { path, rule }))
}

export async function setCards(path: string, cards: Card[]) {
  set(await invoke<Project[]>('set_cards', { path, cards }))
}

export async function toggleBoard() {
  const popped = await WebviewWindow.getByLabel('board')
  if (popped) return popped.setFocus()
  projects.overview = false
  projects.pulls = false
  projects.board = !projects.board
}

export function togglePulls() {
  projects.overview = false
  projects.board = false
  projects.pulls = !projects.pulls
}

export async function popOut() {
  projects.board = false
  const popped = await WebviewWindow.getByLabel('board')
  if (popped) return popped.setFocus()
  new WebviewWindow('board', { title: 'Kanban · Squadron', width: 1240, height: 820, minWidth: 760, minHeight: 480 })
}

export function inspect(path: string) {
  return invoke<Inspection>('inspect_folder', { path })
}

export function home(path: string) {
  return path.replace(/^\/home\/[^/]+/, '~')
}
