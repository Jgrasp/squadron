import type { Key } from './i18n/index.svelte'
import type { Display, PanelKind, PanelMode } from './layout.svelte'

export const MODES: PanelMode[] = ['slide', 'full', 'bottom']

export const modeLabel: Record<PanelMode, Key> = {
  slide: 'shell.modeSlide',
  full: 'shell.modeFull',
  bottom: 'shell.modeBottom',
}

export const PANELS: [PanelKind, Key][] = [
  ['terminal', 'shell.terminal'],
  ['editor', 'shell.editor'],
  ['diff', 'shell.diff'],
  ['subagents', 'conversation.subagents'],
  ['artifacts', 'agent.artifactsTitle'],
  ['kanban', 'shell.kanban'],
  ['pulls', 'shell.pullRequests'],
]

export const covers = (next: Display, prev: Display) =>
  next.mode === prev.mode && (next.mode === 'full' || (next.mode === 'slide' ? next.width >= prev.width : next.height >= prev.height))
