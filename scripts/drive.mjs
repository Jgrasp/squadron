import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { driveDir, instanceOf } from './app.mjs'

export const wrap = (code, id) =>
  `;(async () => { const out = (window.__drive ||= []); try { const value = await (async () => {\n${code}\n})(); out.push({ id: '${id}', value: value ?? null }) } catch (e) { out.push({ id: '${id}', error: e?.stack ? String(e) + '\\n' + e.stack : String(e) }) } })()`

export const answer = (lines, id) =>
  lines
    .split('\n')
    .filter(Boolean)
    .flatMap((line) => JSON.parse(line))
    .find((a) => a.id === id)

const wait = (ms) => new Promise((done) => setTimeout(done, ms))

async function run(dir, code, timeout) {
  const id = Math.random().toString(36).slice(2, 8)
  const file = join(dir, 'answers')
  const from = existsSync(file) ? statSync(file).size : 0
  writeFileSync(join(dir, 'run.js'), wrap(code, id))
  for (const start = Date.now(); Date.now() - start < timeout; await wait(100)) {
    const found = existsSync(file) && answer(readFileSync(file, 'utf8').slice(from), id)
    if (found) {
      console.log(found.error ? `error ${found.error}` : JSON.stringify(found.value))
      process.exit(found.error ? 1 : 0)
    }
  }
  console.error(`No answer after ${timeout} ms: is the dev app up (pnpm app --headless), and does the script finish?`)
  process.exit(1)
}

async function shot(dir) {
  const start = Date.now()
  writeFileSync(join(dir, 'shot'), '')
  const fresh = () => readdirSync(dir).filter((f) => f.endsWith('.png') && statSync(join(dir, f)).mtimeMs >= start)
  for (let n = 0; n < 50 && !fresh().includes('main.png'); n++) await wait(200)
  await wait(400)
  const files = fresh()
  if (!files.length) {
    console.error('No capture: is the dev app up (pnpm app --headless)?')
    process.exit(1)
  }
  for (const f of files) console.log(join(dir, f))
}

if (process.argv[1] === import.meta.filename) {
  const dir = driveDir(instanceOf(resolve(import.meta.dirname, '..')))
  const [what, file, ms] = process.argv.slice(2)
  if (what === 'run' && file) await run(dir, readFileSync(file === '-' ? 0 : file, 'utf8'), Number(ms) || 30000)
  else if (what === 'shot') await shot(dir)
  else {
    console.error('Usage: node scripts/drive.mjs run <script.js | -> [timeout ms]  |  node scripts/drive.mjs shot')
    process.exit(2)
  }
}
