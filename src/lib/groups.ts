import type { Group, Project } from './projects.svelte'

export type ProjectSort = 'activity' | 'attention' | 'name' | 'agents' | 'manual'

export type Tally = { working: number; waiting: number; failed: number; done: number; agents: number; last: number }

export const EMPTY: Tally = { working: 0, waiting: 0, failed: 0, done: 0, agents: 0, last: 0 }

const known = (groups: Group[], id: string) => !!id && groups.some((g) => g.id === id)

export const parentOf = (groups: Group[], group: Group) => (known(groups, group.parent) ? group.parent : '')

export const groupOf = (groups: Group[], project: Project) => (known(groups, project.group) ? project.group : '')

export function within(groups: Group[], id: string, ancestor: string) {
  let at = id
  for (let i = 0; i <= groups.length; i++) {
    if (at === ancestor) return true
    const group = groups.find((g) => g.id === at)
    if (!group?.parent) return false
    at = group.parent
  }
  return false
}

export function inTab(groups: Group[], project: Project, tab: string) {
  const group = groupOf(groups, project)
  if (!tab) return true
  if (tab === '-') return !group
  return !!group && within(groups, group, tab)
}

export function descendants(groups: Group[], id: string) {
  const out: { id: string; path: string[] }[] = []
  const walk = (parent: string, path: string[]) => {
    for (const group of groups.filter((g) => parentOf(groups, g) === parent)) {
      out.push({ id: group.id, path: [...path, group.name] })
      walk(group.id, [...path, group.name])
    }
  }
  walk(id, [])
  return out
}

export function matching(groups: Group[], list: Project[], search: string) {
  const query = search.trim().toLowerCase()
  if (!query) return list
  const named = (id: string): boolean => {
    const group = groups.find((g) => g.id === id)
    return !!group && (group.name.toLowerCase().includes(query) || named(parentOf(groups, group)))
  }
  return list.filter((p) => p.name.toLowerCase().includes(query) || named(groupOf(groups, p)))
}

export function total(list: Project[], tally: (path: string) => Tally): Tally {
  const sum = { ...EMPTY }
  for (const project of list) {
    const one = tally(project.path)
    sum.working += one.working
    sum.waiting += one.waiting
    sum.failed += one.failed
    sum.done += one.done
    sum.agents += one.agents
    sum.last = Math.max(sum.last, one.last)
  }
  return sum
}

const byName = (a: Project, b: Project) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' })

export function sortProjects(list: Project[], by: ProjectSort, tally: (path: string) => Tally, desc = false, pinnedFirst = false) {
  const urgent = (t: Tally) => t.waiting + t.failed
  const compare: Record<ProjectSort, (a: Tally, b: Tally) => number> = {
    activity: (a, b) => b.last - a.last,
    attention: (a, b) => urgent(b) - urgent(a) || b.working - a.working || b.last - a.last,
    agents: (a, b) => b.agents - a.agents || b.last - a.last,
    name: () => 0,
    manual: () => 0,
  }
  const out = [...list]
  if (by === 'name') out.sort(byName)
  else if (by !== 'manual') out.sort((a, b) => compare[by](tally(a.path), tally(b.path)))
  if (desc) out.reverse()
  return pinnedFirst ? [...out.filter((p) => p.pinned), ...out.filter((p) => !p.pinned)] : out
}

export const HOURS = 3_600_000

export function inBar(
  project: Pick<Project, 'path' | 'pinned'>,
  current: string,
  activity: { busy: boolean; last: number },
  now: number,
  hours: number,
) {
  if (project.pinned || project.path === current) return true
  if (hours <= 0) return false
  return activity.busy || (activity.last > 0 && now - activity.last < hours * HOURS)
}
