import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  cacheDir: process.env.SQUADRON_VITE_CACHE ?? 'node_modules/.vite',
  server: {
    port: 5173,
    strictPort: true,
    headers: { 'Cache-Control': 'no-store' },
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
