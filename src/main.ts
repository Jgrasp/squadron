import { getCurrentWindow } from '@tauri-apps/api/window'
import { mount } from 'svelte'
import './app.css'

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) await import('./dev/mock')
const { default: App } = await (getCurrentWindow().label === 'board' ? import('./BoardWindow.svelte') : import('./App.svelte'))

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
