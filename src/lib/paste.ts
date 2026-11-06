import { invoke } from '@tauri-apps/api/core'
import { errorText } from './i18n/index.svelte'
import { codeLanguage, fenceAt } from './codepaste'

function base64(file: Blob) {
  return new Promise<string>((done, fail) => {
    const reader = new FileReader()
    reader.onload = () => done(String(reader.result).split(',')[1] ?? '')
    reader.onerror = () => fail(reader.error)
    reader.readAsDataURL(file)
  })
}

export function copiedFiles(uris: string) {
  const lines = uris.split(/\r?\n/).filter((line) => line && !line.startsWith('#'))
  return lines.length && lines.every((line) => line.startsWith('file://')) ? lines.map((line) => decodeURIComponent(new URL(line).pathname)) : []
}

type Field = { content: () => string; selection: () => { from: number; to: number }; insert: (text: string) => void }

export function imagePaste(attach: (paths: string[]) => void, fail: (error: string) => void) {
  let pasting = 0
  let raw = false

  async function system() {
    const path = await invoke<string | null>('paste_image').catch((e) => (fail(errorText(e)), null))
    if (path) attach([path])
  }

  function key(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'v') return
    raw = event.shiftKey
    const mine = ++pasting
    setTimeout(() => mine === pasting && system(), 120)
  }

  async function paste(event: ClipboardEvent, field?: Field) {
    pasting++
    const plain = raw
    raw = false
    const data = event.clipboardData
    const files = copiedFiles(data?.getData('text/uri-list') ?? '')
    if (files.length) {
      event.preventDefault()
      fail('')
      return attach(files)
    }
    const images = [...(data?.items ?? [])]
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    if (!images.length) {
      const text = data?.getData('text/plain')
      if (!text) system()
      else if (!plain && field) asCode(event, field, text, data?.getData('vscode-editor-data') ?? '')
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

  function asCode(event: ClipboardEvent, field: Field, text: string, editor: string) {
    const lang = codeLanguage(text, editor)
    const { from, to } = field.selection()
    const block = lang && fenceAt(field.content(), from, to, text, lang)
    if (!block) return
    event.preventDefault()
    field.insert(block)
  }

  return { key, paste }
}
