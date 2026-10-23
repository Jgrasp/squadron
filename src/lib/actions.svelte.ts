import { getCurrentWindow } from '@tauri-apps/api/window'
import { interrupt, jumpToWaiting } from './agents.svelte'
import { layout, setPanel, toggleSidebar } from './layout.svelte'
import { projects, toggleBoard, type Project } from './projects.svelte'
import type { Action } from './shortcuts'
import type { Agent } from './stream'

export type SettingsTab =
  | 'projects'
  | 'archived'
  | 'defaults'
  | 'rules'
  | 'claude'
  | 'editors'
  | 'hosting'
  | 'preferences'
  | 'shortcuts'
  | 'layout'
  | 'about'

export const ui = $state({ creating: false, customizing: false, settings: null as SettingsTab | null, adding: null as string | null })

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
    case 'editor':
    case 'terminal':
    case 'diff':
      if (project && agent) setPanel(project.path, layout.panels[project.path] === action ? null : action)
  }
}
