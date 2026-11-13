import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import taskList from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'
import subagents from '../../src-tauri/tests/fixtures/subagents.jsonl?raw'
import background from '../../src-tauri/tests/fixtures/background.jsonl?raw'
import backgroundBash from '../../src-tauri/tests/fixtures/background_bash.jsonl?raw'
import thinking from '../../src-tauri/tests/fixtures/thinking.jsonl?raw'
import advisorPlain from '../../src-tauri/tests/fixtures/advisor_plain.jsonl?raw'
import advisorRedacted from '../../src-tauri/tests/fixtures/advisor_redacted.jsonl?raw'
import picker from '../../src-tauri/src/artifact-pick.js?raw'

type Project = { name: string; path: string; rules: string[]; group?: string; cards?: unknown[]; color?: string; pinned?: boolean; archived?: boolean; runs?: boolean; runs_notify?: boolean; runs_hidden?: string[]; repository?: string; review_prompt?: string; defaults?: Record<string, string> }

const projects: Project[] = JSON.parse(localStorage.getItem('projects') ?? 'null') ?? [
  { name: 'Orbit', path: '/home/demo/agence/orbit', rules: [] },
  { name: 'Nova', path: '/home/demo/agence/nova', rules: [] },
  { name: 'Lumen', path: '/home/demo/agence/lumen-absent', rules: [] },
]

type Group = { id: string; name: string; parent: string }

const groups: Group[] = JSON.parse(localStorage.getItem('groups') ?? 'null') ?? (location.search.includes('groups')
  ? [{ id: 'g-clients', name: 'Clients', parent: '' }, { id: 'g-ring', name: 'Ring', parent: 'g-clients' }, { id: 'g-tools', name: 'Outils', parent: '' }]
  : [])
const saveGroups = () => localStorage.setItem('groups', JSON.stringify(groups))
const mockMcp = {
  on_demand: [{ name: 'sentry', description: 'Production errors of the shop', command: 'npx @sentry/mcp-server' }],
  everywhere: [{ name: 'chrome-devtools', description: '', command: 'npx chrome-devtools-mcp' }],
}
const label = location.search.includes('board') ? 'board' : location.search.includes('path=') ? 'file-mock' : location.search.includes('agent=') ? 'artifacts-mock' : 'main'
const channel = new BroadcastChannel('squadron-events')
channel.onmessage = ({ data }) => data.target === label && emit(data.event, data.payload)
let popup: Window | null = null

