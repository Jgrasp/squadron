import type { Group, Project } from './projects.svelte'

export type Row =
  | { kind: 'group'; id: string; name: string; depth: number; folded: boolean; parent: string }
  | { kind: 'project'; project: Project; depth: number; group: string }

export type Stat = { working: number; waiting: number; failed: number }

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

export function contained(groups: Group[], list: Project[], id: string) {
  return list.filter((p) => groupOf(groups, p) && within(groups, groupOf(groups, p), id))
}

const matches = (text: string, query: string) => text.toLowerCase().includes(query)

export function rows(groups: Group[], list: Project[], folded: Record<string, boolean>, search: string): Row[] {
  const query = search.trim().toLowerCase()
  const keeps = (id: string, parentMatched: boolean): boolean => {
    if (!query || parentMatched) return true
    const group = groups.find((g) => g.id === id)
    if (group && matches(group.name, query)) return true
    return (
      list.some((p) => groupOf(groups, p) === id && matches(p.name, query)) ||
      groups.some((g) => parentOf(groups, g) === id && keeps(g.id, false))
    )
  }
  const out: Row[] = []
  const walk = (parent: string, depth: number, parentMatched: boolean) => {
    for (const group of groups.filter((g) => parentOf(groups, g) === parent)) {
      if (!keeps(group.id, parentMatched)) continue
      const own = !!query && matches(group.name, query)
      const open = !!query || !folded[group.id]
      out.push({ kind: 'group', id: group.id, name: group.name, depth, folded: !open, parent })
      if (open) walk(group.id, depth + 1, parentMatched || own)
    }
    for (const project of list.filter((p) => groupOf(groups, p) === parent)) {
      if (!query || parentMatched || matches(project.name, query)) out.push({ kind: 'project', project, depth, group: parent })
    }
  }
  walk('', 0, false)
  return out
}

export function rollup(groups: Group[], list: Project[], id: string, stat: (path: string) => Stat): Stat {
  const sum: Stat = { working: 0, waiting: 0, failed: 0 }
  for (const project of contained(groups, list, id)) {
    const one = stat(project.path)
    sum.working += one.working
    sum.waiting += one.waiting
    sum.failed += one.failed
  }
  return sum
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
