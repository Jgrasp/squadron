import { spawn } from 'node:child_process'
import { createWriteStream, mkdirSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { basename, join, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
export const instanceOf = (path) =>
  (/[\\/]\.claude[\\/]worktrees[\\/]([^\\/]+)$/.exec(path)?.[1] ?? basename(path))
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '') || 'squadron'
export const driveDir = (name) => join(process.env.XDG_RUNTIME_DIR ?? tmpdir(), `squadron-dev-${name}`)

const free = (port) =>
  new Promise((done) => {
    const server = createServer()
    server.once('error', () => done(false))
    server.once('listening', () => server.close(() => done(true)))
    server.listen(port, '127.0.0.1')
  })

if (process.argv[1] === import.meta.filename) {
  const name = instanceOf(root)
  const headless = process.argv.includes('--headless')
  let port = 5173
  while (!(await free(port))) port++
  const identifier = `dev.squadron.desktop.dev.${name}`
  const drive = driveDir(name)
  mkdirSync(drive, { recursive: true, mode: 0o700 })
  const config = { identifier, build: { devUrl: `http://localhost:${port}`, beforeDevCommand: `pnpm exec vite --port ${port} --strictPort` } }
  const env = {
    ...process.env,
    SQUADRON_INSTANCE: name,
    SQUADRON_IDENTIFIER: identifier,
    VITE_SQUADRON_INSTANCE: name,
    SQUADRON_VITE_CACHE: process.env.SQUADRON_VITE_CACHE ?? `node_modules/.vite-${name}`,
    SQUADRON_DEBUG_DIR: drive,
  }
  const tauri = ['pnpm', 'exec', 'tauri', 'dev', '--config', 'src-tauri/tauri.dev.conf.json', '--config', JSON.stringify(config)]
  const [command, ...args] = headless ? ['xvfb-run', '-a', '-s', '-screen 0 1920x1200x24', ...tauri] : tauri
  console.log(`Squadron dev "${name}": ${identifier} on http://localhost:${port}${headless ? ', headless' : ''}, driven by scripts/drive.mjs through ${drive}`)
  const log = createWriteStream(join(drive, 'log'))
  const child = spawn(command, args, { cwd: root, env, stdio: ['inherit', 'pipe', 'pipe'] })
  for (const [from, to] of [[child.stdout, process.stdout], [child.stderr, process.stderr]]) from.on('data', (chunk) => (to.write(chunk), log.write(chunk)))
  child.on('error', (e) => (console.error(`${command}: ${e.message}`), process.exit(1)))
  child.on('exit', (code) => process.exit(code ?? 1))
}
