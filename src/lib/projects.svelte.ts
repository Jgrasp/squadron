import { invoke } from '@tauri-apps/api/core'

export type Project = {
  name: string
  path: string
  rules: string[]
  exists: boolean
  git: boolean
  branch: string | null
}

export type Inspection = Project & { taken_by: string | null }

export const projects = $state({ list: [] as Project[], current: '', overview: false })

export function current(): Project | undefined {
  return projects.list.find((p) => p.path === projects.current)
}

export function set(list: Project[]) {
  projects.list = list
  if (!list.some((p) => p.path === projects.current)) projects.current = list[0]?.path ?? ''
}

export async function load() {
  set(await invoke<Project[]>('list_projects'))
}

export async function add(path: string, name: string) {
  set(await invoke<Project[]>('add_project', { path, name }))
  projects.current = path
}

export async function remove(path: string) {
  set(await invoke<Project[]>('remove_project', { path }))
}

export async function move(path: string, to: string) {
  set(await invoke<Project[]>('move_project', { path, to }))
  projects.current = to
}

export async function addRule(path: string, rule: string) {
  set(await invoke<Project[]>('add_rule', { path, rule }))
}

export async function removeRule(path: string, rule: string) {
  set(await invoke<Project[]>('remove_rule', { path, rule }))
}

export function inspect(path: string) {
  return invoke<Inspection>('inspect_folder', { path })
}

export function home(path: string) {
  return path.replace(/^\/home\/[^/]+/, '~')
}
