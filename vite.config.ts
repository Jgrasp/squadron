import { svelte } from '@sveltejs/vite-plugin-svelte'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [svelte()],
  // Every `@tauri-apps/api/core` import of the app goes through src/lib/ipc.ts, where the guided tour keeps its demo away from Rust.
  resolve: { alias: [{ find: /^@tauri-apps\/api\/core$/, replacement: fileURLToPath(new URL('./src/lib/ipc.ts', import.meta.url)) }] },
  clearScreen: false,
  cacheDir: process.env.SQUADRON_VITE_CACHE ?? 'node_modules/.vite',
  server: {
    port: 5173,
    strictPort: true,
    headers: { 'Cache-Control': 'no-store' },
    watch: { ignored: ['**/src-tauri/**'] },
  },
})
