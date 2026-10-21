import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import taskList from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'

type Project = { name: string; path: string; rules: string[]; cards?: unknown[] }

const projects: Project[] = JSON.parse(localStorage.getItem('projects') ?? 'null') ?? [
  { name: 'BMC', path: '/home/quentin/agence/bmc', rules: [] },
  { name: 'Fileas', path: '/home/quentin/agence/fileas', rules: [] },
  { name: 'Lumen', path: '/home/quentin/agence/lumen-absent', rules: [] },
]

const label = location.search.includes('board') ? 'board' : 'main'
const channel = new BroadcastChannel('squadron-events')
channel.onmessage = ({ data }) => data.target === label && emit(data.event, data.payload)
let popup: Window | null = null

if (location.search.includes('many'))
  for (const name of ['Mercure', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    projects.push({ name, path: `/home/quentin/agence/${name.toLowerCase()}`, rules: [] })

const COLORS = ['#A78BFA', '#2DD4BF', '#F472B6', '#60A5FA', '#A3E635', '#FACC15']

const folder = (name: string, path: string, rules: string[] = [], cards: unknown[] = [], color = COLORS[0]) => ({
  name,
  path,
  rules,
  cards,
  color,
  exists: !path.includes('absent'),
  git: !path.includes('fileas'),
  branch: path.includes('fileas') ? null : 'feat/T15-segmentation',
})

const views = () => projects.map((p, i) => folder(p.name, p.path, p.rules, p.cards, COLORS[i % COLORS.length]))

const timers = new Map<string, number>()
let claudePath = 'claude'
let tabEffect = 'pulse'
let keyMap: Record<string, string> = {}
let editorApps: unknown[] = []

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
const files = new Map<string, string>()
let terminal = 0
let line = ''
const prompt = '\x1b[38;2;154;214;203mquentin\x1b[0m \x1b[2m~/agence/bmc\x1b[0m \x1b[38;2;201;166;255mfeat/T15-segmentation*\x1b[0m\r\n\x1b[38;2;224;138;91m❯\x1b[0m '

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

mockWindows(label)
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
        if (files.has(a.path)) return files.get(a.path)
        if (a.path.endsWith('.php')) return '<?php\n\nfinal class ChannelSegmentResolver\n{\n    public function resolve(CustomerInterface $customer): array\n    {\n        return [];\n    }\n}\n'
        return a.path.endsWith('resume.txt') ? '2\n' : null
      case 'open_in_editor':
        return
      case 'get_settings':
        return { claude: claudePath, defaults: { model: 'opus', effort: 'high', mode: 'default' }, tab_effect: tabEffect, keys: keyMap, editors: editorApps }
      case 'set_settings':
        claudePath = (a as unknown as { config: { claude: string } }).config.claude
        tabEffect = (a as unknown as { config: { tab_effect: string } }).config.tab_effect
        keyMap = (a as unknown as { config: { keys: Record<string, string> } }).config.keys
        editorApps = (a as unknown as { config: { editors: unknown[] } }).config.editors
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
      case 'set_cards':
        projects.find((p) => p.path === a.path)!.cards = (args as { cards: unknown[] }).cards
        localStorage.setItem('projects', JSON.stringify(projects))
        return views()
      case 'plugin:webview|create_webview_window':
        popup = window.open('/?board', 'board', 'popup,width=1240,height=820')
        return
      case 'plugin:window|get_all_windows':
        return popup && !popup.closed ? [label, 'board'] : [label]
      case 'plugin:window|set_focus':
        if (label === 'main') popup?.focus()
        return
      case 'git_snapshot':
        return a.project.includes('fileas') ? null : 'a'.repeat(40)
      case 'git_changes':
        if (a.project.includes('fileas')) throw 'not a git repository'
        return {
          from: 'a'.repeat(40),
          now: 'b'.repeat(40),
          files: [
            { path: 'composer.lock', status: 'M', add: 3, del: 3 },
            { path: 'resume.txt', status: 'M', add: 1, del: 1 },
            { path: 'src/Segmentation/ChannelSegmentResolver.php', status: 'M', add: 6, del: 2 },
            { path: 'tests/ResolverTest.php', status: 'A', add: 4, del: 0 },
            { path: 'src/Legacy/OldResolver.php', status: 'D', add: 0, del: 12 },
          ],
        }
      case 'git_patch':
        return a.path.endsWith('Test.php')
          ? `diff --git a/${a.path} b/${a.path}\nnew file mode 100644\n--- /dev/null\n+++ b/${a.path}\n@@ -0,0 +1,4 @@\n+<?php\n+\n+final class ResolverTest extends TestCase\n+{\n`
          : `diff --git a/${a.path} b/${a.path}\n--- a/${a.path}\n+++ b/${a.path}\n@@ -10,6 +10,11 @@ final class ChannelSegmentResolver\n {\n-    public function resolve(CustomerInterface $customer): array\n+    public function resolve(\n+        CustomerInterface $customer,\n+        ?ChannelInterface $channel = null,\n+    ): array\n     {\n-        return [];\n+        $channel ??= $this->channelContext->getChannel();\n+\n+        return $this->repository->findForCustomer($customer, $channel);\n     }\n }\n`
      case 'list_dir': {
        const tree: Record<string, [string, boolean][]> = {
          '': [['config', true], ['src', true], ['tests', true], ['vendor', true], ['composer.json', false], ['composer.lock', false], ['resume.txt', false]],
          src: [['Segmentation', true]],
          'src/Segmentation': [['ChannelSegmentResolver.php', false], ['SegmentRepository.php', false]],
          config: [['services.yaml', false]],
          tests: [['ResolverTest.php', false]],
        }
        return (tree[a.path] ?? []).map(([name, dir]) => ({ name, dir }))
      }
      case 'write_project_file':
        files.set(a.path, a.text)
        return
      case 'git_file':
        return a.path.endsWith('.php') ? '<?php\n\nfinal class ChannelSegmentResolver\n{\n    public function resolve(CustomerInterface $customer): array\n    {\n        return [];\n    }\n}\n' : null
      case 'term_open': {
        const id = ++terminal
        setTimeout(() => emit('term-out', { id, data: prompt }), 50)
        return { id, shell: 'zsh' }
      }
      case 'term_write': {
        const id = Number(a.id)
        const data = a.data === '\r' ? `\r\n${line ? `ran: ${line}\r\n` : ''}${prompt}` : a.data === '\x7f' ? '\b \b' : a.data
        line = a.data === '\r' ? '' : a.data === '\x7f' ? line.slice(0, -1) : line + a.data
        emit('term-out', { id, data })
        return
      }
      case 'detect_editors':
        return [
          { name: 'Zed', command: '/home/quentin/.local/zed.app/bin/zed {project} {file}' },
          { name: 'PhpStorm', command: '/opt/phpstorm/bin/phpstorm.sh {project}' },
        ]
      case 'open_with':
        return
      case 'term_resize':
      case 'term_close':
        return
      case 'plugin:dialog|open':
        return '/home/quentin/agence/mercure'
      case 'plugin:dialog|ask':
        return true
    }
  },
  { shouldMockEvents: true },
)

const internals = (window as unknown as { __TAURI_INTERNALS__: { invoke: (cmd: string, args: any, options?: unknown) => Promise<unknown> } }).__TAURI_INTERNALS__
const mocked = internals.invoke
internals.invoke = async (cmd, args, options) => {
  if (cmd !== 'plugin:event|emit_to') return mocked(cmd, args, options)
  channel.postMessage({ target: args.target.label, event: args.event, payload: JSON.parse(JSON.stringify(args.payload ?? null)) })
}