if (location.search.includes('many'))
  for (const name of ['Hermes', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    if (!projects.some((p) => p.name === name)) projects.push({ name, path: `/home/demo/agence/${name.toLowerCase()}`, rules: [] })

if (location.search.includes('groups'))
  for (const [name, group] of [['Orbit', 'g-ring'], ['Nova', 'g-ring'], ['Lumen', 'g-clients'], ['Hermes', 'g-tools'], ['Atlas', 'g-tools']]) {
    const found = projects.find((p) => p.name === name && p.group === undefined)
    if (found) found.group = group
  }

const COLORS = ['#A78BFA', '#2DD4BF', '#F472B6', '#60A5FA', '#A3E635', '#FACC15']

const folder = (name: string, path: string, rules: string[] = [], cards: unknown[] = [], color = COLORS[0]) => ({
  name,
  path,
  rules,
  cards,
  color,
  exists: !path.includes('absent'),
  git: !path.includes('nova'),
  branch: path.includes('nova') ? null : 'feat/T15-segmentation',
})

const views = () => projects.map((p, i) => ({ group: p.group ?? '', ...folder(p.name, p.path, p.rules, p.cards, (p.color ??= COLORS[i % COLORS.length])), pinned: !!p.pinned, archived: !!p.archived, runs: p.runs ?? true, runs_notify: p.runs_notify ?? true, runs_hidden: p.runs_hidden ?? [], repository: p.repository ?? '', review_prompt: p.review_prompt ?? '', defaults: p.defaults ?? {} }))

const loaded = Date.now()
const timers = new Map<string, number>()
let claudePath = 'claude'
let reviewPrompt = ''
let tabEffect = 'pulse'
let projectsEffect = 'snake'
let tabOrder = 'manual'
let tabHours = 4
let keyMap: Record<string, string> = {}
let editorApps: unknown[] = []
let language = 'system'
let runsConfig = { runs: true, runs_linger: 10, runs_hidden: [] as string[], runs_notify: false }

const asks: Record<string, [string, Record<string, unknown>]> = {
  permission: ['Bash', { command: 'lando composer update sylius/channel-segmentation-plugin', description: 'Updates the plugin' }],
  edit: ['Edit', { file_path: 'src/Segmentation/ChannelSegmentResolver.php', old_string: 'resolve(CustomerInterface $customer)', new_string: 'resolve(\n        CustomerInterface $customer,\n        ?ChannelInterface $channel = null,\n    )' }],
  mcp: ['mcp__github__create_pull_request', { owner: 'agence', repo: 'orbit', base: 'develop', head: 'feat/ORB-809', title: 'ORB-809 · Pilotage des batchs', draft: true }],
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
function swarm(n = 48) {
  const id = (i: number) => `toolu_swarm${i}`
  const name = (i: number) => ['SyliusCoreBundle', 'SyliusPaymentBundle', 'SyliusApiBundle', 'BmcSegmentationBundle', 'BmcBatchUtilsBundle'][i % 5] + ` ${i}`
  const lines: unknown[] = [
    {
      type: 'assistant',
      message: {
        id: 'msg_swarm',
        model: 'claude-opus-5-5',
        content: Array.from({ length: n }, (_, k) => ({ type: 'tool_use', id: id(k + 1), name: 'Agent', input: { description: name(k + 1), subagent_type: 'general-purpose', prompt: `List the PHP 8.5 deprecations of ${name(k + 1)}` } })),
      },
    },
  ]
  for (let i = 1; i <= n; i++)
    lines.push({ type: 'system', subtype: 'task_started', task_id: `task${i}`, tool_use_id: id(i), description: name(i), subagent_type: 'general-purpose', prompt: `List the PHP 8.5 deprecations of ${name(i)}.\nDo not edit any file.`, task_type: 'local_agent', is_backgrounded: false })
  for (let i = 1; i <= n; i++) {
    lines.push({ type: 'assistant', parent_tool_use_id: id(i), message: { id: `msg_s${i}`, model: 'claude-opus-5-5', content: [{ type: 'text', text: 'I look for the casts PHP 8.5 deprecates.' }, { type: 'tool_use', id: `toolu_grep${i}`, name: 'Grep', input: { pattern: '\\((integer|boolean|double)\\)', path: 'src/' } }] } })
    lines.push({ type: 'user', parent_tool_use_id: id(i), message: { content: [{ type: 'tool_result', tool_use_id: `toolu_grep${i}`, content: 'src/Model/Payment.php:143: return (integer) $this->amount;' }] } })
    lines.push({ type: 'system', subtype: 'task_progress', task_id: `task${i}`, tool_use_id: id(i), description: 'Grep (integer)|(boolean) in src/', usage: { total_tokens: 7000 + i * 113, tool_uses: 1, duration_ms: 40000 + i * 900 }, last_tool_name: 'Grep' })
  }
  for (let i = 1; i <= Math.floor(n * 0.45); i++) {
    const failed = i === 12 || i === 20
    lines.push({ type: 'system', subtype: 'task_updated', task_id: `task${i}`, patch: { status: failed ? 'failed' : 'completed', end_time: Date.now() } })
    lines.push({ type: 'system', subtype: 'task_notification', task_id: `task${i}`, tool_use_id: id(i), status: failed ? 'failed' : 'completed', summary: failed ? 'API Error: 529 Overloaded · 3 attempts' : `${name(i)}: 3 non-canonical casts, fixes proposed.`, usage: { total_tokens: 18000 + i * 211, tool_uses: 4, duration_ms: 120000 + i * 1000 } })
  }
  lines.push({ type: 'assistant', parent_tool_use_id: id(37), message: { id: 'msg_s37b', model: 'claude-opus-5-5', content: [{ type: 'tool_use', id: 'toolu_bash37', name: 'Bash', input: { command: 'lando php vendor/bin/phpstan analyse vendor/sylius/payment-bundle', description: 'Runs PHPStan on the bundle' } }] } })
  return lines.map((l) => JSON.stringify(l)).join('\n')
}

const waitingReplay = new Map<string, () => void>()
let askId = 0

const goalLines = new Map<string, string[]>()

function goal(id: string, condition: string) {
  const say = (text: string) => JSON.stringify({ type: 'assistant', message: { id: crypto.randomUUID(), model: '<synthetic>', content: [{ type: 'text', text }] } })
  const done = JSON.stringify({ type: 'result', is_error: false, num_turns: condition === 'clear' ? 0 : 3, total_cost_usd: 0, usage: {}, duration_ms: 4000 })
  const feedback = (n: number) =>
    JSON.stringify({ type: 'user', isSynthetic: true, message: { role: 'user', content: [{ type: 'text', text: `Stop hook feedback:\n[${condition}]: No test output in the transcript yet (check ${n}).` }] } })
  const relaunches = condition.includes('never') ? 9 : 1
  const lines = condition === 'clear' ? [say('Goal cleared: the goal'), done] : [say(`Goal set: ${condition}`), ...Array.from({ length: relaunches }, (_, n) => feedback(n + 1)), done]
  goalLines.set(id, lines)
  const next = () => {
    const line = goalLines.get(id)?.shift()
    if (!line) return timers.delete(id)
    emit('agent-line', { id, line })
    timers.set(id, window.setTimeout(next, 900))
  }
  const queued = () => (timers.has(id) ? window.setTimeout(queued, 200) : next())
  queued()
}

function replay(id: string, text: string) {
  if (text.includes('ssh')) setTimeout(() => emit('askpass-request', { id: String(askId++), agent: id, prompt: "The authenticity of host 'git.example.com' can't be established.\nED25519 key fingerprint is SHA256:d4Fq0nCp1vVfF1xGQm9cUqY3y8b1kz0rWbT9wq3E2gU.\nAre you sure you want to continue connecting (yes/no/[fingerprint])? ", command: 'ssh -T git@git.example.com' }), 400)
  if (text.startsWith('/goal ')) return goal(id, text.slice(6))
  if (text.startsWith('/config') || text.startsWith('/mcp')) {
    const out = text.startsWith('/mcp') ? '4 MCP server(s): 1 connected, 2 not connected, 1 disabled. Use `/mcp` in the terminal for details.' : text.includes('=') ? `Set ${text.slice(8).replace('=', ' to ')}` : configUsage
    emit('agent-line', { id, line: JSON.stringify({ type: 'user', isReplay: true, message: { role: 'user', content: [{ type: 'text', text: `<local-command-stdout>${out}</local-command-stdout>` }] } }) })
    emit('agent-line', { id, line: JSON.stringify({ type: 'result', is_error: false, num_turns: 0, total_cost_usd: 0, usage: {}, duration_ms: 20 }) })
    return
  }
  const key = !text.includes('team-share:') && Object.keys(asks).find((k) => text.includes(k))
  if (key) {
    const [tool_name, input] = asks[key]
    const ask = String(askId++)
    waitingReplay.set(ask, () => replay(id, 'ok'))
    emit('permission-request', { id: ask, agent: id, tool_name, input: { ...input, file_path: input.file_path ? `${projectOf.get(id)}/${input.file_path}` : undefined }, tool_use_id: null })
    return
  }
  if (claudePath === 'claude' && text.includes('notfound')) throw 'claude-not-found'
  if (text.startsWith('/compact')) {
    emit('agent-line', { id, line: JSON.stringify({ type: 'system', subtype: 'compact_boundary', compact_metadata: { pre_tokens: 182_000, post_tokens: 21_000 } }) })
    emit('agent-line', { id, line: JSON.stringify({ type: 'result', is_error: false, num_turns: 0, total_cost_usd: 0, usage: {}, duration_ms: 3000 }) })
    return
  }
  const shot = 'Voici la page :\\n\\n![Accueil](/captures/home.png)\\n\\nEt une capture effacée : ![Ancienne](/captures/old.png)'
  files.set('/captures/home.png', `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="640" height="360" fill="#2a2420"/><text x="40" y="190" font-size="40" fill="#e8875a">Capture</text></svg>')}`)
  const page = () => {
    const canvas = Object.assign(document.createElement('canvas'), { width: 1280, height: 800 })
    const g = canvas.getContext('2d')!
    g.fillStyle = '#1d1a18'
    g.fillRect(0, 0, 1280, 800)
    g.fillStyle = '#e8875a'
    g.fillRect(0, 0, 1280, 90)
    g.fillStyle = '#3a332e'
    for (let y = 140; y < 760; y += 120) g.fillRect(60, y, 1160, 90)
    return canvas.toDataURL('image/png').split(',')[1]
  }
  const shots = () =>
    tools
      .trim()
      .split('\n')
      .map((l) => {
        const d = JSON.parse(l)
        if (d.type !== 'user' || d.message?.content?.[0]?.type !== 'tool_result') return l
        d.message.content[0].content = Array.from({ length: text.includes('many') ? 5 : 1 }, () => ({ type: 'image', source: { type: 'base64', media_type: 'image/png', data: page() } }))
        return JSON.stringify(d)
      })
      .join('\n')
  const claudeDoc = () => {
    let first = ''
    return tools
      .trim()
      .split('\n')
      .map((l) => {
        const d = JSON.parse(l)
        const start = d.event?.type === 'content_block_start' && d.event.content_block.type === 'tool_use' ? d.event.content_block : null
        if (start && (!first || start.id === first)) {
          first = start.id
          start.name = 'mcp__claude_ai_Claude_Docs__batch'
          return JSON.stringify(d)
        }
        const use = d.type === 'assistant' ? d.message?.content?.find((c: { type: string; id: string }) => c.type === 'tool_use' && c.id === first) : null
        if (use) {
          use.name = 'mcp__claude_ai_Claude_Docs__batch'
          use.input = { container: { kind: 'project', create: { name: 'Plan de migration', doc: { markdown: '# Plan de migration' } } }, batch: [] }
          return JSON.stringify(d)
        }
        if (d.type === 'user' && d.message?.content?.[0]?.tool_use_id === first) {
          d.message.content[0].content = 'created.bound:true\nhttps://claude.ai/artifact/plan-de-migration-3f2a9c1e-5b7d-4e8a-9c21-7d4e5f6a8b90'
          return JSON.stringify(d)
        }
        return l
      })
      .join('\n')
  }
  const forkShare = () => {
    const artifact = /args: artifact (\S+)/.exec(text)?.[1] ?? 'artifact'
    const destination = /destination (\S+?)\.\s/.exec(text)?.[1] ?? 'team'
    const steps = ['Reading the artifact', 'Copying the kit', 'Writing the sync layer', 'Checking the page', 'Creating the Apps Script', 'Deploying the web app']
    const say = (content: unknown[], parent: string | null, model = 'claude-sonnet-5-5') => ({ type: 'assistant', parent_tool_use_id: parent, message: { id: crypto.randomUUID(), model, role: 'assistant', content } })
    const result = (id: string, content: string, parent: string | null) => ({ type: 'user', parent_tool_use_id: parent, message: { role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content }] } })
    const link = 'https://script.google.com/a/macros/example.com/s/AKfycbxDemo/exec'
    const out: unknown[] = [
      say([{ type: 'tool_use', id: 'toolu_fork', name: 'Skill', input: { skill: 'team-share:share-artifact', args: artifact } }], null, 'claude-opus-5-5'),
      { type: 'system', subtype: 'task_started', task_type: 'local_agent', task_id: 'tfork', tool_use_id: 'toolu_fork', description: '/team-share:share-artifact', subagent_type: 'general-purpose', prompt: artifact },
    ]
    steps.forEach((message, done) => {
      out.push({ type: 'mock_pilot', tool: 'set_progress', args: { artifact, done, total: steps.length, message } })
      const id = `toolu_step${done}`
      const call = done === 5 ? { type: 'tool_use', id, name: 'Bash', input: { command: `python3 share.py deploy W "${artifact}"`, description: 'Deploys the page as a web app from your Google account' } } : { type: 'tool_use', id, name: 'Read', input: { file_path: `/work/${artifact}/step${done}.txt` } }
      out.push(say([call], 'toolu_fork'), result(id, 'ok', 'toolu_fork'))
    })
    out.push(
      { type: 'mock_pilot', tool: 'publish_artifact', args: { id: artifact, destination, link } },
      say([{ type: 'text', text: `Shared with the team at ${link}` }], 'toolu_fork'),
      { type: 'system', subtype: 'task_notification', task_id: 'tfork', status: 'completed', summary: '/team-share:share-artifact' },
      result('toolu_fork', `Shared with the team at ${link}`, null),
      say([{ type: 'text', text: 'The artifact is shared with the team.' }], null, 'claude-opus-5-5'),
      { type: 'result', subtype: 'success', is_error: false, result: 'The artifact is shared with the team.', total_cost_usd: 0.04, usage: { input_tokens: 10, output_tokens: 20 } },
    )
    return out.map((d) => JSON.stringify(d)).join('\n')
  }
  const links = 'J\'ai poussé la correction et ouvert la PR :\\n\\nhttps://bitbucket.org/orbit/orbit/pull-requests/2108\\n\\nLa PR https://github.com/agence/nova/pull/57 dont elle dépend est fusionnée, et [celle-ci](https://github.com/agence/nova/pull/61) bloque encore.\\n\\nhttps://github.com/autre-org/prive/pull/12\\n\\nJe surveille la CI.'
  const stream = text.includes('team-share:share-artifact') ? forkShare() : text.includes('plugins') ? pluginRun() : text.includes('advisorplain') ? advisorPlain : text.includes('advisor') ? advisorRedacted : text.includes('inlinepath') ? hello.replaceAll('"Bonjour"', JSON.stringify('J’ai mis à jour `config/services.yaml` et `src/Segmentation/SegmentRepository.php:12`, puis relu `config/services.yaml` et `absent.md`.')) : text.includes('filecard') ? hello.replaceAll('"Bonjour"', JSON.stringify('Voici l’archive :\n\n[squadron-claude-design.zip](</home/demo/Téléchargements/squadron-claude-design.zip>)\n\nEt la liste des URL :\n\n[neogeofanatic-urls.csv](</home/demo/Documents/neogeofanatic-urls.csv>)')) : text.includes('bashblock') ? hello.replaceAll('"Bonjour"', JSON.stringify('Il reste à recharger nginx :\n\n```bash\nsudo nginx -t && sudo systemctl reload nginx\n```')) : text.includes('links') ? hello.replaceAll('"Bonjour"', `"${links}"`) : text.includes('shots') ? shots() : text.includes('claudedoc') ? claudeDoc() : text.includes('capture') ? hello.replaceAll('"Bonjour"', `"${shot}"`) : text.includes('think') ? thinking : text.includes('swarm') ? swarm() : text.includes('subagents') ? subagents : text.includes('bgshell') ? backgroundBash : text.includes('background') ? background : text.includes('tasks') ? taskList : text.includes('tool') ? tools : text.includes('logout') ? nologin : text.includes('long') ? sigint : hello
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
    const line = lines[i++]
    if (line.startsWith('{"type":"mock_pilot"')) {
      const { tool, args } = JSON.parse(line)
      ;(window as unknown as { __pilot: (agent: string, tool: string, args: unknown) => Promise<unknown> }).__pilot(id, tool, args).finally(() => timers.set(id, window.setTimeout(next, 700)))
      return
    }
    emit('agent-line', { id, line: line.replaceAll('/project', projectOf.get(id) ?? '/project') })
    const sub = JSON.parse(line)
    const call = sub.type === 'assistant' && sub.parent_tool_use_id && sub.message.content.find((c: { name?: string }) => c.name === 'Bash')
    if (call) {
      const ask = String(askId++)
      waitingReplay.set(ask, next)
      emit('permission-request', { id: ask, agent: id, tool_name: 'Bash', input: call.input, tool_use_id: call.id })
      return
    }
    if (text.includes('bgshell') && sub.type === 'result' && i < lines.length) return timers.set(id, window.setTimeout(next, 20_000))
    timers.set(id, window.setTimeout(next, lines[i]?.includes('"type":"advisor_tool_result"') ? 5000 : text.includes('long') || text.includes('tasks') || text.includes('subagents') || text.includes('background') ? 150 : text.includes('swarm') ? 8 : text.includes('team-share:') ? 500 : 60))
  }
  next()
}

const PLUGIN_DIR = '/home/demo/.claude/plugins/cache'

