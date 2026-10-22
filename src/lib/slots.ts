export type Slot = 'agent.info' | 'agent.tools' | 'agent.menu' | 'menu.squadron'

export type Arrangement = { order?: string[]; hidden?: string[] }

export const SLOTS: Record<Slot, { title: string; hint: string; items: [string, string, string?][]; locked?: string[]; hidden?: string[] }> = {
  'agent.info': {
    title: 'Agent header',
    hint: 'The line under the name of an agent',
    items: [
      ['options', 'Model, effort and mode'],
      ['remote', 'Remote control', 'Turns it on, and shows when the conversation can be followed from claude.ai'],
      ['subagents', 'Subagents', 'Shown once the agent has launched subagents'],
      ['cost', 'Cost'],
      ['context', 'Context tokens'],
      ['turns', 'Turns'],
      ['branch', 'Git branch'],
      ['tasks', 'Task progress'],
    ],
    hidden: ['branch', 'tasks'],
  },
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
      ['remote', 'Remote control', 'Follow and answer this conversation from claude.ai or the Claude app'],
      ['archive', 'Archive', 'Stops the agent and removes it from the list'],
    ],
  },
  'menu.squadron': {
    title: 'Squadron menu',
    hint: 'The menu under the Squadron logo',
    items: [
      ['projects', 'Projects', 'Order, pins, colors and folders'],
      ['archived', 'Archived agents', 'Restore or delete them'],
      ['defaults', 'Agent defaults', 'Model, effort and mode of new agents'],
      ['rules', 'Permission rules', 'What each project always allows'],
      ['claude', 'Claude Code', 'Executable'],
      ['editors', 'Editors', 'Apps that open the project'],
      ['preferences', 'Preferences', 'Working tab effect'],
      ['shortcuts', 'Shortcuts', 'Keys for panels, agents and Kanban'],
      ['layout', 'Layout', 'Order and visibility of buttons and menus'],
      ['customize', 'Customize the layout', 'Drag the buttons where you want them'],
      ['about', 'About Squadron', 'Version, Claude Code and account'],
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
