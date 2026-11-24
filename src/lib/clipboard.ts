import { invoke } from '@tauri-apps/api/core'

export const copyText = (text: string): Promise<void> => invoke<void>('copy_text', { text }).catch(() => navigator.clipboard.writeText(text))

export const copyImage = (path: string, data: string): Promise<void> =>
  invoke<void>('copy_image', { path }).catch(() => {
    const [head, body] = data.split(',')
    const type = head.slice(5, head.indexOf(';'))
    const blob = new Blob([Uint8Array.from(atob(body), (c) => c.charCodeAt(0))], { type })
    return navigator.clipboard.write([new ClipboardItem({ [type]: blob })])
  })
