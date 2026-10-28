import type { Key } from './i18n/index.svelte'

export type Slot = 'agent.info' | 'agent.tools' | 'agent.menu' | 'menu.squadron'

export type Arrangement = { order?: string[]; hidden?: string[] }

export const SLOTS: Record<Slot, { title: Key; hint: Key; items: [string, Key, Key?][]; locked?: string[]; hidden?: string[] }> = {
  'agent.info': {
    title: 'settings.slotInfo',
    hint: 'settings.slotInfoHint',
    items: [
      ['options', 'settings.itemOptions'],
      ['remote', 'settings.itemRemote', 'settings.itemRemoteInfoHint'],
      ['subagents', 'settings.itemSubagents', 'settings.itemSubagentsHint'],
      ['artifacts', 'settings.itemArtifacts', 'settings.itemArtifactsHint'],
      ['context', 'settings.itemContext'],
      ['turns', 'settings.itemTurns'],
      ['branch', 'settings.itemBranch', 'settings.itemBranchHint'],
      ['pull', 'settings.itemPull', 'settings.itemPullHint'],
      ['tasks', 'settings.itemTasks'],
    ],
    hidden: ['tasks'],
  },
  'agent.tools': {
    title: 'settings.slotTools',
    hint: 'settings.slotToolsHint',
    items: [
      ['editor', 'settings.itemEditor'],
      ['terminal', 'settings.itemTerminal'],
      ['diff', 'settings.itemDiff'],
      ['kanban', 'settings.itemKanban'],
      ['pulls', 'settings.itemPulls', 'settings.itemPullsHint'],
      ['runs', 'settings.itemRuns', 'settings.itemRunsHint'],
    ],
  },
  'agent.menu': {
    title: 'settings.slotMenu',
    hint: 'settings.slotMenuHint',
    items: [
      ['reload', 'common.reload', 'settings.itemReloadHint'],
      ['rename', 'common.rename', 'settings.itemRenameHint'],
      ['fork', 'settings.itemFork', 'settings.itemForkHint'],
      ['remote', 'settings.itemRemote', 'settings.itemRemoteMenuHint'],
      ['archive', 'common.archive', 'settings.itemArchiveHint'],
    ],
  },
  'menu.squadron': {
    title: 'settings.slotSquadron',
    hint: 'settings.slotSquadronHint',
    items: [
      ['settings', 'settings.title', 'settings.itemSettingsHint'],
      ['updates', 'settings.itemUpdates', 'settings.itemUpdatesHint'],
      ['about', 'settings.itemAbout', 'settings.itemAboutHint'],
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

export function movedAmong(list: string[], among: string[], id: string, index: number) {
  const others = among.filter((i) => i !== id)
  const rest = list.filter((i) => i !== id)
  return moved(list, id, index < others.length ? rest.indexOf(others[index]) : rest.indexOf(others.at(-1)!) + 1)
}
