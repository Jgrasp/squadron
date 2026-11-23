import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export type Dialog = { type: 'alert' | 'confirm' | 'prompt' | 'beforeunload'; message: string; defaultPrompt?: string }

export type BrowserTab = { id: string; agent: string; url: string; title: string; loading: boolean; dialog: Dialog | null; width: number | null }

export type BrowserStatus = { tabs: BrowserTab[] }

export const browser = $state<BrowserStatus>({ tabs: [] })

const apply = (status: BrowserStatus) => Object.assign(browser, status)

invoke<BrowserStatus>('browser_status')
  .then(apply)
  .catch(() => {})

listen<BrowserStatus>('browser-status', ({ payload }) => apply(payload))
