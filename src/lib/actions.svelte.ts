import { getCurrentWindow } from '@tauri-apps/api/window'
import { archive, fork, interrupt, jumpToWaiting, pin, shelve, show } from './agents.svelte'
import { layout, setPanel, toggleSidebar } from './layout.svelte'
import { projects, showBoard, toggleBoard, type Project } from './projects.svelte'
import type { Action } from './shortcuts'
import type { Row } from './spotlight'
import type { Agent } from './stream'
import { checkNow } from './update.svelte'

export type SettingsTab =
  | 'projects'
  | 'archived'
  | 'defaults'
  | 'rules'
  | 'claude'
  | 'editors'
  | 'hosting'
  | 'tickets'
  | 'preferences'
  | 'shortcuts'
  | 'layout'
  | 'changelog'
  | 'about'

export const ui = $state({ projects: false, creating: false, customizing: false, searching: false, settings: null as SettingsTab | null, settingsProject: '', adding: null as string | null, drops: 0 })

export type Context = { project?: Project; agent?: Agent }

export function run(action: Action, { project, agent }: Context) {
  switch (action) {
    case 'quit':
      return getCurrentWindow().close()
    case 'jump':
      if (!jumpToWaiting()) projects.overview = true
      return
    case 'kanban':
      return toggleBoard()
    case 'sidebar':
      return project && toggleSidebar()
    case 'newAgent':
      if (project?.exists) ui.creating = true
      return
    case 'interrupt':
      return agent && interrupt(agent.id)
    case 'search':
      ui.searching = true
      return
    case 'projects':
      ui.projects = !ui.projects
      return
    case 'back':
      return history.back()
    case 'forward':
      return history.forward()
    case 'editor':
    case 'terminal':
    case 'diff':
      if (project?.exists) setPanel(project.path, layout.panels[project.path] === action ? null : action)
  }
}

export async function perform({ verb, project, agent }: Row) {
  if (verb.id.startsWith('settings:')) {
    ui.settings = verb.id.slice('settings:'.length) as SettingsTab
    return
  }
  if (project && verb.scope === 'project') {
    projects.overview = projects.board = projects.pulls = false
    projects.current = project.path
  }
  switch (verb.id) {
    case 'new':
      ui.creating = true
      return
    case 'pulls':
      projects.pulls = true
      return
    case 'board':
      return showBoard()
    case 'editor':
    case 'terminal':
    case 'diff':
      return project && setPanel(project.path, verb.id)
    case 'archiveProject':
      return project && shelve(project)
    case 'archive':
      return agent && archive(agent.id)
    case 'fork':
      if (!agent) return
      show(agent)
      return fork(agent.id)
    case 'interrupt':
      return agent && interrupt(agent.id)
    case 'pin':
    case 'unpin':
      return agent && pin(agent.id, verb.id === 'pin')
    case 'overview':
      projects.overview = true
      return
    case 'jump':
      return run('jump', {})
    case 'layout':
      projects.overview = projects.board = projects.pulls = false
      ui.customizing = true
      return
    case 'updates':
      return checkNow()
  }
}
