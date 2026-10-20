import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'

type Project = { name: string; path: string }

const projects: Project[] = [
  { name: 'BMC', path: '/home/quentin/agence/bmc' },
  { name: 'Fileas', path: '/home/quentin/agence/fileas' },
  { name: 'Lumen', path: '/home/quentin/agence/lumen-absent' },
]

if (location.search.includes('many'))
  for (const name of ['Mercure', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    projects.push({ name, path: `/home/quentin/agence/${name.toLowerCase()}` })

const folder = (name: string, path: string) => ({
  name,
  path,
  exists: !path.includes('absent'),
  git: !path.includes('fileas'),
  branch: path.includes('fileas') ? null : 'feat/T15-segmentation',
})

const views = () => projects.map((p) => folder(p.name, p.path))

const timers = new Map<string, number>()
let claudePath = 'claude'

function replay(id: string, text: string) {
  if (claudePath === 'claude' && text.includes('introuvable')) throw 'claude-introuvable'
  const stream = text.includes('outil') ? tools : text.includes('erreur') ? nologin : text.includes('long') ? sigint : hello
  const lines = stream
    .trim()
    .split('\n')
    .filter((l) => !(text.includes('long') && /"type":"(result|user)"/.test(l) && /interrupted|aborted/.test(l)))
  const crash = text.includes('plante')
  let i = 0
  const next = () => {
    if (crash && i === 6) {
      timers.delete(id)
      emit('agent-exit', { id, stderr: 'API Error: 529 {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}' })
      return
    }
    if (i >= lines.length) return timers.delete(id)
    emit('agent-line', { id, line: lines[i++].replaceAll('/projet', projectOf.get(id) ?? '/projet') })
    timers.set(id, window.setTimeout(next, text.includes('long') ? 120 : 60))
  }
  next()
}

const projectOf = new Map<string, string>()

if (location.search.includes('stress'))
  setTimeout(async () => {
    const { create } = await import('../lib/agents.svelte')
    const { find, agents } = await import('../lib/agents.svelte')
    const { apply, userMessage } = await import('../lib/stream')
    const started = performance.now()
    await create(projects[0].path, 'Session longue', { model: 'opus', effort: 'high', mode: 'default' }, 'Début')
    const agent = find(agents.list.at(-1)!.id)!
    for (let turn = 0; turn < 120; turn++) {
      if (turn) userMessage(agent, `Consigne ${turn} : continue le travail sur le module ${turn}`)
      for (const line of (turn % 2 ? tools : sigint).trim().split('\n')) {
        const d = JSON.parse(line.replaceAll('/projet', projects[0].path))
        if (d.type === 'result' && d.is_error) continue
        apply(agent, d)
      }
      apply(agent, { type: 'result', is_error: false, total_cost_usd: turn * 0.01, usage: {}, duration_ms: 42000 })
    }
    console.log('stress', Math.round(performance.now() - started), 'ms', agent.entries.length, 'entrées')
  }, 300)

mockWindows('main')
mockIPC(
  (cmd, args) => {
    const a = args as Record<string, string>
    switch (cmd) {
      case 'list_projects':
        return views()
      case 'inspect_folder':
        return {
          ...folder(a.path.split('/').pop() ?? '', a.path),
          taken_by: projects.find((p) => p.path === a.path)?.name ?? null,
        }
      case 'add_project':
        projects.push({ name: a.name, path: a.path })
        return views()
      case 'remove_project':
        projects.splice(
          projects.findIndex((p) => p.path === a.path),
          1,
        )
        return views()
      case 'move_project':
        projects.find((p) => p.path === a.path)!.path = a.to
        return views()
      case 'create_agent':
        projectOf.set(a.id, a.project)
        return replay(a.id, a.text)
      case 'send_message':
        return replay(a.id, a.text)
      case 'interrupt_agent': {
        clearTimeout(timers.get(a.id))
        timers.delete(a.id)
        const tail = sigint.trim().split('\n').slice(-2)
        tail.forEach((line) => emit('agent-line', { id: a.id, line }))
        emit('agent-exit', { id: a.id, stderr: '' })
        return
      }
      case 'reload_agent':
      case 'archive_agent':
        clearTimeout(timers.get(a.id))
        return
      case 'read_project_file':
        return a.path.endsWith('resume.txt') ? '2\n' : null
      case 'open_in_editor':
        return
      case 'claude_path':
        return claudePath
      case 'set_claude_path':
        claudePath = a.path
        return
      case 'plugin:dialog|open':
        return '/home/quentin/agence/mercure'
      case 'plugin:dialog|ask':
        return true
    }
  },
  { shouldMockEvents: true },
)
