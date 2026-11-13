import { invoke } from '@tauri-apps/api/core'
import type { ArtifactInfo } from './artifacts'
import { refresh, shelf } from './artifacts.svelte'
import { extensionItem, extensions, releaseTool } from './extensions.svelte'
import { errorText, locale, t } from './i18n/index.svelte'
import { text } from './extensions'
import { saveSettings, settings } from './settings.svelte'

const DAY = 86_400_000

export const KEEP_DAYS = [7, 30, 90, 365]

export type Rule = { days: number; from: 'artifact' | 'extension' | 'squadron'; extension?: string }

export const cleanupErrors = $state<Record<string, string>>({})

const weight = (days: number) => (days === 0 ? Infinity : days)

function shareOf(key: string) {
  const item = extensionItem('artifact.actions', key)
  const action = item && 'artifact.share' in item.action ? item.action['artifact.share'] : undefined
  return item && action ? { item, action } : undefined
}

export function extensionDays(id: string, days: number) {
  const own = settings.artifacts_extensions[id]
  return own === undefined ? days : own < 0 ? settings.artifacts_after_days : own
}

export function baseRule(artifact: ArtifactInfo): Rule {
  const rules = Object.keys(artifact.links ?? {}).flatMap((key): Rule[] => {
    const share = shareOf(key)
    const days = share?.action.cleanup?.days
    return share && days !== undefined ? [{ days: extensionDays(share.item.extension, days), from: 'extension', extension: share.item.extension }] : []
  })
  return rules.sort((a, b) => weight(b.days) - weight(a.days))[0] ?? { days: settings.artifacts_after_days, from: 'squadron' }
}

export const ruleOf = (artifact: ArtifactInfo): Rule => (artifact.keep !== undefined && artifact.keep !== null ? { days: artifact.keep, from: 'artifact' } : baseRule(artifact))

export function dueAt(artifact: ArtifactInfo) {
  const { days } = ruleOf(artifact)
  if (!days) return null
  return Math.max(artifact.modified, settings.artifacts_since, ...Object.values(artifact.links ?? {}).map((l) => l.at)) + days * DAY
}

export const keepLabel = (days: number) => (days === 0 ? t('agent.artifactKeepNever') : days === 365 ? t('agent.artifactKeepYear') : t('agent.artifactKeepDays', { count: days }))

export const extensionName = (id: string) => {
  const name = extensions.list.find((m) => m.id === id)?.manifest?.name
  return name ? text(name, locale()) : id
}

export async function release(agent: string, artifact: ArtifactInfo, remove = true) {
  for (const [key, shared] of Object.entries(artifact.links ?? {})) {
    const share = shareOf(key)
    const unshare = share?.action.unshare
    if (!share || typeof unshare !== 'object') throw new Error(t('agent.artifactCannotStop', { destination: key }))
    const extension = extensions.list.find((m) => m.id === share.item.extension)
    if (!extension?.enabled || extension.pending.length) throw new Error(t('agent.artifactCannotStop', { destination: extensionName(share.item.extension) }))
    const data = await invoke<string>('extension_data_dir', { id: extension.id }).catch(() => '')
    const dir = `${shelf.root}/${agent}/${artifact.id}`
    const input = { verb: 'unshare', destination: key, artifact: { id: artifact.id, title: artifact.title, dir, entry: artifact.entry }, link: shared.link, delete: remove, data }
    await releaseTool(extension.id, unshare.tool, input, agent)
    await invoke('set_artifact_share', { agent, id: artifact.id, destination: key, link: null })
  }
}

export async function releaseAgent(agent: string) {
  const list = await invoke<ArtifactInfo[]>('list_artifacts', { agent }).catch(() => [])
  for (const artifact of list.filter((a) => Object.keys(a.links ?? {}).length)) await release(agent, artifact).catch(() => {})
}

export async function sweep() {
  if (!settings.artifacts_since) return saveSettings({ ...$state.snapshot(settings), artifacts_since: Date.now() })
  const owners = await invoke<string[]>('artifact_owners').catch(() => [])
  for (const agent of owners) {
    const list = await invoke<ArtifactInfo[]>('list_artifacts', { agent }).catch(() => [])
    let changed = false
    for (const artifact of list) {
      const due = dueAt(artifact)
      if (due === null || due > Date.now()) continue
      const key = `${agent}/${artifact.id}`
      try {
        await release(agent, artifact)
        await invoke('delete_artifact', { agent, id: artifact.id })
        delete cleanupErrors[key]
        changed = true
      } catch (e) {
        cleanupErrors[key] = errorText(e)
      }
    }
    if (changed && shelf.of[agent]) await refresh(agent)
  }
}
