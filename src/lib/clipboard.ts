import { invoke } from '@tauri-apps/api/core'

export const copyText = (text: string): Promise<void> => invoke<void>('copy_text', { text }).catch(() => navigator.clipboard.writeText(text))
