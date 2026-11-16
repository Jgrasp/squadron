import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { ask, message } from './asking.svelte'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { ui } from './actions.svelte'
import { agents, closeAll } from './agents.svelte'
import { errorText, t } from './i18n/index.svelte'
import { notify } from './notify.svelte'

export const update = $state<{ found: Update | null; progress: number | null; error: string; checking: boolean }>({
  found: null,
  progress: null,
  error: '',
  checking: false,
})

const TOLD = 'squadron.update-told'

export async function lookForUpdate() {
  if (import.meta.env.DEV) return
  try {
    update.found = await check()
  } catch {}
  const version = update.found?.version
  if (!version || localStorage.getItem(TOLD) === version) return
  localStorage.setItem(TOLD, version)
  notify('updates', t('notify.squadronUpdate'), t('notify.squadronUpdateBody', { version }), { settings: 'about' })
}

export const news = $state<{ version: string }>({ version: '' })

export async function showChangelogOnce() {
  if (import.meta.env.DEV) return
  const version = await getVersion()
  const seen = localStorage.getItem('squadron.seen-version')
  localStorage.setItem('squadron.seen-version', version)
  if (seen && seen !== version) news.version = version
}

export function openChangelog() {
  news.version = ''
  ui.settings = 'changelog'
}

export async function checkNow() {
  const title = t('settings.itemUpdates')
  if (import.meta.env.DEV) return message(t('settings.updateFromSources'), { title })
  update.checking = true
  try {
    update.found = await check()
  } catch {
    return message(t('settings.updateFailed'), { title: t('settings.updateFailedTitle'), icon: 'github' })
  } finally {
    update.checking = false
  }
  if (!update.found) return message(t('settings.upToDate', { version: await getVersion() }), { title })
  if (await ask(t('settings.updateAsk', { version: update.found.version }), { title, okLabel: t('settings.update'), cancelLabel: t('settings.later') })) install()
}

export async function install() {
  if (!update.found || update.progress !== null) return
  update.found = (await check().catch(() => null)) ?? update.found
  const found = update.found
  if (await invoke<boolean>('package_manager_busy'))
    return message(t('settings.updatePackagesBusy', { version: found.version }), { title: t('settings.updateTitle'), kind: 'warning' })
  const working = agents.list.filter((a) => a.status === 'working' || a.status === 'waiting').length
  if (
    working &&
    !(await ask(t('settings.updateRestart', { version: found.version, count: working }), {
      title: t('settings.updateTitle'),
      kind: 'warning',
      okLabel: t('settings.updateNow'),
      cancelLabel: t('settings.later'),
    }))
  )
    return
  update.error = ''
  update.progress = 0
  let total = 0
  let done = 0
  try {
    await found.downloadAndInstall((event) => {
      if (event.event === 'Started') total = event.data.contentLength ?? 0
      if (event.event === 'Progress') {
        done += event.data.chunkLength
        update.progress = total ? done / total : 0
      }
    })
    await closeAll()
    await relaunch()
  } catch (e) {
    update.progress = null
    update.error = errorText(e)
  }
}
