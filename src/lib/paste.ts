import { invoke } from '@tauri-apps/api/core'
import { errorText } from './i18n/index.svelte'

function base64(file: Blob) {
  return new Promise<string>((done, fail) => {
    const reader = new FileReader()
    reader.onload = () => done(String(reader.result).split(',')[1] ?? '')
    reader.onerror = () => fail(reader.error)
    reader.readAsDataURL(file)
  })
}

export function imagePaste(attach: (paths: string[]) => void, fail: (error: string) => void) {
  let pasting = 0

  async function system() {
    const path = await invoke<string | null>('paste_image').catch((e) => (fail(errorText(e)), null))
    if (path) attach([path])
  }

  function key(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'v') return
    const mine = ++pasting
    setTimeout(() => mine === pasting && system(), 120)
  }

  async function paste(event: ClipboardEvent) {
    pasting++
    const data = event.clipboardData
    const images = [...(data?.items ?? [])]
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    if (!images.length) {
      if (!data?.getData('text/plain')) system()
      return
    }
    if (!data?.getData('text/plain')) event.preventDefault()
    fail('')
    for (const image of images) {
      const name = image.name && image.name !== 'image.png' ? image.name : `screenshot-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.${image.type.split('/')[1] || 'png'}`
      try {
        attach([await invoke<string>('save_attachment', { name, data: await base64(image) })])
      } catch (e) {
        fail(errorText(e))
      }
    }
  }

  return { key, paste }
}
