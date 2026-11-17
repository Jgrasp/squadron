import { emitTo, listen } from '@tauri-apps/api/event'
import { availableMonitors, getCurrentWindow } from '@tauri-apps/api/window'
import { getAllWebviewWindows, WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { find, shownAgent } from './agents.svelte'
import { boundsKey, PAGE_THEME, panelsOf, text, themeOf, windowOf, type OptionValue, type Panel } from './extensions'
import { extensions, onPanelPost, panelClosed, panelMessage, setOption } from './extensions.svelte'
import { locale } from './i18n/index.svelte'

export type Gesture = { focused: boolean; hovered: boolean; visible: boolean }

export type WindowState = { title: string; owner: string; page: string; extension: string; panel: string; theme: Record<string, string>; options: unknown }

type FromWindow = { label: string; kind: 'ready' | 'message' | 'option' | 'state' | 'closed'; gesture?: Gesture; message?: unknown; id?: string; value?: unknown }

export const gestures: Record<string, Gesture & { extension: string }> = {}

const open = new Map<string, () => void>()

const main = getCurrentWindow().label === 'main'

const windowed = () => panelsOf(extensions.list).filter((p) => p.window)

const panelOf = (label: string) => windowed().find((p) => windowOf(p) === label)

const ownerOf = (panel: Panel) => {
  const name = extensions.list.find((m) => m.id === panel.extension)?.manifest?.name
  return name ? text(name, locale()) : panel.extension
}

function stateOf(panel: Panel): WindowState {
  const style = getComputedStyle(document.documentElement)
  return {
    title: text(panel.title, locale()),
    owner: ownerOf(panel),
    page: panel.page,
    extension: panel.extension,
    panel: panel.id,
    theme: Object.fromEntries(PAGE_THEME.map((name) => [name, style.getPropertyValue(`--${name}`).trim()])),
    options: $state.snapshot(extensions.options[panel.extension] ?? {}),
  }
}

export type Bounds = { x: number; y: number; width: number; height: number }

async function boundsOf(label: string): Promise<Partial<Bounds> & { center?: boolean }> {
  try {
    const kept = JSON.parse(localStorage.getItem(boundsKey(label)) ?? 'null') as Bounds | null
    if (!kept || ![kept.x, kept.y, kept.width, kept.height].every(Number.isFinite)) return { width: 1100, height: 760, center: true }
    const size = { width: Math.max(480, kept.width), height: Math.max(360, kept.height) }
    const [x, y] = [kept.x + size.width / 2, kept.y + size.height / 2]
    const seen = (await availableMonitors()).some((m) => {
      const [left, top] = [m.position.x / m.scaleFactor, m.position.y / m.scaleFactor]
      return x >= left && y >= top && x < left + m.size.width / m.scaleFactor && y < top + m.size.height / m.scaleFactor
    })
    return seen ? { ...size, x: kept.x, y: kept.y } : { ...size, center: true }
  } catch {
    return { width: 1100, height: 760, center: true }
  }
}

export async function openWindow(panel: Panel) {
  const label = windowOf(panel)
  const shown = await WebviewWindow.getByLabel(label)
  if (shown) return Promise.all([shown.unminimize(), shown.show()]).then(() => shown.setFocus())
  new WebviewWindow(label, {
    url: `index.html?${new URLSearchParams({ extension: panel.extension, panel: panel.id })}`,
    title: `${text(panel.title, locale())} · ${ownerOf(panel)}`,
    ...(await boundsOf(label)),
    minWidth: 480,
    minHeight: 360,
    maximized: panel.window === 'maximized',
    fullscreen: panel.window === 'fullscreen',
  })
}

export const windowOpen = (panel: Panel) => open.has(windowOf(panel))

function forget(label: string) {
  open.get(label)?.()
  open.delete(label)
  delete gestures[label]
}

export async function raiseMain() {
  const me = getCurrentWindow()
  if (document.activeElement instanceof HTMLElement) document.activeElement.blur()
  await me.unminimize()
  await me.show()
  await me.setFocus()
}

if (main) {
  listen<FromWindow>('extension-window', ({ payload: p }) => {
    const panel = panelOf(p.label)
    if (!panel) return
    if (p.gesture) gestures[p.label] = { ...p.gesture, extension: panel.extension }
    if (p.kind === 'ready') {
      if (!open.has(p.label)) open.set(p.label, onPanelPost(panel.extension, panel.id, (message) => emitTo(p.label, 'extension-window-post', { extension: panel.extension, panel: panel.id, message }).catch(() => {})))
      emitTo(p.label, 'extension-window-state', stateOf(panel)).catch(() => {})
    }
    if (p.kind === 'message') {
      panelMessage(panel, (p.gesture?.visible && find(shownAgent() ?? '')) || null, p.message).catch(() => {})
    }
    if (p.kind === 'option' && typeof p.id === 'string') setOption(panel.extension, p.id, (p.value ?? null) as OptionValue | null).catch((err) => console.warn(err))
    if (p.kind === 'closed') {
      forget(p.label)
      panelClosed(panel)
    }
  })

  getAllWebviewWindows().then((all) => all.filter((w) => w.label.startsWith('extension-')).forEach((w) => emitTo(w.label, 'extension-window-ask').catch(() => {})))

  $effect.root(() => {
    $effect(() => {
      const live = new Set(windowed().map(windowOf))
      for (const label of open.keys())
        if (!live.has(label)) {
          forget(label)
          WebviewWindow.getByLabel(label).then((w) => w?.destroy())
        }
    })
    $effect(() => {
      themeOf(extensions.list)
      void extensions.options
      for (const panel of windowed()) {
        void extensions.options[panel.extension]
        if (open.has(windowOf(panel))) emitTo(windowOf(panel), 'extension-window-state', stateOf(panel)).catch(() => {})
      }
    })
  })
}
