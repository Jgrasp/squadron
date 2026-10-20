import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) await import('./dev/mock')

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
