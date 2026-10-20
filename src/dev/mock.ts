import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import taskList from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'

type Project = { name: string; path: string; rules: string[] }

const projects: Project[] = [
  { name: 'BMC', path: '/home/quentin/agence/bmc', rules: [] },
  { name: 'Fileas', path: '/home/quentin/agence/fileas', rules: [] },
  { name: 'Lumen', path: '/home/quentin/agence/lumen-absent', rules: [] },
]

if (location.search.includes('many'))
  for (const name of ['Mercure', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    projects.push({ name, path: `/home/quentin/agence/${name.toLowerCase()}`, rules: [] })

const folder = (name: string, path: string, rules: string[] = []) => ({
  name,
  path,
  rules,
  exists: !path.includes('absent'),
  git: !path.includes('fileas'),
  branch: path.includes('fileas') ? null : 'feat/T15-segmentation',
})

const views = () => projects.map((p) => folder(p.name, p.path, p.rules))

const timers = new Map<string, number>()
let claudePath = 'claude'

const asks: Record<string, [string, Record<string, unknown>]> = {
  permission: ['Bash', { command: 'lando composer update sylius/channel-segmentation-plugin', description: 'Updates the plugin' }],
  edit: ['Edit', { file_path: 'src/Segmentation/ChannelSegmentResolver.php', old_string: 'resolve(CustomerInterface $customer)', new_string: 'resolve(\n        CustomerInterface $customer,\n        ?ChannelInterface $channel = null,\n    )' }],
  mcp: ['mcp__github__create_pull_request', { owner: 'agence', repo: 'bmc', base: 'develop', head: 'feat/BAT-809', title: 'BAT-809 · Pilotage des batchs', draft: true }],
  plan: ['ExitPlanMode', { plan: '# Segment CSV export\n\n1. Add the `export_csv` action to the grid.\n2. Create `SegmentCsvExporter`.\n3. Test the exporter.' }],
  questions: ['AskUserQuestion', { questions: [
    { question: 'Which CSV separator?', header: 'Separator', multiSelect: false, options: [{ label: 'Semicolon (Recommended)', description: 'French Excel' }, { label: 'Comma' }, { label: 'Tab' }] },
    { question: 'Which columns to add?', header: 'Columns', multiSelect: true, options: [{ label: 'Creation date' }, { label: 'Order count' }, { label: 'Revenue' }] },
  ] }],
  question: ['AskUserQuestion', { questions: [
    { question: 'How should I handle the 3 failing tests?', header: 'Tests', multiSelect: false, options: [
      { label: 'Update the tests (Recommended)', description: 'Tests follow the new split() behaviour' },
      { label: 'Restore keys in split()', description: 'Keeps the previous behaviour' },
      { label: 'Mark the tests incomplete', description: 'CI stays green, a ticket must be opened' },
    ] },
  ] }],
}
const waitingReplay = new Map<string, () => void>()
let askId = 0

function replay(id: string, text: string) {
  const key = Object.keys(asks).find((k) => text.includes(k))
  if (key) {
    const [tool_name, input] = asks[key]
    const ask = String(askId++)
    waitingReplay.set(ask, () => replay(id, 'ok'))
    emit('permission-request', { id: ask, agent: id, tool_name, input: { ...input, file_path: input.file_path ? `${projectOf.get(id)}/${input.file_path}` : undefined }, tool_use_id: null })
    return
  }
  if (claudePath === 'claude' && text.includes('notfound')) throw 'claude-not-found'
  const stream = text.includes('tasks') ? taskList : text.includes('tool') ? tools : text.includes('logout') ? nologin : text.includes('long') ? sigint : hello
  const lines = stream
    .trim()
    .split('\n')
    .filter((l) => !(text.includes('long') && /"type":"(result|user)"/.test(l) && /interrupted|aborted/.test(l)))
  const crash = text.includes('crash')
  let i = 0
  const next = () => {
    if (crash && i === 6) {
      timers.delete(id)
      emit('agent-exit', { id, stderr: 'API Error: 529 {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}' })
      return
    }
    if (i >= lines.length) return timers.delete(id)
    emit('agent-line', { id, line: lines[i++].replaceAll('/project', projectOf.get(id) ?? '/project') })
    timers.set(id, window.setTimeout(next, text.includes('long') || text.includes('tasks') ? 150 : 60))
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
    await create(projects[0].path, 'Long session', { model: 'opus', effort: 'high', mode: 'default' }, 'Start')
    const agent = find(agents.list.at(-1)!.id)!
    for (let turn = 0; turn < 120; turn++) {
      if (turn) userMessage(agent, `Instruction ${turn}: keep working on module ${turn}`)
      for (const line of (turn % 2 ? tools : sigint).trim().split('\n')) {
        const d = JSON.parse(line.replaceAll('/project', projects[0].path))
        if (d.type === 'result' && d.is_error) continue
        apply(agent, d)
      }
      apply(agent, { type: 'result', is_error: false, total_cost_usd: turn * 0.01, usage: {}, duration_ms: 42000 })
    }
    console.log('stress', Math.round(performance.now() - started), 'ms', agent.entries.length, 'entries')
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
        projects.push({ name: a.name, path: a.path, rules: [] })
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
      case 'answer_permission':
        waitingReplay.get(a.id)?.()
        waitingReplay.delete(a.id)
        return
      case 'cancel_permissions':
      case 'add_rule':
        if (cmd === 'add_rule') projects.find((p) => p.path === a.path)?.rules.push(a.rule)
        return cmd === 'add_rule' ? views() : undefined
      case 'save_agent':
        localStorage.setItem(`agent:${a.id}`, a.data)
        return
      case 'delete_agent_file':
        localStorage.removeItem(`agent:${a.id}`)
        return
      case 'load_agents':
        return Object.keys(localStorage).filter((k) => k.startsWith('agent:')).map((k) => localStorage.getItem(k))
      case 'add_cost': {
        const key = `cost:${a.day}`
        const total = Number(localStorage.getItem(key) ?? 0) + Number(a.usd)
        localStorage.setItem(key, String(total))
        return total
      }
      case 'day_cost':
        return Number(localStorage.getItem(`cost:${a.day}`) ?? 0)
      case 'fork_agent':
      case 'restore_agent':
        projectOf.set(a.id, a.project)
        return
      case 'read_project_file':
        if (a.path.endsWith('.php')) return 'final class ChannelSegmentResolver\n{\n    public function resolve(CustomerInterface $customer): array\n    {\n        return [];\n    }\n}\n'
        return a.path.endsWith('resume.txt') ? '2\n' : null
      case 'open_in_editor':
        return
      case 'get_settings':
        return { claude: claudePath, defaults: { model: 'opus', effort: 'high', mode: 'default' } }
      case 'set_settings':
        claudePath = (a as unknown as { config: { claude: string } }).config.claude
        return
      case 'suggest_title':
        return new Promise((resolve) => setTimeout(() => resolve('Plugin upgrade to 2.3'), 800))
      case 'claude_version':
        return '2.1.304 (Claude Code)'
      case 'remove_rule': {
        const project = projects.find((p) => p.path === a.path)
        if (project) project.rules = project.rules.filter((r) => r !== a.rule)
        return views()
      }
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
