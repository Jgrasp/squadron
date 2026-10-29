import type { SettingsTab } from './actions.svelte'
import { t, type Key } from './i18n/index.svelte'
import type { Project } from './projects.svelte'
import type { Action } from './shortcuts'
import type { Agent } from './stream'

export type Verb = {
  id: string
  words: string[]
  title: () => string
  icon: string
  scope: 'project' | 'agent' | 'none'
  complete?: string
  action?: Action
  onlyCurrent?: boolean
  forProject?: (project: Project) => boolean
  forAgent?: (agent: Agent) => boolean
}

export type Row = { key: string; verb: Verb; title: string; project?: Project; agent?: Agent; complete: string }

export type World = { projects: Project[]; current?: Project; agents: Agent[]; shown?: Agent; priority?: (project: Project) => number }

const TABS: [SettingsTab, Key][] = [
  ['projects', 'settings.projects'],
  ['archived', 'settings.tabArchived'],
  ['defaults', 'settings.tabDefaults'],
  ['rules', 'settings.tabRules'],
  ['claude', 'settings.tabClaude'],
  ['editors', 'settings.tabEditors'],
  ['hosting', 'settings.tabHosting'],
  ['tickets', 'tickets.tab'],
  ['preferences', 'settings.tabPreferences'],
  ['shortcuts', 'settings.tabShortcuts'],
  ['layout', 'settings.tabLayout'],
  ['changelog', 'settings.tabChangelog'],
  ['about', 'settings.tabAbout'],
]

const running = (agent: Agent) => agent.status === 'working' || agent.status === 'waiting'

export const VERBS: Verb[] = [
  { id: 'open', words: ['open', 'go', 'switch'], title: () => t('spotlight.open'), icon: 'folder', scope: 'project' },
  { id: 'new', words: ['new', 'agent'], title: () => t('spotlight.newAgent'), icon: 'plus', scope: 'project', action: 'newAgent', forProject: (p) => p.exists },
  { id: 'pulls', words: ['pr', 'pull', 'pulls'], title: () => t('spotlight.pulls'), icon: 'pull', scope: 'project' },
  { id: 'board', words: ['board', 'kanban'], title: () => t('spotlight.board'), icon: 'kanban', scope: 'project', action: 'kanban' },
  { id: 'diff', words: ['diff'], title: () => t('spotlight.diff'), icon: 'diff', scope: 'project', action: 'diff', forProject: (p) => p.exists },
  { id: 'terminal', words: ['term', 'terminal'], title: () => t('spotlight.terminal'), icon: 'terminal', scope: 'project', action: 'terminal', forProject: (p) => p.exists },
  { id: 'editor', words: ['edit', 'editor'], title: () => t('spotlight.editor'), icon: 'editor', scope: 'project', action: 'editor', forProject: (p) => p.exists },
  { id: 'archive', words: ['arc', 'archive'], title: () => t('spotlight.archive'), icon: 'archive', scope: 'agent' },
  { id: 'archiveProject', words: ['arc', 'archive'], title: () => t('spotlight.archiveProject'), icon: 'archive', scope: 'project', onlyCurrent: true },
  { id: 'fork', words: ['fork'], title: () => t('spotlight.fork'), icon: 'branch', scope: 'agent', forAgent: (a) => !!a.sessionId },
  { id: 'interrupt', words: ['stop', 'interrupt'], title: () => t('spotlight.interrupt'), icon: 'stop', scope: 'agent', action: 'interrupt', forAgent: running },
  { id: 'pin', words: ['pin'], title: () => t('spotlight.pin'), icon: 'pin', scope: 'agent', forAgent: (a) => !a.pinned },
  { id: 'unpin', words: ['unpin', 'pin'], title: () => t('spotlight.unpin'), icon: 'pin', scope: 'agent', forAgent: (a) => !!a.pinned },
  { id: 'overview', words: ['overview'], title: () => t('spotlight.overview'), icon: 'grid', scope: 'none' },
  { id: 'jump', words: ['jump', 'waiting'], title: () => t('spotlight.jump'), icon: 'jump', scope: 'none', action: 'jump' },
  { id: 'layout', words: ['layout', 'customize'], title: () => t('spotlight.layout'), icon: 'options', scope: 'none' },
  { id: 'updates', words: ['update', 'updates', 'check'], title: () => t('spotlight.updates'), icon: 'update', scope: 'none' },
  ...TABS.map(
    ([tab, key]): Verb => ({
      id: `settings:${tab}`,
      words: ['set', 'settings'],
      title: () => t('spotlight.settings', { tab: t(key) }),
      icon: 'options',
      scope: 'none',
      complete: `set ${tab}`,
    }),
  ),
]

export const STARTERS = ['pr', 'new', 'open', 'board', 'diff', 'term', 'set', 'arc']

const LIMIT = 6

const fold = (text: string) => text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()

export const words = (text: string) => fold(text).split(/[^\p{L}\p{N}]+/u).filter(Boolean)

const reaches = (list: string[], token: string) => list.some((w) => w.startsWith(token))

export function match(query: string, world: World): Row[] {
  const [first, ...rest] = words(query)
  if (!first || first.length < 2) return []
  const found: (Row & { rank: number; weight: number })[] = []
  VERBS.forEach((verb, order) => {
    const title = words(verb.title())
    if (!reaches([...verb.words, title[0]], first)) return
    const own = [...verb.words, ...title]
    const rank = (verb.words.includes(first) ? 0 : 1000) + order
    const accepts = (extra: string[]) => rest.every((token) => reaches([...own, ...extra], token))
    const base = verb.complete ?? verb.words[0]
    if (verb.scope === 'none') {
      if (accepts([])) found.push({ key: verb.id, verb, title: verb.title(), complete: base, rank, weight: 0 })
      return
    }
    if (verb.scope === 'project') {
      for (const project of world.projects) {
        if (verb.forProject && !verb.forProject(project)) continue
        const here = project.path === world.current?.path
        if (verb.onlyCurrent && !rest.length && !here) continue
        if (!accepts(words(project.name))) continue
        found.push({
          key: `${verb.id}:${project.path}`,
          verb,
          title: verb.title(),
          project,
          complete: `${base} ${fold(project.name)}`,
          rank,
          weight: (here ? 1000 : 0) + (world.priority?.(project) ?? 0),
        })
      }
      return
    }
    for (const agent of rest.length ? world.agents : world.shown ? [world.shown] : []) {
      if (verb.forAgent && !verb.forAgent(agent)) continue
      const project = world.projects.find((p) => p.path === agent.project)
      if (!accepts([...words(agent.name), ...words(project?.name ?? '')])) continue
      found.push({
        key: `${verb.id}:${agent.id}`,
        verb,
        title: verb.title(),
        project,
        agent,
        complete: `${base} ${fold(agent.name)}`,
        rank,
        weight: agent.id === world.shown?.id ? 1000 : 0,
      })
    }
  })
  return found.sort((a, b) => a.rank - b.rank || b.weight - a.weight).slice(0, LIMIT)
}
