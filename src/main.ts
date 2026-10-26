import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'
import { locale } from './lib/i18n/index.svelte'

document.documentElement.lang = locale()

document.addEventListener('animationstart', (event) => {
  if (!['snake', 'pulse', 'ring'].includes(event.animationName)) return
  for (const a of (event.target as Element).getAnimations({ subtree: true })) if (a instanceof CSSAnimation && a.animationName === event.animationName) a.startTime = 0
})

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) await import('./dev/mock')
const { default: App } = await (getCurrentWindow().label === 'board' ? import('./BoardWindow.svelte') : import('./App.svelte'))

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