const CONTEXT_DEMO = {
  totalTokens: 40_480,
  categories: [
    { id: 'system_prompt', tokens: 3_150 },
    { id: 'system_tools', tokens: 6_210 },
    { id: 'mcp_tools', tokens: 2_740 },
    { id: 'mcp_instructions', tokens: 1_620 },
    { id: 'mcp_tools_deferred', tokens: 110_000 },
    { id: 'custom_agents', tokens: 380 },
    { id: 'memory_files', tokens: 2_620 },
    { id: 'skills', tokens: 1_830 },
    { id: 'messages', tokens: 21_930 },
  ],
  skills: {
    skillFrontmatter: [
      { name: 'loop', source: 'built-in', tokens: 100 },
      { name: 'code-review', source: 'built-in', tokens: 110 },
      { name: 'pdf', source: 'syncedSkills', tokens: 110 },
      { name: 'chiffrage', source: 'userSettings', tokens: 100 },
      { name: 'impeccable:impeccable', source: 'plugin', pluginName: 'impeccable', tokens: 110 },
      { name: 'shopify-plugin:shopify', source: 'plugin', pluginName: 'shopify-plugin', tokens: 110 },
      { name: 'shopify-plugin:ucp', source: 'plugin', pluginName: 'shopify-plugin', tokens: 110 },
      { name: 'ponytail:ponytail', source: 'plugin', pluginName: 'ponytail', tokens: 110 },
      { name: 'ponytail:ponytail-review', source: 'plugin', pluginName: 'ponytail', tokens: 110 },
      { name: 'codegraph:explorer', source: 'plugin', pluginName: 'codegraph', tokens: 70 },
      { name: 'other', source: 'built-in', tokens: 790 },
    ],
  },
  agents: [
    { agentType: 'impeccable:impeccable-documenter', source: 'plugin', pluginId: 'impeccable@impeccable', tokens: 70 },
    { agentType: 'impeccable:impeccable-finish-reviewer', source: 'plugin', pluginId: 'impeccable@impeccable', tokens: 80 },
    { agentType: 'visual-review', source: 'userSettings', tokens: 230 },
  ],
  memoryFiles: [
    { path: '/home/demo/.claude/CLAUDE.md', type: 'User', tokens: 900 },
    { path: '/home/demo/project/CLAUDE.md', type: 'Project', tokens: 840 },
    { path: '/home/demo/.claude/projects/project/memory/MEMORY.md', type: 'AutoMem', tokens: 880 },
  ],
  mcpTools: [
    { name: 'mcp__plugin_codegraph_codegraph__codegraph_explore', serverName: 'plugin_codegraph_codegraph', tokens: 630, isLoaded: true },
    { name: 'mcp__squadron__permission', serverName: 'squadron', tokens: 410, isLoaded: true },
    { name: 'mcp__squadron-pilot__overview', serverName: 'squadron-pilot', tokens: 1_700, isLoaded: true },
  ],
  mcpInstructions: [
    { serverName: 'plugin:codegraph:codegraph', tokens: 510 },
    { serverName: 'squadron-pilot', tokens: 680 },
    { serverName: 'sentry-selfhosted', tokens: 280 },
    { serverName: 'agent-browser', tokens: 150 },
  ],
  messageBreakdown: {
    toolCallsByType: [
      { name: 'Read', callTokens: 60, resultTokens: 2_400 },
      { name: 'Bash', callTokens: 180, resultTokens: 3_100 },
      { name: 'mcp__plugin_codegraph_codegraph__codegraph_explore', callTokens: 40, resultTokens: 2_150 },
      { name: 'Skill', callTokens: 30, resultTokens: 20 },
    ],
    assistantMessageTokens: 3_200,
    userMessageTokens: 900,
    attachmentsByType: [
      { name: 'hook_success', tokens: 2_900 },
      { name: 'skill_listing', tokens: 4_200 },
      { name: 'agent_listing_delta', tokens: 1_700 },
      { name: 'deferred_tools_delta', tokens: 8_000 },
    ],
  },
}

function pluginRun() {
  const [init, ...rest] = hello.trim().split('\n')
  const head = JSON.parse(init)
  head.plugins = [
    { name: 'impeccable', path: `${PLUGIN_DIR}/impeccable/impeccable/4.6.0`, source: 'impeccable@impeccable', version: '4.6.0' },
    { name: 'shopify-plugin', path: `${PLUGIN_DIR}/claude-plugins-official/shopify-ai-toolkit/2.1.2`, source: 'shopify-ai-toolkit@claude-plugins-official', version: '2.1.2' },
    { name: 'ponytail', path: `${PLUGIN_DIR}/ponytail/ponytail/5.3.2`, source: 'ponytail@ponytail', version: '5.3.2' },
    { name: 'codegraph', path: '/home/demo/codegraph-plugin', source: 'codegraph@inline' },
    { name: 'cc-plugin-telemetry', path: 'builtin', source: 'cc-plugin-telemetry@builtin' },
  ]
  const at = () => new Date().toISOString()
  const say = (id: string, text: string) => JSON.stringify({ type: 'assistant', timestamp: at(), message: { id: `msg_${id}`, model: 'claude-haiku-5-5', content: [{ type: 'text', text }] } })
  const tool = (id: string, name: string, input: object) => JSON.stringify({ type: 'assistant', timestamp: at(), message: { id: `msg_${id}`, model: 'claude-haiku-5-5', content: [{ type: 'tool_use', id, name, input }] } })
  const result = (id: string, text: string) => JSON.stringify({ type: 'user', message: { content: [{ type: 'tool_result', tool_use_id: id, content: text }] } })
  const body = (dir: string, size: number) => JSON.stringify({ type: 'user', isSynthetic: true, message: { content: [{ type: 'text', text: `Base directory for this skill: ${dir}\n${'Instructions. '.repeat(size)}` }] } })
  return [
    JSON.stringify({ type: 'system', subtype: 'hook_response', hook_event: 'SessionStart', output: `PONYTAIL MODE ACTIVE — level: full\n${'Rules. '.repeat(700)}` }),
    JSON.stringify(head),
    say('p0', 'Je regarde d’abord la page produit, puis je lance une passe de polish.'),
    tool('p1', 'Read', { file_path: '/project/src/sections/product-main.liquid' }),
    result('p1', 'liquid '.repeat(300)),
    tool('p2', 'mcp__plugin_codegraph_codegraph__codegraph_explore', { query: 'ProductForm variant picker' }),
    result('p2', 'symbol '.repeat(850)),
    tool('p3', 'Skill', { skill: 'impeccable:impeccable', args: 'polish' }),
    result('p3', 'Launching skill: impeccable:impeccable'),
    body(`${PLUGIN_DIR}/impeccable/impeccable/4.6.0/skills/impeccable`, 800),
    tool('p4', 'Read', { file_path: `${PLUGIN_DIR}/impeccable/impeccable/4.6.0/skills/impeccable/reference/polish.md` }),
    result('p4', 'polish '.repeat(800)),
    tool('p5', 'Edit', { file_path: '/project/src/sections/product-main.liquid', old_string: 'gap: 4px', new_string: 'gap: 12px' }),
    result('p5', 'The file has been updated.'),
    say('p6', 'Le prix barré manque de contraste, je corrige. Je passe à la fiche Shopify.'),
    tool('p7', 'Skill', { skill: 'shopify-plugin:shopify' }),
    result('p7', 'Launching skill: shopify-plugin:shopify'),
    body(`${PLUGIN_DIR}/claude-plugins-official/shopify-ai-toolkit/2.1.2/skills/shopify`, 650),
    tool('p8', 'mcp__plugin_shopify-plugin_shopify-dev__graphql', { query: 'product(handle: "veste-lin") { variants { title } }' }),
    result('p8', JSON.stringify({ variants: ['S', 'M', 'L', 'XL'].map((title) => ({ title, sku: `VL-${title}` })) }).repeat(12)),
    ...rest.map((l) => l.replaceAll('"Bonjour"', JSON.stringify('Les 4 variantes remontent. J’ai resserré l’espacement du sélecteur et corrigé le contraste du prix barré.'))),
  ].join('\n')
}

type MockPlugin = { id: string; name: string; version: string; scope: string; enabled: boolean; installPath: string; description: string }

const mockPlugins: MockPlugin[] = [
  { id: 'impeccable@impeccable', name: 'impeccable', version: '4.6.0', scope: 'user', enabled: true, installPath: `${PLUGIN_DIR}/impeccable/impeccable/4.6.0`, description: 'Design fluency for frontend development. 1 skill with 24 commands and curated anti-pattern detection.' },
  { id: 'shopify-ai-toolkit@claude-plugins-official', name: 'shopify-plugin', version: '2.1.2', scope: 'user', enabled: false, installPath: `${PLUGIN_DIR}/claude-plugins-official/shopify-ai-toolkit/2.1.2`, description: 'Build anything on Shopify: CLI, Admin API, Functions, Polaris extensions.' },
  { id: 'ponytail@ponytail', name: 'ponytail', version: '5.3.2', scope: 'user', enabled: true, installPath: `${PLUGIN_DIR}/ponytail/ponytail/5.3.2`, description: 'Forces the laziest solution that works. YAGNI, stdlib first, one line over fifty.' },
  { id: 'crew@jgrasp-marketplace', name: 'crew', version: '8a206b3', scope: 'user', enabled: false, installPath: `${PLUGIN_DIR}/jgrasp-marketplace/crew/8a206b3`, description: 'Architect, challenger, devs and testers driven by a PO agent.' },
]
const pluginFlags: { user: Record<string, boolean>; projects: Record<string, Record<string, boolean>> } = {
  user: { 'impeccable@impeccable': true, 'shopify-ai-toolkit@claude-plugins-official': false, 'ponytail@ponytail': true, 'crew@jgrasp-marketplace': false },
  projects: { '/home/demo/agence/orbit': { 'shopify-ai-toolkit@claude-plugins-official': true }, '/home/demo/agence/nova': { 'impeccable@impeccable': false } },
}
const pluginDetails: Record<string, { always: number; components: { kind: string; name: string; always: number; invoke: number }[] }> = {
  'impeccable@impeccable': { always: 439, components: [{ kind: 'skill', name: 'impeccable', always: 220, invoke: 3700 }, { kind: 'agent', name: 'impeccable-documenter', always: 60, invoke: 960 }, { kind: 'agent', name: 'impeccable-finish-reviewer', always: 70, invoke: 5200 }] },
  'shopify-ai-toolkit@claude-plugins-official': { always: 2100, components: [{ kind: 'skill', name: 'shopify', always: 1300, invoke: 9000 }, { kind: 'skill', name: 'ucp', always: 500, invoke: 4000 }, { kind: 'mcp', name: 'shopify-dev', always: 300, invoke: 0 }] },
  'ponytail@ponytail': { always: 641, components: [{ kind: 'skill', name: 'ponytail', always: 140, invoke: 1100 }, { kind: 'skill', name: 'ponytail-review', always: 150, invoke: 1700 }] },
}

