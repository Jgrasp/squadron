import { getVersion } from '@tauri-apps/api/app'
import { ask, message } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { agents, saveAll } from './agents.svelte'
import { errorText, t } from './i18n/index.svelte'

export const update = $state<{ found: Update | null; progress: number | null; error: string; checking: boolean }>({
  found: null,
  progress: null,
  error: '',
  checking: false,
})

export async function lookForUpdate() {
  if (import.meta.env.DEV) return
  try {
    update.found = await check()
  } catch {}
}

export async function checkNow() {
  const title = t('settings.itemUpdates')
  if (import.meta.env.DEV) return message(t('settings.updateFromSources'), { title })
  update.checking = true
  try {
    update.found = await check()
  } catch (e) {
    return message(t('settings.updateUnreachable', { error: errorText(e) }), { title, kind: 'error' })
  } finally {
    update.checking = false
  }
  if (!update.found) return message(t('settings.upToDate', { version: await getVersion() }), { title })
  if (await ask(t('settings.updateAsk', { version: update.found.version }), { title, okLabel: t('settings.update'), cancelLabel: t('settings.later') })) install()
}

export async function install() {
  const found = update.found
  if (!found || update.progress !== null) return
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
    await saveAll()
    await relaunch()
  } catch (e) {
    update.progress = null
    update.error = errorText(e)
  }
}
