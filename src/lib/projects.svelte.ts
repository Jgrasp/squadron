import { invoke } from '@tauri-apps/api/core'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { Card } from './board'
import { ofProject } from './agents.svelte'
import { requestPanel, setPanel } from './layout.svelte'
import { settings } from './settings.svelte'
import type { Options } from './stream'

export type Defaults = Partial<Pick<Options, 'model' | 'effort' | 'mode' | 'advisor'>>

export type Project = {
  name: string
  path: string
  rules: string[]
  cards: Card[]
  color: string
  pinned: boolean
  tab_closed_at: number
  group: string
  runs: boolean
  runs_notify: boolean
  runs_hidden: string[]
  repository: string
  review_prompt: string
  auto_review: boolean
  defaults: Defaults
  exists: boolean
  git: boolean
  branch: string | null
}

export type Group = { id: string; name: string; parent: string }

type Tree = { groups: Group[]; projects: Project[] }

export type Inspection = Project & { taken_by: string | null; near: string | null }

export const projects = $state({ list: [] as Project[], groups: [] as Group[], current: localStorage.getItem('squadron.project') ?? '', overview: false, usage: false, board: false, pulls: false })

$effect.root(() => {
  $effect(() => {
    if (projects.current) localStorage.setItem('squadron.project', projects.current)
  })
})

export function defaultsFor(path: string): Options {
  const own = projects.list.find((p) => p.path === path)?.defaults
  const base = settings.defaults
  const advisor = own?.advisor ?? base.advisor
  return { model: own?.model ?? base.model, effort: own?.effort ?? base.effort, mode: own?.mode ?? base.mode, ...(advisor ? { advisor } : {}) }
}

export function current(): Project | undefined {
  return projects.list.find((p) => p.path === projects.current)
}

export const sandbox: { project: Project | null } = { project: null }

export function set(list: Project[]) {
  projects.list = sandbox.project ? [...list.filter((p) => p.path !== sandbox.project!.path), sandbox.project] : list
  if (!list.some((p) => p.path === projects.current)) projects.current = list[0]?.path ?? ''
}

export async function load() {
  projects.groups = await invoke<Group[]>('list_groups')
  set(await invoke<Project[]>('list_projects'))
}

function setTree(tree: Tree) {
  projects.groups = tree.groups
  set(tree.projects)
}

export async function addGroup(name: string, parent = '') {
  const id = crypto.randomUUID()
  setTree(await invoke<Tree>('add_group', { id, name, parent }))
  return id
}

export async function editGroup(id: string, changes: { name?: string; parent?: string }) {
  setTree(await invoke<Tree>('edit_group', { id, name: changes.name ?? null, parent: changes.parent ?? null }))
}

export async function removeGroup(id: string) {
  setTree(await invoke<Tree>('remove_group', { id }))
}

export async function moveProjects(paths: string[], group: string) {
  setTree(await invoke<Tree>('move_projects', { paths, group }))
}

export async function add(path: string, name: string, color = '') {
  set(await invoke<Project[]>('add_project', { path, name, color: color || null }))
  projects.current = path
}

export async function scratch(name: string) {
  const [path, all] = await invoke<[string, Project[]]>('scratch_project', { name })
  set(all)
  projects.overview = projects.usage = false
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
    tab_closed_at?: number
    archived?: boolean
    runs?: boolean
    runs_notify?: boolean
    runs_hidden?: string[]
    repository?: string
    review_prompt?: string
    auto_review?: boolean
    defaults?: Defaults
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
  const project = projects.list.find((p) => p.path === path)
  if (project) project.cards = cards
  set(await invoke<Project[]>('set_cards', { path, cards }))
}

export async function toggleBoard() {
  const popped = await WebviewWindow.getByLabel('board')
  if (popped) return popped.setFocus()
  projects.overview = projects.usage = false
  if (projects.board) return void (projects.board = false)
  projects.pulls = false
  projects.board = true
  setPanel(projects.current, null)
  requestPanel('kanban')
}

export async function showBoard() {
  const popped = await WebviewWindow.getByLabel('board')
  if (popped) return popped.setFocus()
  projects.overview = projects.usage = false
  projects.pulls = false
  projects.board = true
  setPanel(projects.current, null)
  requestPanel('kanban')
}

export function toggleUsage() {
  projects.usage = !projects.usage
  if (projects.usage) projects.overview = projects.board = projects.pulls = false
}

export function togglePulls() {
  projects.overview = projects.usage = false
  if (projects.pulls) return void (projects.pulls = false)
  projects.board = false
  projects.pulls = true
  setPanel(projects.current, null)
  requestPanel('pulls')
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
