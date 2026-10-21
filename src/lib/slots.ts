export type Slot = 'agent.tools' | 'agent.menu' | 'menu.squadron'

export type Arrangement = { order?: string[]; hidden?: string[] }

export const SLOTS: Record<Slot, { title: string; hint: string; items: [string, string][]; locked?: string[] }> = {
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
      ['reload', 'Reload'],
      ['rename', 'Rename'],
      ['fork', 'Fork'],
      ['archive', 'Archive'],
    ],
  },
  'menu.squadron': {
    title: 'Squadron menu',
    hint: 'The menu under the Squadron logo',
    items: [
      ['defaults', 'Agent defaults'],
      ['rules', 'Permission rules'],
      ['claude', 'Claude Code'],
      ['editors', 'Editors'],
      ['preferences', 'Preferences'],
      ['shortcuts', 'Shortcuts'],
      ['layout', 'Layout'],
      ['customize', 'Customize the layout'],
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
