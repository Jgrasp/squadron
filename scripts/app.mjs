import { spawn } from 'node:child_process'
import { createServer } from 'node:net'
import { basename, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
export const instanceOf = (path) =>
  (/[\\/]\.claude[\\/]worktrees[\\/]([^\\/]+)$/.exec(path)?.[1] ?? basename(path))
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'squadron'

const free = (port) =>
  new Promise((done) => {
    const server = createServer()
    server.once('error', () => done(false))
    server.once('listening', () => server.close(() => done(true)))
    server.listen(port, '127.0.0.1')
  })

if (process.argv[1] === import.meta.filename) {
  const name = instanceOf(root)
  let port = 5173
  while (!(await free(port))) port++
  const identifier = `dev.squadron.desktop.dev.${name}`
  const config = { identifier, build: { devUrl: `http://localhost:${port}`, beforeDevCommand: `pnpm exec vite --port ${port} --strictPort` } }
  const env = { ...process.env, SQUADRON_INSTANCE: name, SQUADRON_IDENTIFIER: identifier, VITE_SQUADRON_INSTANCE: name, SQUADRON_VITE_CACHE: process.env.SQUADRON_VITE_CACHE ?? `node_modules/.vite-${name}` }
  console.log(`Squadron dev "${name}": ${identifier} on http://localhost:${port}`)
  spawn('pnpm', ['exec', 'tauri', 'dev', '--config', 'src-tauri/tauri.dev.conf.json', '--config', JSON.stringify(config)], { cwd: root, env, stdio: 'inherit' }).on('exit', (code) => process.exit(code ?? 1))
}
