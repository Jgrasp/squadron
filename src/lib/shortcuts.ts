export type Action = 'newAgent' | 'jump' | 'kanban' | 'editor' | 'terminal' | 'diff' | 'sidebar' | 'interrupt' | 'quit'

export const ACTIONS: [Action, string, string][] = [
  ['newAgent', 'New agent', 'Ctrl+N'],
  ['jump', 'Next agent waiting for you', 'Ctrl+J'],
  ['kanban', 'Kanban', 'Ctrl+K'],
  ['editor', 'Editor panel', 'Ctrl+E'],
  ['terminal', 'Terminal panel', 'Ctrl+T'],
  ['diff', 'Diff panel', 'Ctrl+D'],
  ['sidebar', 'Fold the agent list', 'Ctrl+B'],
  ['interrupt', 'Interrupt the agent', 'Escape'],
  ['quit', 'Quit Squadron', 'Ctrl+Q'],
]

type Keys = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>

export function combo(e: Keys) {
  if (['Control', 'Shift', 'Alt', 'Meta', 'AltGraph'].includes(e.key)) return ''
  const key = e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key
  return [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Meta', key].filter(Boolean).join('+')
}

export const usable = (value: string) => /^(Ctrl|Alt|Meta)\+/.test(value) || /^(Escape|F\d{1,2})$/.test(value)

export const binding = (keys: Record<string, string>, action: Action) => keys[action] || ACTIONS.find(([id]) => id === action)![2]

export const pressed = (e: Keys, keys: Record<string, string>) => {
  const value = combo(e)
  return value ? ACTIONS.find(([id]) => binding(keys, id) === value)?.[0] : undefined
}

export const label = (value: string) => value.replaceAll('+', ' ')
