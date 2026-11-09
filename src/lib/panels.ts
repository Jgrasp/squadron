import type { Key } from './i18n/index.svelte'
import type { Display, PanelKind, PanelMode } from './layout.svelte'

export const MODES: PanelMode[] = ['slide', 'side', 'bottom', 'full']

export const modeLabel: Record<PanelMode, Key> = {
  slide: 'shell.modeSlide',
  side: 'shell.modeSide',
  bottom: 'shell.modeBottom',
  full: 'shell.modeFull',
}

export const modeHint: Record<PanelMode, Key> = {
  slide: 'shell.modeSlideHint',
  side: 'shell.modeSideHint',
  bottom: 'shell.modeBottomHint',
  full: 'shell.modeFullHint',
}

export const modeShort: Record<PanelMode, Key> = {
  slide: 'settings.placeSlide',
  side: 'settings.placeSide',
  bottom: 'settings.placeBottom',
  full: 'settings.placeFull',
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
