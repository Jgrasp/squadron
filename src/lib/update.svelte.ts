import { getVersion } from '@tauri-apps/api/app'
import { ask, message } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { agents, saveAll } from './agents.svelte'

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
  const title = 'Check for updates'
  if (import.meta.env.DEV) return message('This Squadron runs from its sources: updates come to the installed app.', { title })
  update.checking = true
  try {
    update.found = await check()
  } catch (e) {
    return message(`Squadron could not reach GitHub: ${e}`, { title, kind: 'error' })
  } finally {
    update.checking = false
  }
  if (!update.found) return message(`Squadron ${await getVersion()} is up to date.`, { title })
  if (await ask(`Squadron ${update.found.version} is available. Install it now?`, { title, okLabel: 'Update', cancelLabel: 'Later' })) install()
}

export async function install() {
  const found = update.found
  if (!found || update.progress !== null) return
  const working = agents.list.filter((a) => a.status === 'working' || a.status === 'waiting').length
  if (
    working &&
    !(await ask(`Squadron restarts to install ${found.version}. ${working} agent${working > 1 ? 's are' : ' is'} still at work and will stop.`, {
      title: 'Update Squadron',
      kind: 'warning',
      okLabel: 'Update now',
      cancelLabel: 'Later',
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
    update.error = String(e)
  }
}
