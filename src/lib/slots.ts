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
      ['branch', 'Working branch', 'Green with a worktree, native or declared by the agent'],
      ['pull', 'Pull request', 'Its state on Bitbucket or GitHub; opens its review in the Diff panel'],
      ['tasks', 'Task progress'],
    ],
    hidden: ['tasks'],
  },
  'agent.tools': {
    title: 'Agent tools',
    hint: 'Buttons at the top right of an agent',
    items: [
      ['editor', 'Editor'],
      ['terminal', 'Terminal'],
      ['diff', 'Diff'],
      ['kanban', 'Kanban'],
      ['pulls', 'Pull requests', 'Those you review and those you wrote'],
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
      ['settings', 'Settings', 'Projects, agents, tools and interface'],
      ['about', 'About Squadron', 'Version, Claude Code and account'],
    ],
    locked: ['settings'],
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
