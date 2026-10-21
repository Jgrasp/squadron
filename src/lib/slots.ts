export type Slot = 'agent.tools' | 'agent.menu' | 'menu.squadron'

export type Arrangement = { order?: string[]; hidden?: string[] }

export const SLOTS: Record<Slot, { title: string; hint: string; items: [string, string, string?][]; locked?: string[] }> = {
  'agent.tools': {
    title: 'Agent tools',
    hint: 'Buttons at the top right of an agent',
    items: [
      ['editor', 'Editor'],
      ['terminal', 'Terminal'],
      ['diff', 'Diff'],
      ['kanban', 'Kanban'],
    ],
  },
  'agent.menu': {
    title: 'Agent menu',
    hint: 'The ⋯ menu next to Send',
    items: [
      ['reload', 'Reload', 'Restarts the session, keeps the conversation'],
      ['rename', 'Rename', 'Give the agent a clearer name'],
      ['fork', 'Fork', 'New agent, same context'],
      ['archive', 'Archive', 'Stops the agent and removes it from the list'],
    ],
  },
  'menu.squadron': {
    title: 'Squadron menu',
    hint: 'The menu under the Squadron logo',
    items: [
      ['defaults', 'Agent defaults', 'Model, effort and mode of new agents'],
      ['rules', 'Permission rules', 'What each project always allows'],
      ['claude', 'Claude Code', 'Executable'],
      ['editors', 'Editors', 'Apps that open the project'],
      ['preferences', 'Preferences', 'Working tab effect'],
      ['shortcuts', 'Shortcuts', 'Keys for panels, agents and Kanban'],
      ['layout', 'Layout', 'Order and visibility of buttons and menus'],
      ['customize', 'Customize the layout', 'Drag the buttons where you want them'],
    ],
    locked: ['layout'],
  },
}

export function arrange(known: string[], order: string[] = []) {
  const kept = order.filter((id, i) => known.includes(id) && order.indexOf(id) === i)
  return [...kept, ...known.filter((id) => !kept.includes(id))]
}

export function moved(list: string[], id: string, index: number) {
  const rest = list.filter((i) => i !== id)
  rest.splice(Math.max(0, Math.min(index, rest.length)), 0, id)
  return rest
}