const projectOf = new Map<string, string>()
const mcpServers: { name: string; status: string; error?: string }[] = [
  { name: 'bitbucket', status: 'connected' },
  { name: 'claude.ai Figma', status: 'needs-auth' },
  { name: 'codegraph', status: 'failed', error: 'spawn ENOENT' },
  { name: 'sentry', status: 'disabled' },
]
const configUsage = `Usage: /config key=value

  autoCompact=true|false
  checkpoints=true|false
  language=<value>
  model=default|opus|sonnet|haiku|fable
  outputStyle=default|Explanatory|Learning|Concise
  theme=dark|light|dark-daltonized|light-daltonized|dark-ansi|light-ansi
  verbose=true|false
  workflowSizeGuideline=small|medium|large|unlimited
`
const catalog = {
  type: 'control_response',
  response: {
    subtype: 'success',
    response: {
      commands: [
        { name: 'compact', description: 'Free up context by summarizing the conversation so far', argumentHint: '<optional custom summarization instructions>', builtin: true },
        { name: 'clear', description: 'Start a new session with empty context', argumentHint: '[name]', aliases: ['reset', 'new'], builtin: true },
        { name: 'context', description: 'Visualize current context usage', builtin: true },
        { name: 'model', description: 'Set the AI model for Claude Code', argumentHint: '<model>', builtin: true },
        { name: 'effort', description: 'Set effort level for model usage', argumentHint: '<low|medium|high|xhigh|max|auto|ultracode [on|off]>', builtin: true },
        { name: 'fast', description: 'Toggle fast mode', argumentHint: '[on|off]', builtin: true },
        { name: 'mcp', description: 'Manage MCP servers', argumentHint: '[reconnect|enable|disable [<server>|all]]', builtin: true },
        { name: 'config', description: 'Change a Claude Code setting', argumentHint: 'key=value', builtin: true },
        { name: 'output-style', description: 'List output styles or switch to one', argumentHint: '[style]', builtin: true },
        { name: 'ponytail:ponytail', description: '(ponytail) Lazy senior dev mode', argumentHint: '[lite|full|ultra]' },
        { name: 'code-review', description: 'Review the current diff', argumentHint: '[target]' },
        { name: 'codegraph:setup', description: '(codegraph) Install and configure CodeGraph on a project' },
        { name: 'greet', description: 'Greet the user in a given tone', argumentHint: '<warm|formal|pirate>' },
      ],
      models: [
        { value: 'default', displayName: 'Default (recommended)', description: 'Opus 5.5 · Best for everyday, complex tasks' },
        { value: 'opus', displayName: 'Opus 5.5', description: 'Best for everyday, complex tasks' },
        { value: 'fable', displayName: 'Fable 5.1', description: 'Most capable for your hardest and longest-running tasks' },
        { value: 'sonnet', displayName: 'Sonnet 5.5', description: 'Fast and capable' },
        { value: 'haiku', displayName: 'Haiku 5.5', description: 'Fastest for quick answers' },
        { value: 'claude-haiku-4-5-20251001', displayName: 'Haiku 4.5', description: 'Fastest for quick answers' },
        { value: 'claude-opus-5', displayName: 'Opus 5', description: 'Best for everyday, complex tasks' },
        { value: 'claude-opus-4-7', displayName: 'Opus 4.7', description: 'Best for everyday, complex tasks' },
        { value: 'claude-opus-4-6', displayName: 'Opus 4.6', description: 'Best for everyday, complex tasks', supportedEffortLevels: ['low', 'medium', 'high', 'max'] },
      ],
      available_output_styles: ['default', 'Explanatory', 'Learning'],
      agents: [
        { name: 'general-purpose', description: 'General-purpose agent for researching complex questions and executing multi-step tasks.' },
        { name: 'Explore', description: 'Read-only search agent for broad fan-out searches.', model: 'haiku' },
        { name: 'reviewer', description: 'Reviews the diff of a task against its plan and the project conventions. Never fixes the code.' },
        { name: 'crew:architect', description: 'Explores the code, proposes the technical POC of a request, then writes the step-by-step plan. Never changes the code.', model: 'opus' },
        { name: 'crew:dev-back', description: 'Back-end developer. Carries out one step of the plan and commits it once its checks pass.' },
      ],
    },
  },
}
const replies = new Map<string, (result: string) => void>()
;(window as unknown as { __pilot: unknown }).__pilot = (agent: string, tool: string, args = {}) =>
  new Promise((resolve) => {
    const id = crypto.randomUUID()
    replies.set(id, resolve)
    emit('pilot-request', { id, agent, tool, args })
  })
