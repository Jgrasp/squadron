import { t, type Key } from './i18n/index.svelte'

export type Action = 'newAgent' | 'jump' | 'kanban' | 'editor' | 'terminal' | 'diff' | 'sidebar' | 'interrupt' | 'back' | 'forward' | 'search' | 'quit'

export const ACTIONS: [Action, Key, string][] = [
  ['newAgent', 'common.newAgent', 'Ctrl+N'],
  ['jump', 'settings.actionJump', 'Ctrl+J'],
  ['kanban', 'settings.actionKanban', 'Ctrl+K'],
  ['editor', 'settings.actionEditor', 'Ctrl+E'],
  ['terminal', 'settings.actionTerminal', 'Ctrl+T'],
  ['diff', 'settings.actionDiff', 'Ctrl+D'],
  ['sidebar', 'settings.actionSidebar', 'Ctrl+B'],
  ['interrupt', 'settings.actionInterrupt', 'Escape'],
  ['back', 'settings.actionBack', 'Alt+ArrowLeft'],
  ['forward', 'settings.actionForward', 'Alt+ArrowRight'],
  ['search', 'settings.actionSearch', 'Ctrl+Space'],
  ['quit', 'settings.actionQuit', 'Ctrl+Q'],
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

const KEYS: Record<string, Key> = {
  Shift: 'settings.keyShift',
  Escape: 'settings.keyEscape',
  Space: 'settings.keySpace',
  Enter: 'settings.keyEnter',
  Backspace: 'settings.keyBackspace',
  Delete: 'settings.keyDelete',
}

const ARROWS: Record<string, string> = { ArrowLeft: '←', ArrowRight: '→' }

export const label = (value: string) =>
  value
    .split('+')
    .map((name) => (KEYS[name] ? t(KEYS[name]) : (ARROWS[name] ?? name)))
    .join(' ')