const files = new Map<string, string>()
type Shelved = { id: string; kind: 'html' | 'markdown' | 'svg'; entry: string; title: string; modified: number; link: string | null; links?: Record<string, { link: string; at: number }>; keep?: number | null; content: string }
const shelves = new Map<string, Shelved[]>(JSON.parse(localStorage.getItem('shelves') ?? '[]'))
const shelve = () => localStorage.setItem('shelves', JSON.stringify([...shelves]))
const ENTRY = { html: 'index.html', markdown: 'index.md', svg: 'index.svg' }
const titled = (kind: string, content: string, id: string) =>
  (kind === 'markdown' ? /^# (.+)$/m.exec(content)?.[1] : /<title>([^<]*)<\/title>/i.exec(content)?.[1])?.trim() || id
;(window as unknown as { __artifactUrl: unknown }).__artifactUrl = (agent: string, artifact: { id: string }) => {
  const found = shelves.get(agent)?.find((a) => a.id === artifact.id)
  const body = found?.kind === 'html' ? found.content.replace(/<\/body>|$/i, (end) => `<script>${picker}<\/script>${end}`) : (found?.content ?? '')
  return URL.createObjectURL(new Blob([body], { type: found?.kind === 'svg' ? 'image/svg+xml' : 'text/html;charset=utf-8' }))
}
let terminal = 0
let line = ''
const shells = new Map<number, number>()
const prompt = '\x1b[38;2;154;214;203mdemo\x1b[0m \x1b[2m~/agence/orbit\x1b[0m \x1b[38;2;201;166;255mfeat/T15-segmentation*\x1b[0m\r\n\x1b[38;2;224;138;91m❯\x1b[0m '

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

const orbitPull = () => ({
              host: 'bitbucket',
              number: 2108,
              title: 'feat(segments): channel-aware segment resolver',
              description: '## Result\n\n- The resolver takes the channel into account\n- `ResolverTest` covers a missing channel\n\n## Tests\n\n```bash\nvendor/bin/phpunit tests/Segmentation\n```',
              url: 'https://bitbucket.org/orbit/orbit/pull-requests/2108',
              state: 'open',
              draft: false,
              approvals: 1,
              changes_requested: false,
              comments: 3,
              checks: 'passing',
              branch: 'feat/orbit',
              base: 'main',
              head: 'b'.repeat(40),
              author: 'Quentin Briais',
              author_avatar: '',
              updated: new Date().toISOString(),
              reviewed: null,
              conflicts: false,
              reviewers: [
                { name: 'Sébastien Leprovost', avatar: '', verdict: 'approved' },
                { name: 'Adrien Dhermy', avatar: 'https://avatars.githubusercontent.com/u/9919?s=64', verdict: 'waiting' },
              ],
            })

mockWindows(label)
const JIRA_TOKENS = 'https://id.atlassian.com/manage-profile/security/api-tokens'

const DEMO_ACCOUNTS = '{"jira":[{"id":"3f9a1c2e","values":{"site":"acme.atlassian.net","email":"demo@acme.com"},"host":"acme.atlassian.net","secrets":["token"]}]}'

mockIPC(
  (cmd, args) => {
    const a = args as Record<string, string>
    switch (cmd) {
      case 'list_projects':
        return views()
      case 'list_groups':
        return groups
      case 'add_group':
        groups.push({ id: a.id, name: a.name, parent: a.parent })
        saveGroups()
        return { groups, projects: views() }
      case 'edit_group': {
        const group = groups.find((g) => g.id === a.id)!
        if (a.name !== null) group.name = a.name
        if (a.parent !== null) group.parent = a.parent
        saveGroups()
        return { groups, projects: views() }
      }
      case 'remove_group': {
        const gone = groups.find((g) => g.id === a.id)!
        for (const g of groups) if (g.parent === gone.id) g.parent = gone.parent
        for (const p of projects) if (p.group === gone.id) p.group = gone.parent
        groups.splice(groups.indexOf(gone), 1)
        saveGroups()
        localStorage.setItem('projects', JSON.stringify(projects))
        return { groups, projects: views() }
      }
      case 'move_projects':
        for (const p of projects) if ((args as unknown as { paths: string[] }).paths.includes(p.path)) p.group = a.group
        localStorage.setItem('projects', JSON.stringify(projects))
        return { groups, projects: views() }
      case 'inspect_folder':
        return {
          ...folder(a.path.split('/').pop() ?? '', a.path),
          taken_by: projects.find((p) => p.path === a.path)?.name ?? null,
          taken_archived: !!projects.find((p) => p.path === a.path)?.archived,
          near: a.path.includes('absent') ? a.path.replace(/-?absent.*$/, '') : null,
        }
      case 'add_project':
        projects.push({ name: a.name, path: a.path, rules: [] })
        return views()
      case 'scratch_project': {
        const path = '/home/user/.local/share/squadron/conversations'
        const found = projects.find((p) => p.path === path)
        if (found) found.archived = false
        else projects.push({ name: a.name, path, rules: [] })
        return [path, views()]
      }
      case 'remove_project':
        projects.splice(
          projects.findIndex((p) => p.path === a.path),
          1,
        )
        return views()
      case 'move_project':
        projects.find((p) => p.path === a.path)!.path = a.to
        return views()
      case 'agent_definition':
        return a.name === 'crew:architect'
          ? {
              path: '/home/demo/.claude/plugins/cache/demo/crew/1.0.0/agents/architect.md',
              source: 'plugin',
              plugin: 'crew',
              description: 'Explores the code, proposes the technical POC of a request, then writes the step-by-step plan. Never changes the code.',
              model: 'opus',
              effort: 'high',
              tools: ['Read', 'Glob', 'Grep', 'Write', 'Edit', 'mcp__plugin_codegraph_codegraph__codegraph_explore'],
              skills: ['codegraph:explorer'],
              body: '# Role\n\nYou are the architect. You explore the code and design: first a POC, then the plan.\n\n## Principles\n\n- Everything you state about the code, you checked in the code. Quote the file path.\n- Reuse what exists and follow the conventions of the project.\n\n## POC\n\nRead the files the delegation points to, then write the POC:\n\n- **Approach**: the solution you propose.\n- **Files**: to create and to change, with their paths.\n\n## Plan\n\nSplit the work into steps that each leave the project green.\n\n## Answer\n\nFive lines at most: `OK`, `UNCHANGED` or `BLOCKED` with the reason.',
            }
          : a.name === 'reviewer'
            ? { path: `${a.project}/.claude/agents/reviewer.md`, source: 'project', plugin: null, description: 'Reviews the diff of a task against its plan and the project conventions. Never fixes the code.', model: null, effort: null, tools: null, skills: [], body: 'Review the diff.' }
            : null
      case 'create_agent':
        projectOf.set(a.id, a.project)
        emit('agent-line', { id: a.id, line: JSON.stringify(catalog) })
        if ((a.options as unknown as { agent?: string }).agent)
          emit('agent-line', {
            id: a.id,
            line: JSON.stringify({ type: 'system', subtype: 'init', session_id: crypto.randomUUID(), slash_commands: ['compact', 'clear', 'ponytail:ponytail', 'code-review', 'codegraph:setup', 'greet'], skills: ['ponytail:ponytail', 'code-review', 'codegraph:setup', 'greet'], tools: (a.options as unknown as { agent?: string }).agent === 'crew:architect' ? ['Read', 'Glob', 'Grep', 'Write', 'Edit', 'mcp__plugin_codegraph_codegraph__codegraph_explore'] : ['Bash', 'Read', 'Edit', 'Write', 'WebFetch', 'Agent'] }),
          })
        return a.text.trim() ? replay(a.id, a.text) : undefined
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
      case 'open_link':
        window.open(a.url, '_blank')
        return
      case 'extensions_field_link':
        window.open(JIRA_TOKENS, '_blank')
        return
      case 'link_preview':
        return a.url.includes('github.com')
          ? { title: 'GitHub - nektos/act: Run your GitHub Actions locally 🚀', description: 'Run your GitHub Actions locally 🚀. Contribute to nektos/act development by creating an account on GitHub.', site: 'GitHub', image: 'https://opengraph.githubassets.com/c5edb8ab3e5c401b76cfb5316227093cdc7d5114a362915f2d72274d48976101/nektos/act', icon: 'https://github.githubassets.com/favicons/favicon.png' }
          : { title: 'Usage guide - act - User Guide', description: '', site: new URL(a.url).host, image: null, icon: 'https://nektosact.com/favicon.svg' }
      case 'plugins_installed':
        return mockPlugins.map((p) => ({ ...p, enabled: pluginFlags.user[p.id] ?? false }))
      case 'plugins_marketplaces':
        return [
          { name: 'claude-plugins-official', source: 'github', repo: 'anthropics/claude-plugins-official' },
          { name: 'impeccable', source: 'github', repo: 'pbakaus/impeccable' },
          { name: 'ponytail', source: 'github', repo: 'ponytail-dev/ponytail' },
          { name: 'jgrasp-marketplace', source: 'directory', path: '/home/demo/Projects/marketplace' },
        ]
      case 'plugins_catalog':
        return new Promise((resolve) =>
          setTimeout(() => resolve([
            { id: '42crunch-api-security-testing@claude-plugins-official', name: '42crunch-api-security-testing', description: 'Audit OpenAPI specs, detect OWASP API Security risks, and apply AI-powered fixes.', marketplace: 'claude-plugins-official', installs: 3474 },
            { id: 'security-guidance@claude-plugins-official', name: 'security-guidance', description: 'Warns about risky patterns when editing files: command injection, XSS, unsafe deserialization.', marketplace: 'claude-plugins-official', installs: 28100 },
            { id: 'shopify-ai-toolkit@claude-plugins-official', name: 'shopify-ai-toolkit', description: 'Build anything on Shopify.', marketplace: 'claude-plugins-official', installs: 9120 },
          ]), 400),
        )
      case 'plugin_details':
        return pluginDetails[a.id] ?? Promise.reject(`Plugin "${a.id}" not found.`)
      case 'plugin_flags':
        return { user: pluginFlags.user, projects: Object.fromEntries(((args as { projects: string[] }).projects).map((p) => [p, pluginFlags.projects[p] ?? {}])) }
      case 'plugin_run': {
        const [verb, id, , scope] = (args as { args: string[] }).args
        const dir = (args as { dir: string | null }).dir
        if (verb === 'enable' || verb === 'disable') {
          if (scope === 'user') pluginFlags.user[id] = verb === 'enable'
          else if (dir) pluginFlags.projects[dir] = { ...pluginFlags.projects[dir], [id]: verb === 'enable' }
        } else if (verb === 'install') {
          mockPlugins.push({ id, name: id.split('@')[0], version: '1.0.0', scope: 'user', enabled: true, installPath: `${PLUGIN_DIR}/${id}`, description: 'Freshly installed.' })
          pluginFlags.user[id] = true
        } else if (verb === 'uninstall') mockPlugins.splice(mockPlugins.findIndex((p) => p.id === id), 1)
        return new Promise((resolve) => setTimeout(() => resolve('✔ Done'), 250))
      }
      case 'claude_config':
        return new Promise((done) => setTimeout(() => done(a.setting ? `Set ${a.setting.replace('=', ' to ')}` : configUsage), 600))
      case 'stop_task': {
        clearTimeout(timers.get(a.id))
        timers.delete(a.id)
        const task = (args as { taskId: string }).taskId
        emit('agent-line', { id: a.id, line: JSON.stringify({ type: 'system', subtype: 'task_updated', task_id: task, patch: { status: 'killed' } }) })
        emit('agent-line', { id: a.id, line: JSON.stringify({ type: 'system', subtype: 'task_notification', task_id: task, status: 'stopped' }) })
        return
      }
      case 'mcp_request': {
        const { subtype, serverName, enabled } = (args as { request: { subtype: string; serverName?: string; enabled?: boolean } }).request
        const toggled = mcpServers.find((s) => s.name === serverName)
        if (subtype === 'mcp_toggle' && toggled) toggled.status = enabled ? 'connected' : 'disabled'
        const response =
          subtype === 'mcp_status'
            ? { mcpServers }
            : subtype === 'mcp_authenticate'
              ? { authUrl: 'https://claude.ai/oauth/authorize?demo' }
              : {}
        setTimeout(() => emit('agent-line', { id: a.id, line: JSON.stringify({ type: 'control_response', response: { subtype: 'success', request_id: a.requestId, response } }) }), 300)
        return
      }
      case 'set_workdir':
        return
      case 'git_branch':
        return `feat/${String(a.path).split('/').at(-1)}`
      case 'stop_turn':
        goalLines.set(a.id, (goalLines.get(a.id) ?? []).filter((l) => l.includes('"result"')))
        return
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
      case 'save_agent': {
        const data = JSON.parse(a.data)
        delete data.stored
        data.entries ??= JSON.parse(localStorage.getItem(`agent:${a.id}`) ?? '{}').entries
        localStorage.setItem(`agent:${a.id}`, JSON.stringify(data))
        return
      }
      case 'load_entries':
        return JSON.stringify(JSON.parse(localStorage.getItem(`agent:${a.id}`) ?? '{}').entries ?? [])
      case 'agent_texts':
        return Object.fromEntries(
          (args as { ids: string[] }).ids.map((id) => [
            id,
            (JSON.parse(localStorage.getItem(`agent:${id}`) ?? '{}').entries ?? [])
              .map((e: any) => (e.kind === 'user' ? e.text : e.kind === 'agent' ? e.blocks.map((b: any) => (b.type === 'text' ? b.text : '')).join('\n') : ''))
              .join('\n'),
          ]),
        )
      case 'delete_agent_file':
        localStorage.removeItem(`agent:${a.id}`)
        return
      case 'archive_agent_file':
        localStorage.setItem(`archived:${a.id}`, localStorage.getItem(`agent:${a.id}`) ?? '{}')
        localStorage.removeItem(`agent:${a.id}`)
        return
      case 'archived_texts':
        return Object.keys(localStorage)
          .filter((k) => k.startsWith('archived:'))
          .map((k) => JSON.parse(localStorage.getItem(k) ?? '{}'))
          .map((agent) => ({
            id: agent.id,
            name: agent.name,
            project: agent.project,
            since: agent.since,
            text: (agent.entries ?? [])
              .map((e: any) => (e.kind === 'user' ? e.text : e.kind === 'agent' ? e.blocks.map((b: any) => (b.type === 'text' ? b.text : '')).join('\n') : ''))
              .join('\n'),
          }))
      case 'list_archived':
        return Object.keys(localStorage)
          .filter((k) => k.startsWith('archived:'))
          .map((k) => {
            const { entries = [], ...head } = JSON.parse(localStorage.getItem(k) ?? '{}')
            const stored = { count: entries.length, created: entries.find((e: { at?: number }) => e.at)?.at ?? null, users: entries.filter((e: { kind: string }) => e.kind === 'user').length }
            return JSON.stringify({ ...head, stored })
          })
      case 'mcp_servers':
        return mockMcp
      case 'mcp_on_demand':
      case 'mcp_everywhere': {
        const [from, to] = cmd === 'mcp_on_demand' ? [mockMcp.everywhere, mockMcp.on_demand] : [mockMcp.on_demand, mockMcp.everywhere]
        const server = from.splice(from.findIndex((s) => s.name === a.name), 1)[0]
        if (server) to.push(server)
        return null
      }
      case 'mcp_describe': {
        const server = mockMcp.on_demand.find((s) => s.name === a.name)
        if (server) server.description = a.description
        return null
      }
      case 'set_agent_servers':
        return null
      case 'archived_entries':
        return JSON.stringify(JSON.parse(localStorage.getItem(`archived:${a.id}`) ?? '{}').entries ?? [])
      case 'restore_archived': {
        const data = localStorage.getItem(`archived:${a.id}`)
        localStorage.removeItem(`archived:${a.id}`)
        localStorage.setItem(`agent:${a.id}`, data ?? '{}')
        return data
      }
      case 'delete_archived':
        localStorage.removeItem(`archived:${a.id}`)
        return
      case 'edit_project': {
        const project = projects.find((p) => p.path === a.path)!
        Object.assign(project, (args as { changes: Partial<Project> }).changes)
        localStorage.setItem('projects', JSON.stringify(projects))
        return views()
      }
      case 'order_projects': {
        const order = (args as { paths: string[] }).paths
        projects.sort((x, y) => order.indexOf(x.path) - order.indexOf(y.path))
        localStorage.setItem('projects', JSON.stringify(projects))
        return views()
      }
      case 'agent_ids':
        return Object.keys(localStorage).filter((k) => k.startsWith('agent:')).map((k) => k.slice(6))
      case 'load_agent':
        return localStorage.getItem(`agent:${a.id}`)
      case 'load_agents':
        return Object.keys(localStorage)
          .filter((k) => k.startsWith('agent:'))
          .map((k) => {
            const { entries = [], ...header } = JSON.parse(localStorage.getItem(k)!)
            return JSON.stringify({ ...header, stored: { count: entries.length, created: entries.find((e: any) => 'at' in e)?.at ?? null } })
          })
      case 'account_name':
        return 'Camille'
      case 'system_load':
        return { cpu: 18 + Math.random() * 25, used: 13.6 * 2 ** 30, total: 31.2 * 2 ** 30 }
      case 'fork_agent':
      case 'restore_agent':
        projectOf.set(a.id, a.project)
        emit('agent-line', { id: a.id, line: JSON.stringify(catalog) })
        return
      case 'artifacts_dir':
        return '/home/demo/.local/share/squadron/artifacts'
      case 'list_artifacts':
        return (shelves.get(a.agent) ?? []).map(({ content, ...rest }) => rest)
      case 'read_artifact':
        return shelves.get(a.agent)?.find((x) => x.id === a.id)?.content ?? ''
      case 'write_artifact': {
        const list = (shelves.get(a.agent) ?? []).filter((x) => x.id !== a.id)
        const old = shelves.get(a.agent)?.find((x) => x.id === a.id)
        shelves.set(a.agent, [...list, { id: a.id, kind: a.kind as Shelved['kind'], entry: ENTRY[a.kind as keyof typeof ENTRY], title: titled(a.kind, a.content, a.id), modified: Date.now(), link: old?.link ?? null, links: old?.links, content: a.content }])
        shelve()
        return `/home/demo/.local/share/squadron/artifacts/${a.agent}/${a.id}/${ENTRY[a.kind as keyof typeof ENTRY]}`
      }
      case 'set_artifact_keep':
        for (const x of shelves.get(a.agent) ?? []) if (x.id === a.id) x.keep = (a as unknown as { keep: number | null }).keep
        shelve()
        return null
      case 'artifact_owners':
        return [...shelves.keys()]
      case 'set_artifact_link':
        for (const x of shelves.get(a.agent) ?? []) if (x.id === a.id) x.link = a.link
        shelve()
        return null
      case 'set_artifact_share':
        for (const x of shelves.get(a.agent) ?? [])
          if (x.id === a.id) {
            const links = { ...x.links }
            if (a.link) links[a.destination] = { link: a.link, at: Date.now() }
            else delete links[a.destination]
            x.links = links
          }
        shelve()
        return null
      case 'extension_data_dir':
        return `/home/demo/.config/dev.squadron.desktop/extensions-data/${a.id}`
      case 'extensions_wasm_tool':
        return new Promise((r) => setTimeout(() => r(a.name === 'deploy' ? 'https://script.google.com/macros/s/AKfycbx4QmDemo/exec' : ''), 1500))
      case 'delete_artifact':
        shelves.set(a.agent, (shelves.get(a.agent) ?? []).filter((x) => x.id !== a.id))
        shelve()
        return null
      case 'open_artifact_folder':
        return null
      case 'read_project_file':
        if (files.has(a.path)) return files.get(a.path)
        if (a.path.endsWith('.php')) return '<?php\n\nfinal class ChannelSegmentResolver\n{\n    public function resolve(CustomerInterface $customer): array\n    {\n        return [];\n    }\n}\n'
        return a.path.endsWith('resume.txt') ? '2\n' : null
      case 'open_in_editor':
        return
      case 'get_settings':
        return { claude: claudePath, defaults: { model: 'opus', effort: 'high', mode: 'default' }, tab_effect: tabEffect, projects_effect: projectsEffect, tab_order: tabOrder, tab_hours: tabHours, keys: keyMap, editors: editorApps, pilot: true, pilot_all: false, language, review_prompt: reviewPrompt, ...runsConfig }
      case 'set_settings':
        claudePath = (a as unknown as { config: { claude: string } }).config.claude
        tabEffect = (a as unknown as { config: { tab_effect: string } }).config.tab_effect
        projectsEffect = (a as unknown as { config: { projects_effect: string } }).config.projects_effect
        tabOrder = (a as unknown as { config: { tab_order: string } }).config.tab_order
        tabHours = (a as unknown as { config: { tab_hours: number } }).config.tab_hours
        keyMap = (a as unknown as { config: { keys: Record<string, string> } }).config.keys
        editorApps = (a as unknown as { config: { editors: unknown[] } }).config.editors
        language = (a as unknown as { config: { language: string } }).config.language
        reviewPrompt = (a as unknown as { config: { review_prompt: string } }).config.review_prompt
        runsConfig = (({ runs, runs_linger, runs_hidden, runs_notify }) => ({ runs, runs_linger, runs_hidden, runs_notify }))((a as unknown as { config: typeof runsConfig }).config)
        return
      case 'suggest_title':
        return new Promise((resolve) => setTimeout(() => resolve('Plugin upgrade to 2.3'), 800))
      case 'claude_version':
        return '2.1.304 (Claude Code)'
      case 'plugin:app|version':
        return '0.36.0'
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
      case 'plugin:webview|create_webview_window': {
        const made = (args as { options?: { label: string; url: string } }).options
        if (made && made.label !== 'board') return void window.open(made.url, made.label, 'popup,width=760,height=820')
        popup = window.open('/?board', 'board', 'popup,width=1240,height=820')
        return
      }
      case 'plugin:window|get_all_windows':
        return popup && !popup.closed ? [label, 'board'] : [label]
      case 'plugin:window|set_focus':
        if (label === 'main') popup?.focus()
        return
      case 'pipeline_runs': {
        if (a.dir.includes('nova')) throw 'The project is not on Bitbucket, GitHub, GitLab or Azure DevOps.'
        const at = (ms: number) => new Date(loaded + ms).toISOString()
        const elapsed = Date.now() - loaded
        const run = (id: number, name: string, title: string, status: string, started: number, finished: number | null, event = 'push') => ({
          host: 'github', id: String(id), number: id, name, title, event, branch: 'main', url: `https://github.com/orbit/orbit/actions/runs/${id}`, author: id === 6 ? 'Copilot' : id % 2 ? 'Sébastien Leprovost' : 'adhermy', author_avatar: id === 4 ? 'https://avatars.githubusercontent.com/u/9919?s=64' : '',
          status: finished !== null && elapsed > finished ? status : elapsed > started ? 'running' : 'queued', started: at(started), finished: finished !== null && elapsed > finished ? at(finished) : null,
        })
        return [
          run(3, 'Release', 'Orbit 2.4.0', 'success', 0, 25_000),
          run(4, 'Tests', 'Segment CSV export', 'failure', 5_000, 40_000, 'pull_request'),
          run(5, 'Deploy staging', 'Segment CSV export', 'success', 30_000, null),
          run(6, 'Copilot code review', 'Segment CSV export', 'success', 0, 10_000, 'dynamic'),
          run(2, 'Release', 'Orbit 2.3.0', 'success', -3 * 3_600_000, -3 * 3_600_000 + 300_000),
        ]
      }
      case 'pull_request':
        return a.dir.includes('orbit') ? orbitPull() : null
      case 'linked_pull': {
        const url = a.url as string
        const projectsOf = a.projects as unknown as string[]
        if (url.includes('2108')) {
          return { repo: 'orbit/orbit', pull: orbitPull(), additions: 214, deletions: 38, files: 9, project: projectsOf.find((p) => p.includes('orbit')) ?? null }
        }
        if (url.includes('/57'))
          return {
            repo: 'agence/nova',
            pull: { host: 'github', number: 57, title: 'chore: bump Sylius to 2.3.4', description: '', url, state: 'merged', draft: false, approvals: 2, changes_requested: false, comments: 1, checks: 'passing', branch: 'chore/sylius', base: 'develop', head: '', author: 'Sébastien Leprovost', author_avatar: '', updated: new Date(Date.now() - 3 * 86_400_000).toISOString(), reviewed: null, conflicts: false, reviewers: [] },
            additions: 12,
            deletions: 12,
            files: 2,
            project: null,
          }
        if (url.includes('/61'))
          return {
            repo: 'agence/nova',
            pull: { host: 'github', number: 61, title: 'fix(export): CSV separator follows the locale', description: '', url, state: 'open', draft: false, approvals: 0, changes_requested: true, comments: 7, checks: 'failing', branch: 'fix/csv-locale', base: 'develop', head: '', author: 'Adrien Dhermy', author_avatar: '', updated: new Date(Date.now() - 7_200_000).toISOString(), reviewed: null, conflicts: true, reviewers: [{ name: 'Quentin Briais', avatar: '', verdict: 'changes_requested' }] },
            additions: 40,
            deletions: 6,
            files: 3,
            project: projectsOf.find((p) => p.includes('nova')) ?? null,
          }
        throw 'Not found'
      }
      case 'save_capture': {
        const path = `/captures/${files.size}.png`
        files.set(path, `data:image/png;base64,${a.data}`)
        return path
      }
      case 'save_attachment':
        files.set(`/attachments/${a.name}`, `data:image/png;base64,${a.data}`)
        return `/attachments/${a.name}`
      case 'attachment_preview':
        if (files.has(a.path)) return files.get(a.path)
        throw 'Not an image.'
      case 'open_attachment':
        return
      case 'attachment_size':
        return a.path.length * 3_917
      case 'project_sessions':
        return [
          { id: '271982a4-ba81-4f9b-ac49-2c87f1ef6e88', title: '[ORB-809] Pilotage tickets', cwd: a.project, here: true, updated: Date.now() - 3_600_000, size: 34_000_000, app: true, archived: false, model: 'opus', effort: 'high', mode: 'default' },
          { id: '524864f5-9fdd-4c1b-949d-d095b2079be7', title: 'ORB-809 : T7C admin grafts', cwd: `${a.project}/.worktrees/T7C`, here: true, updated: Date.now() - 7_200_000, size: 12_900_000, app: true, archived: false, model: 'opus', effort: 'xhigh', mode: 'default' },
          { id: 'c5a6da9d-6748-4b1f-8b9d-f3703886fcc7', title: 'Fix the CSV export', cwd: a.project, here: true, updated: Date.now() - 86_400_000, size: 640_000, app: false, archived: false, model: null, effort: null, mode: null },
          { id: '91c0d768-d00c-4f8d-83e3-36db4bff2000', title: 'Old spike on prices', cwd: a.project, here: true, updated: Date.now() - 30 * 86_400_000, size: 80_000, app: true, archived: true, model: 'sonnet', effort: 'low', mode: 'plan' },
        ]
      case 'read_session':
        return [
          JSON.stringify({ type: 'user', timestamp: new Date().toISOString(), message: { content: 'Where are we on the tickets?' } }),
          JSON.stringify({ type: 'assistant', uuid: 'u1', message: { id: 'm1', model: 'claude-opus', content: [{ type: 'text', text: 'T70 is in review, T7C is next.' }] } }),
        ].join('\n')
      case 'review_checkout':
        return `/home/demo/.local/share/squadron/reviews/orbit-${a.number}`
      case 'pull_lists': {
        if (!a.dir.includes('orbit') && !(location.search.includes('groups') && a.dir.includes('nova'))) throw 'The project is not on Bitbucket, GitHub, GitLab or Azure DevOps.'
        const pull = (number: number, title: string, author: string, branch: string, extra: object) => ({
          host: 'bitbucket', number, title, description: `## Result\n\n${title}.\n\n- First change\n- Second change`, url: `https://bitbucket.org/orbit/orbit/pull-requests/${number}`, state: 'open', draft: false,
          approvals: 0, changes_requested: false, comments: 0, checks: null, branch, base: 'main', head: 'd'.repeat(40), author,
          updated: new Date(Date.now() - number * 3_600_000).toISOString(), author_avatar: '', reviewed: null, conflicts: false, mergeable: false, reviewed_head: null, reviewed_at: null, head_at: null,
          reviewers: [
            { name: 'Sébastien Leprovost', avatar: '', verdict: number % 2 ? 'approved' : 'waiting' },
            { name: 'Adrien Dhermy', avatar: 'https://avatars.githubusercontent.com/u/9919?s=64', verdict: number === 2101 ? 'changes_requested' : 'waiting' },
            { name: 'Loris Marino', avatar: '', verdict: 'waiting' },
          ],
          ...extra,
        })
        return {
          review: [
            pull(2093, 'Remove the old GfK import and its patches', 'Amandine T.', 'dev/T21', { comments: 2, reviewed: 'changes_requested', reviewed_head: 'e'.repeat(40) }),
            pull(2098, 'Lower the autovacuum thresholds of sale', 'Amandine T.', 'dev/T19', { checks: 'passing' }),
            pull(2071, 'Export segments as CSV', 'Lucas M.', 'dev/T12', { reviewed: 'approved', approvals: 1, reviewed_head: 'e'.repeat(40) }),
          ],
          mine: [
            pull(2108, 'feat(segments): channel-aware segment resolver', 'Quentin Briais', 'feat/orbit', { approvals: 1, comments: 3, checks: 'passing', mergeable: true }),
            pull(2101, 'Faster price index rebuild', 'Quentin Briais', 'dev/T17', { changes_requested: true, checks: 'failing', comments: 5 }),
          ],
          others: [
            pull(2110, 'Retry failed Stripe webhooks', 'Lucas M.', 'fix/stripe-retry', { checks: 'passing' }),
            pull(2104, 'Keep promotions after login', 'Amandine T.', 'fix/cart-promo', { changes_requested: true, conflicts: true, comments: 7 }),
            pull(2097, 'Deployment checklist for Black Friday', 'Lucas M.', 'docs/bf-checklist', { reviewers: [{ name: 'Loris Marino', avatar: '', verdict: 'approved' }], approvals: 1 }),
            pull(2095, 'Draft: rework the search facets', 'Amandine T.', 'wip/facets', { draft: true }),
          ],
        }
      }
      case 'forge_accounts':
        return { bitbucket: localStorage.getItem('bitbucket'), bitbucket_user: true, github: 'octo', gitlab: localStorage.getItem('gitlab'), azure: localStorage.getItem('azure') }
      case 'bitbucket_sign_in':
        if (a.token === 'bad') throw 'Bitbucket refused this email and token.'
        localStorage.setItem('bitbucket', a.email)
        return
      case 'gitlab_sign_in':
        if (a.token === 'bad') throw 'GitLab refused this token.'
        localStorage.setItem('gitlab', `octo@${a.site.replace(/^https?:\/\//, '') || 'gitlab.com'}`)
        return
      case 'azure_sign_in':
        if (a.token === 'bad') throw 'Azure DevOps refused this token.'
        localStorage.setItem('azure', 'Octo Cat')
        return
      case 'forge_sign_out':
        localStorage.removeItem(a.account)
        return
      case 'pull_changes':
        return new Promise((resolve) => setTimeout(resolve, 1500)).then(() => ({
          changes: {
            from: 'c'.repeat(40),
            now: 'b'.repeat(40),
            files: [
              { path: 'src/Segmentation/ChannelSegmentResolver.php', status: 'M', add: 6, del: 2 },
              ...['Export', 'Import', 'Pricing', 'Stock', 'Channel', 'Customer', 'Order'].map((name) => ({ path: `src/${name}/${name}Service.php`, status: 'M', add: 6, del: 2 })),
              { path: 'tests/ResolverTest.php', status: 'A', add: 4, del: 0 },
            ],
          },
          ahead: 1,
          added: a.since ? 2 : 0,
        }))
      case 'pull_remarks':
        return [
          { id: 1, author: 'Amandine T.', avatar: '', body: 'Can the channel stay required here? Every caller has one.', path: 'src/Segmentation/ChannelSegmentResolver.php', line: 13, removed: false, reply_to: null, at: '2026-10-23T09:10:00Z', kind: 'line' },
          { id: 2, author: 'Quentin Briais', avatar: '', body: 'The CLI import has none.', path: 'src/Segmentation/ChannelSegmentResolver.php', line: 13, removed: false, reply_to: 1, at: '2026-10-23T09:20:00Z' },
          { id: 4, author: 'Lucas M.', avatar: '', body: 'The scheduler has none either, keep it nullable.', path: 'src/Segmentation/ChannelSegmentResolver.php', line: 13, removed: false, reply_to: 1, at: '2026-10-23T09:42:00Z' },
          { id: 5, author: 'Lucas M.', avatar: '', body: 'Why not reuse the export channel option?', path: 'src/Export/ExportService.php', line: 16, removed: false, reply_to: null, at: '2026-10-23T10:05:00Z', kind: 'line' },
          { id: 3, author: 'Amandine T.', avatar: '', body: 'Looks good once the test covers a missing channel.', path: null, line: null, removed: false, reply_to: null, at: '2026-10-23T09:30:00Z' },
        ]
      case 'pull_me':
        return { name: 'Quentin Briais', avatar: '' }
      case 'pull_reply':
        return
      case 'update_pull':
      case 'merge_pull':
        return
      case 'post_remarks':
        return { done: (args as { drafts: unknown[] }).drafts.length, said: true, error: null }
      case 'git_changes':
        if (a.project.includes('nova')) throw 'not a git repository'
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
      case 'branch_changes':
        if (a.project.includes('nova')) throw 'not a git repository'
        return {
          base: 'develop',
          commits: 4,
          changes: {
            from: 'c'.repeat(40),
            now: 'b'.repeat(40),
            files: [
              { path: 'config/packages/segmentation.yaml', status: 'A', add: 9, del: 0 },
              { path: 'src/Segmentation/ChannelSegmentResolver.php', status: 'M', add: 18, del: 4 },
              { path: 'src/Segmentation/SegmentRepository.php', status: 'M', add: 22, del: 3 },
              { path: 'tests/ResolverTest.php', status: 'A', add: 4, del: 0 },
              { path: 'templates/admin/segment/index.html.twig', status: 'M', add: 7, del: 1 },
            ],
          },
        }
      case 'git_patch':
        return a.path.endsWith('Test.php')
          ? `diff --git a/${a.path} b/${a.path}\nnew file mode 100644\n--- /dev/null\n+++ b/${a.path}\n@@ -0,0 +1,4 @@\n+<?php\n+\n+final class ResolverTest extends TestCase\n+{\n`
          : `diff --git a/${a.path} b/${a.path}\nindex 3f2a91c..${a.now.slice(0, 7)} 100644\n--- a/${a.path}\n+++ b/${a.path}\n@@ -10,6 +10,11 @@ final class ChannelSegmentResolver\n {\n-    public function resolve(CustomerInterface $customer): array\n+    public function resolve(\n+        CustomerInterface $customer,\n+        ?ChannelInterface $channel = null,\n+    ): array\n     {\n-        return [];\n+        $channel ??= $this->channelContext->getChannel();\n+\n+        return $this->repository->findForCustomer($customer, $channel);\n     }\n }\n`
      case 'existing_files':
        return (args as { paths: string[] }).paths.filter((p) => ['config/services.yaml', 'src/Segmentation/SegmentRepository.php'].includes(p))
      case 'project_files':
        return ['composer.json', 'composer.lock', 'config/', 'config/services.yaml', 'resume.txt', 'src/', 'src/Segmentation/', 'src/Segmentation/ChannelSegmentResolver.php', 'src/Segmentation/SegmentRepository.php', 'tests/', 'tests/ResolverTest.php']
      case 'project_grep':
        return [
          { path: 'src/Segmentation/ChannelSegmentResolver.php', line: 5, text: '    public function resolve(CustomerInterface $customer): array' },
          { path: 'src/Segmentation/SegmentRepository.php', line: 12, text: '        return $this->resolve($customer, $channel);' },
          { path: 'tests/ResolverTest.php', line: 3, text: 'final class ResolverTest extends TestCase' },
        ].filter((h) => h.text.toLowerCase().includes(String(a.query).toLowerCase()))
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
        if (!a.path.endsWith('.php')) return null
        return [
          '<?php',
          '',
          'declare(strict_types=1);',
          '',
          'namespace App\\Segmentation;',
          '',
          'use Sylius\\Component\\Channel\\Model\\ChannelInterface;',
          'use Sylius\\Component\\Customer\\Model\\CustomerInterface;',
          'final class ChannelSegmentResolver',
          '{',
          '    public function resolve(',
          '        CustomerInterface $customer,',
          '        ?ChannelInterface $channel = null,',
          '    ): array',
          '    {',
          '        $channel ??= $this->channelContext->getChannel();',
          '',
          '        return $this->repository->findForCustomer($customer, $channel);',
          '    }',
          '}',
          '',
          'final class ChannelSegmentCache',
          '{',
          ...['customer', 'channel', 'segment', 'group', 'zone', 'locale', 'currency'].flatMap((name) => [
            `    public function ${name}(string $key): ?string`,
            '    {',
            `        return $this->${name}s[$key] ?? null;`,
            '    }',
            '',
          ]),
          '}',
          '',
        ].join('\n')
      case 'term_open': {
        const id = ++terminal
        setTimeout(() => emit('term-out', { id, data: prompt }), 50)
        return { id, shell: 'zsh' }
      }
      case 'shell_run': {
        const id = ++terminal
        const command = String(a.command)
        const long = command.includes('sleep')
        const lines = command.includes('fail')
          ? [' \x1b[32m✓\x1b[0m src/lib/board.test.ts (12)\r\n', ' \x1b[31m✗ src/lib/agents.test.ts > keeps the worktree\r\n   expected /tmp/a to be /tmp/b\x1b[0m\r\n', '\r\n Tests  1 failed | 16 passed\r\n']
          : [`ran: ${command}\r\n`, 'building 10%\r', 'building 100%\r\n', 'done\r\n']
        lines.forEach((data, i) => setTimeout(() => emit('shell-out', { id, data }), 300 * (i + 1)))
        shells.set(id, setTimeout(() => emit('shell-exit', { id, code: command.includes('fail') ? 1 : 0 }), long ? 60_000 : 300 * (lines.length + 1)))
        return id
      }
      case 'term_close':
        if (shells.has(Number(a.id))) {
          clearTimeout(shells.get(Number(a.id)))
          shells.delete(Number(a.id))
          setTimeout(() => emit('shell-exit', { id: Number(a.id), code: null }), 50)
        }
        return
      case 'term_write': {
        const id = Number(a.id)
        const data = a.data === '\r' ? `\r\n${line ? `ran: ${line}\r\n` : ''}${prompt}` : a.data === '\x7f' ? '\b \b' : a.data
        line = a.data === '\r' ? '' : a.data === '\x7f' ? line.slice(0, -1) : line + a.data
        emit('term-out', { id, data })
        return
      }
      case 'set_agent_options':
        return
      case 'squadron_prompt_size':
        return 3300
      case 'context_usage':
        setTimeout(() => {
          const response = { ...CONTEXT_DEMO, maxTokens: 1_000_000, autoCompactThreshold: 967_000, isAutoCompactEnabled: true }
          emit('agent-line', { id: a.id, line: JSON.stringify({ type: 'control_response', response: { subtype: 'success', response } }) })
        }, 50)
        return
      case 'remote_control':
        if ((args as { enabled: boolean }).enabled)
          for (const line of [
            { type: 'system', subtype: 'bridge_state', state: 'ready' },
            { type: 'control_response', response: { subtype: 'success', response: { session_url: 'https://claude.ai/code/session_demo' } } },
            { type: 'system', subtype: 'bridge_state', state: 'connected' },
          ])
            emit('agent-line', { id: a.id, line: JSON.stringify(line) })
        if ((args as { enabled: boolean }).enabled)
          setTimeout(() => {
            const message = { type: 'user', isReplay: true, parent_tool_use_id: null, timestamp: new Date().toISOString(), message: { role: 'user', content: 'Sent from my phone: what is left to do?' } }
            emit('agent-line', { id: a.id, line: JSON.stringify(message) })
            const peer = `Another Claude session sent a message:\n<cross-session-message from="uds:/run/x.sock" from-name="ORB-809 : T7C admin grafts" from-mode="prompting">\nPR T7C (6/6) open: #1554, \`ia/ORB-809-T7C\` → \`dev/ORB-809\`.\n\nFixed in the PR:\n- attribute grid in 500: \`ProductAttributeRepository\` now extends the Sylius one.\n- tax rate grid in 500: filter \`zone.channelIds.id\` → \`zone.channels.id\`.\n</cross-session-message>\n\nThis came from another Claude session.`
            emit('agent-line', { id: a.id, line: JSON.stringify({ ...message, origin: { kind: 'peer' }, message: { role: 'user', content: peer } }) })
            replay(a.id, 'ok')
          }, 2500)
        return
      case 'pilot_reply':
        replies.get(a.id)?.(a.result)
        replies.delete(a.id)
        return
      case 'get_layout':
        return JSON.parse(localStorage.getItem('layout') ?? '{}')
      case 'set_layout':
        localStorage.setItem('layout', JSON.stringify((args as { layout: unknown }).layout))
        return
      case 'extensions_list':
        return {
          disabled: JSON.parse(localStorage.getItem('extensions-disabled') ?? '[]'),
          approved: JSON.parse(localStorage.getItem('extensions-approved') ?? '{}'),
          sources: [],
          origins: { jira: 'Jgrasp/squadron-extensions', lando: 'Jgrasp/squadron-extensions', 'team-share': 'Jgrasp/squadron-extensions' },
          accounts: JSON.parse(localStorage.getItem('extensions-accounts') ?? DEMO_ACCOUNTS),
          extensions: [
            {
              dir: '/home/demo/.config/dev.squadron.desktop/extensions/jira',
              linked: false,
              error: null,
              manifest: {
                id: 'jira',
                version: '0.1.0',
                api: 1,
                name: { en: 'Jira' },
                description: { en: 'Your sprint and tickets next to your agents', fr: 'Votre sprint et vos tickets à côté de vos agents' },
                wasm: 'jira.wasm',
                permissions: { exec: ['claude'], network: ['api.media.atlassian.com'] },
                contributes: {
                  accounts: {
                    label: { en: 'Jira sites', fr: 'Sites Jira' },
                    fields: [
                      { id: 'site', label: { en: 'Site', fr: 'Site' }, kind: 'host', hint: { en: 'company.atlassian.net' } },
                      { id: 'email', label: { en: 'Email', fr: 'E-mail' } },
                      {
                        id: 'token',
                        label: { en: 'API token', fr: 'Jeton d’API' },
                        kind: 'secret',
                        hint: { en: 'Create API token (not “with scopes”)', fr: 'Create API token (pas « with scopes »)' },
                        url: JIRA_TOKENS,
                      },
                    ],
                  },
                },
              },
            },
            {
              dir: '/home/demo/.config/dev.squadron.desktop/extensions/lando',
              linked: false,
              error: null,
              manifest: {
                id: 'lando',
                version: '1.0.0',
                api: 1,
                name: { en: 'Lando' },
                description: { en: 'Logs and docs of the Lando stack', fr: 'Journaux et documentation de la pile Lando' },
                contributes: {
                  items: [
                    { slot: 'agent.tools', id: 'logs', label: { en: 'Logs', fr: 'Journaux' }, icon: 'terminal', action: { 'terminal.run': 'lando logs -f' } },
                    { slot: 'agent.menu', id: 'review', label: { en: 'Review the diff', fr: 'Relire le diff' }, hint: { en: 'Inserts a review prompt', fr: 'Insère un prompt de relecture' }, action: { 'prompt.insert': 'Review the diff and list the risks.' } },
                    { slot: 'menu.squadron', id: 'docs', label: { en: 'Lando docs', fr: 'Doc Lando' }, hint: { en: 'docs.lando.dev' }, action: { 'url.open': 'https://docs.lando.dev' } },
                    { slot: 'artifact.actions', id: 'pdf', label: { en: 'Export as PDF', fr: 'Exporter en PDF' }, hint: { en: 'Prints the page to a file next to the artifact', fr: 'Imprime la page dans un fichier à côté de l’artifact' }, action: { 'prompt.insert': 'Export the open artifact as a PDF.' } },
                  ],
                  tools: [{ name: 'logs', description: 'Last 200 lines of the Lando logs', command: 'lando logs --tail 200' }],
                },
              },
            },
            {
              dir: '/home/demo/.config/dev.squadron.desktop/extensions/team-share',
              linked: false,
              error: null,
              manifest: {
                id: 'team-share',
                version: '0.1.0',
                api: 1,
                name: { en: 'Team share (Google)' },
                wasm: 'team-share.wasm',
                permissions: { exec: ['python3'] },
                contributes: {
                  skills: ['skills/share'],
                  items: [
                    {
                      slot: 'artifact.actions',
                      id: 'team',
                      label: { en: 'Share with the team', fr: 'Partager avec l’équipe' },
                      hint: { en: 'Google web app for example.com accounts', fr: 'App web Google pour les comptes example.com' },
                      action: {
                        'artifact.share': {
                          confirm: { en: '“{artifact.title}” becomes a Google web app that only example.com accounts can open, deployed from your Google account.' },
                          lines: ['Google Apps Script · access: example.com'],
                          shared: { en: 'Shared with the team', fr: 'Partagé avec l’équipe' },
                          share: 'Call the Skill tool with skill team-share:share-artifact and args: artifact {artifact.id} at {artifact.dir}, destination {destination}. Do nothing else.',
                          update: { tool: 'deploy' },
                          unshare: { tool: 'undeploy' },
                          cleanup: { days: 90 },
                          delete: { en: 'Also delete the Apps Script project', fr: 'Supprimer aussi le projet Apps Script' },
                          deleteHint: { en: 'The statuses and notes the team entered are deleted with it.', fr: 'Les statuts et notes saisis par l’équipe sont supprimés avec lui.' },
                        },
                      },
                    },
                  ],
                },
              },
            },
            { dir: '/home/demo/code/sentry-extension', linked: true, error: null, manifest: { id: 'sentry', version: '0.1.0', api: 1, name: { en: 'Sentry' }, contributes: { items: [{ slot: 'agent.info', id: 'x', label: { en: 'X' }, action: { squadron: 'kanban' } }] } } },
          ],
        }
      case 'extensions_approve': {
        const { id, commands } = args as { id: string; commands: string[] }
        localStorage.setItem('extensions-approved', JSON.stringify({ ...JSON.parse(localStorage.getItem('extensions-approved') ?? '{}'), [id]: commands }))
        return
      }
      case 'extensions_account_save':
      case 'extensions_account_remove': {
        const { id, account, values } = args as { id: string; account: string | null; values?: Record<string, string> }
        const all = JSON.parse(localStorage.getItem('extensions-accounts') ?? DEMO_ACCOUNTS)
        const list: { id: string; values: Record<string, string>; host: string | null; secrets: string[] }[] = all[id] ?? []
        const kept = list.filter((a) => a.id !== account)
        const fresh = account ?? Math.random().toString(16).slice(2, 10)
        if (values) {
          const site = values.site?.trim().replace(/^https?:\/\//, '').split('/')[0].toLowerCase() ?? null
          const old = list.find((a) => a.id === account)
          kept.push({ id: fresh, values: { site, email: values.email.trim() }, host: site, secrets: values.token?.trim() || old ? ['token'] : [] })
        }
        localStorage.setItem('extensions-accounts', JSON.stringify({ ...all, [id]: kept }))
        return fresh
      }
      case 'extensions_secrets':
        return []
      case 'extensions_set_tools':
      case 'extensions_wasm_event':
        return
      case 'extensions_set_skills':
        return []
      case 'extensions_wasm_sync':
        return {}
      case 'extensions_enable': {
        const { id, on } = args as { id: string; on: boolean }
        const off = JSON.parse(localStorage.getItem('extensions-disabled') ?? '[]').filter((d: string) => d !== id)
        localStorage.setItem('extensions-disabled', JSON.stringify(on ? off : [...off, id]))
        return
      }
      case 'detect_editors':
        return [
          { name: 'Zed', command: '/home/demo/.local/zed.app/bin/zed {project} {file}' },
          { name: 'PhpStorm', command: '/opt/phpstorm/bin/phpstorm.sh {project}' },
        ]
      case 'open_with':
        return
      case 'term_resize':
      case 'answer_askpass':
        return
      case 'sudo_needs_password':
        return { needed: true, user: 'demo' }
      case 'sudo_run': {
        if (a.password !== 'demo') throw 'Wrong password.'
        const run = askId++
        const lines = ['Reading package lists...\n', 'Building dependency tree...\n', 'The following NEW packages will be installed:\n  libssl-dev\n', 'Setting up libssl-dev (3.5.5-1) ...\n']
        lines.forEach((data, i) => setTimeout(() => emit('sudo-out', { id: run, data }), 500 * (i + 1)))
        setTimeout(() => emit('sudo-exit', { id: run, code: 0 }), 500 * (lines.length + 1))
        return run
      }
      case 'sudo_stop':
        return
      case 'ssh_key_state':
        return 'encrypted'
      case 'ssh_add_key':
        if (a.passphrase !== 'demo') throw 'Wrong passphrase.'
        return
      case 'plugin:dialog|open':
        return '/home/demo/agence/hermes'
      case 'plugin:dialog|ask':
        return true
      case 'plugin:dialog|message':
        return (args as { buttons?: { OkCancelCustom?: string[] } }).buttons?.OkCancelCustom?.[0] ?? 'Ok'
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
