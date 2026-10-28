import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import hello from '../../src-tauri/tests/fixtures/hello.jsonl?raw'
import nologin from '../../src-tauri/tests/fixtures/nologin.jsonl?raw'
import sigint from '../../src-tauri/tests/fixtures/sigint.jsonl?raw'
import taskList from '../../src-tauri/tests/fixtures/tasks.jsonl?raw'
import tools from '../../src-tauri/tests/fixtures/tools.jsonl?raw'
import subagents from '../../src-tauri/tests/fixtures/subagents.jsonl?raw'
import background from '../../src-tauri/tests/fixtures/background.jsonl?raw'
import thinking from '../../src-tauri/tests/fixtures/thinking.jsonl?raw'

type Project = { name: string; path: string; rules: string[]; cards?: unknown[]; color?: string; pinned?: boolean; archived?: boolean; runs?: boolean; runs_notify?: boolean; runs_hidden?: string[]; repository?: string; review_prompt?: string; tickets?: unknown[] }

const projects: Project[] = JSON.parse(localStorage.getItem('projects') ?? 'null') ?? [
  { name: 'Orbit', path: '/home/demo/agence/orbit', rules: [], tickets: [{ connection: 'akki', key: 'RING', name: 'RingApp' }] },
  { name: 'Nova', path: '/home/demo/agence/nova', rules: [] },
  { name: 'Lumen', path: '/home/demo/agence/lumen-absent', rules: [] },
]

const label = location.search.includes('board') ? 'board' : 'main'
const channel = new BroadcastChannel('squadron-events')
channel.onmessage = ({ data }) => data.target === label && emit(data.event, data.payload)
let popup: Window | null = null

if (location.search.includes('many'))
  for (const name of ['Hermes', 'Atlas', 'Optika', 'Vinéa', 'Kalia', 'Sorbet', 'Néréide', 'Pixel Rando'])
    projects.push({ name, path: `/home/demo/agence/${name.toLowerCase()}`, rules: [] })

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

const views = () => projects.map((p, i) => ({ ...folder(p.name, p.path, p.rules, p.cards, (p.color ??= COLORS[i % COLORS.length])), pinned: !!p.pinned, archived: !!p.archived, runs: p.runs ?? true, runs_notify: p.runs_notify ?? true, runs_hidden: p.runs_hidden ?? [], repository: p.repository ?? '', review_prompt: p.review_prompt ?? '', tickets: p.tickets ?? [] }))

const loaded = Date.now()
const timers = new Map<string, number>()
let claudePath = 'claude'
let reviewPrompt = ''
let tabEffect = 'pulse'
let tabOrder = 'manual'
let keyMap: Record<string, string> = {}
let editorApps: unknown[] = []
let language = 'system'
let ticketFieldsOff: string[] = []
let ticketConnections = JSON.parse(localStorage.getItem('ticket-connections') ?? 'null') ?? [
  { id: 'akki', name: 'Akki', site: 'akki.atlassian.net', email: 'quentin@akki.fr' },
  { id: 'ringover', name: 'Client Ringover', site: 'ringover.atlassian.net', email: 'quentin@akki.fr' },
]
const jiraProjects: Record<string, { key: string; name: string }[]> = {
  akki: [{ key: 'RING', name: 'RingApp' }, { key: 'TMA', name: 'Maintenance' }, { key: 'SQD', name: 'Squadron' }],
  ringover: [{ key: 'RBK', name: 'RingBack' }, { key: 'ORB', name: 'Orbit' }],
}
const person = (name: string) => ({ name, avatar: '' })
const sprintTickets = [
  ['RING-482', 'Export CSV des appels manqués depuis le tableau de bord', 'À faire', 'Quentin Briais'],
  ['RING-479', 'Le webhook Twilio rejoue deux fois l’événement hangup', 'En cours', 'Marie Laurent'],
  ['RING-471', 'Pagination de l’API /v2/calls', 'Revue', 'Quentin Briais'],
  ['RING-468', 'Traduire les libellés du module SVI', 'À faire', ''],
  ['RING-460', 'Mise à jour Symfony 7.3 → 7.4', 'À faire', 'Julien Durand'],
] as const
const fullTicket = (key: string) => {
  const found = sprintTickets.find(([k]) => k === key) ?? sprintTickets[0]
  return {
    connection: 'akki',
    key,
    url: `https://akki.atlassian.net/browse/${key}`,
    title: found[1],
    kind: 'Story',
    status: found[2],
    priority: 'High',
    assignee: found[3] ? person(found[3]) : null,
    reporter: person('Julien Durand'),
    labels: ['dashboard', 'export'],
    components: ['Back'],
    versions: ['2.14'],
    epic: { relation: 'Epic', key: 'RING-400', title: 'Tableau de bord superviseur', status: 'En cours' },
    sprint: 'RING Sprint 42',
    updated: new Date(Date.now() - 2 * 3600_000).toISOString(),
    description: 'En tant que superviseur, je veux exporter la liste des appels manqués de la période affichée pour la transmettre au service client.\n\n## Critères d’acceptation\n\n- Bouton « Exporter » au-dessus du tableau\n- Colonnes : date, numéro, ligne, durée d’attente\n- Respecte les filtres actifs et le fuseau du compte\n- Au-delà de 50 000 lignes, export asynchrone par e-mail',
    subtasks: [{ relation: '', key: 'RING-483', title: 'Bouton front', status: 'À faire' }],
    links: [{ relation: 'relates to', key: 'RING-301', title: 'Fuseau horaire du compte', status: 'Terminé' }],
    attachments: ['maquette-export.png', 'exemple.csv'],
    comments: [
      { id: '1', parent: null, author: person('Julien Durand'), created: '2025-10-20T09:00:00Z', body: 'Le client est en Guyane, donc fuseau UTC−3 à tester.' },
      { id: '2', parent: null, author: person('Marie Laurent'), created: '2025-10-24T10:00:00Z', body: 'Attention, le fuseau doit être celui du compte, pas du navigateur. Cf. RING-301.' },
      { id: '3', parent: '2', author: person('Quentin Briais'), created: '2025-10-24T11:00:00Z', body: 'Le fuseau du compte est dans `Account::timezone`, je m’appuie dessus.' },
      { id: '4', parent: '3', author: person('Marie Laurent'), created: '2025-10-25T08:00:00Z', body: 'Parfait 👍' },
      { id: '5', parent: null, author: person('Julien Durand'), created: '2025-10-26T08:00:00Z', body: 'Le client veut aussi le numéro de ligne SVI dans l’export.' },
    ],
    custom: [{ name: 'Story Points', value: '3' }],
  }
}
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
  if (text.includes('sudo')) setTimeout(() => emit('askpass-request', { id: String(askId++), agent: id, prompt: '[sudo] password for demo: ' }), 400)
  if (text.startsWith('/goal ')) return goal(id, text.slice(6))
  const key = Object.keys(asks).find((k) => text.includes(k))
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
  const links = 'J\'ai poussé la correction et ouvert la PR :\\n\\nhttps://bitbucket.org/orbit/orbit/pull-requests/2108\\n\\nLa PR https://github.com/agence/nova/pull/57 dont elle dépend est fusionnée, et [celle-ci](https://github.com/agence/nova/pull/61) bloque encore.\\n\\nhttps://github.com/autre-org/prive/pull/12\\n\\nJe surveille la CI.'
  const stream = text.includes('bashblock') ? hello.replaceAll('"Bonjour"', JSON.stringify('Il reste à recharger nginx :\n\n```bash\nsudo nginx -t && sudo systemctl reload nginx\n```')) : text.includes('links') ? hello.replaceAll('"Bonjour"', `"${links}"`) : text.includes('shots') ? shots() : text.includes('claudedoc') ? claudeDoc() : text.includes('capture') ? hello.replaceAll('"Bonjour"', `"${shot}"`) : text.includes('think') ? thinking : text.includes('swarm') ? swarm() : text.includes('subagents') ? subagents : text.includes('background') ? background : text.includes('tasks') ? taskList : text.includes('tool') ? tools : text.includes('logout') ? nologin : text.includes('long') ? sigint : hello
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
    emit('agent-line', { id, line: line.replaceAll('/project', projectOf.get(id) ?? '/project') })
    const sub = JSON.parse(line)
    const call = sub.type === 'assistant' && sub.parent_tool_use_id && sub.message.content.find((c: { name?: string }) => c.name === 'Bash')
    if (call) {
      const ask = String(askId++)
      waitingReplay.set(ask, next)
      emit('permission-request', { id: ask, agent: id, tool_name: 'Bash', input: call.input, tool_use_id: call.id })
      return
    }
    timers.set(id, window.setTimeout(next, text.includes('long') || text.includes('tasks') || text.includes('subagents') || text.includes('background') ? 150 : text.includes('swarm') ? 8 : 60))
  }
  next()
}

const projectOf = new Map<string, string>()
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
      case 'create_agent':
        projectOf.set(a.id, a.project)
        emit('agent-line', { id: a.id, line: JSON.stringify(catalog) })
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
      case 'save_agent':
        localStorage.setItem(`agent:${a.id}`, a.data)
        return
      case 'delete_agent_file':
        localStorage.removeItem(`agent:${a.id}`)
        return
      case 'archive_agent_file':
        localStorage.setItem(`archived:${a.id}`, localStorage.getItem(`agent:${a.id}`) ?? '{}')
        localStorage.removeItem(`agent:${a.id}`)
        return
      case 'list_archived':
        return Object.keys(localStorage).filter((k) => k.startsWith('archived:')).map((k) => localStorage.getItem(k))
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
        return Object.keys(localStorage).filter((k) => k.startsWith('agent:')).map((k) => localStorage.getItem(k))
      case 'system_load':
        return { cpu: 18 + Math.random() * 25, used: 13.6 * 2 ** 30, total: 31.2 * 2 ** 30 }
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
        return { claude: claudePath, defaults: { model: 'opus', effort: 'high', mode: 'default' }, tab_effect: tabEffect, tab_order: tabOrder, keys: keyMap, editors: editorApps, pilot: true, pilot_all: false, language, review_prompt: reviewPrompt, ticket_fields_off: ticketFieldsOff, ...runsConfig }
      case 'set_settings':
        claudePath = (a as unknown as { config: { claude: string } }).config.claude
        tabEffect = (a as unknown as { config: { tab_effect: string } }).config.tab_effect
        tabOrder = (a as unknown as { config: { tab_order: string } }).config.tab_order
        keyMap = (a as unknown as { config: { keys: Record<string, string> } }).config.keys
        editorApps = (a as unknown as { config: { editors: unknown[] } }).config.editors
        language = (a as unknown as { config: { language: string } }).config.language
        reviewPrompt = (a as unknown as { config: { review_prompt: string } }).config.review_prompt
        ticketFieldsOff = (a as unknown as { config: { ticket_fields_off: string[] } }).config.ticket_fields_off
        runsConfig = (({ runs, runs_linger, runs_hidden, runs_notify }) => ({ runs, runs_linger, runs_hidden, runs_notify }))((a as unknown as { config: typeof runsConfig }).config)
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
      case 'pipeline_runs': {
        if (a.dir.includes('nova')) throw 'The project is not on Bitbucket or GitHub.'
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
        if (!a.dir.includes('orbit')) throw 'The project is not on Bitbucket or GitHub.'
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
            pull(2093, 'Remove the old GfK import and its patches', 'Amandine T.', 'dev/T21', { comments: 2 }),
            pull(2098, 'Lower the autovacuum thresholds of sale', 'Amandine T.', 'dev/T19', { checks: 'passing' }),
            pull(2071, 'Export segments as CSV', 'Lucas M.', 'dev/T12', { reviewed: 'approved', approvals: 1, reviewed_head: 'e'.repeat(40) }),
          ],
          mine: [
            pull(2108, 'feat(segments): channel-aware segment resolver', 'Quentin Briais', 'feat/orbit', { approvals: 1, comments: 3, checks: 'passing', mergeable: true }),
            pull(2101, 'Faster price index rebuild', 'Quentin Briais', 'dev/T17', { changes_requested: true, checks: 'failing', comments: 5 }),
          ],
        }
      }
      case 'ticket_connections':
        return ticketConnections
      case 'ticket_connect': {
        if ((a as { token: string }).token === 'bad') throw 'Jira refused this email and token.'
        const id = a.id ?? a.site.split('.')[0]
        const found = { id, name: a.name || id, site: a.site, email: a.email }
        ticketConnections = [...ticketConnections.filter((c: { id: string }) => c.id !== id), found]
        localStorage.setItem('ticket-connections', JSON.stringify(ticketConnections))
        return ticketConnections
      }
      case 'ticket_disconnect':
        ticketConnections = ticketConnections.filter((c: { id: string }) => c.id !== a.id)
        localStorage.setItem('ticket-connections', JSON.stringify(ticketConnections))
        return ticketConnections
      case 'ticket_projects':
        if (a.connection === 'ringover') return new Promise((_, reject) => setTimeout(() => reject('Jira refused this email and token.'), 400))
        return new Promise((resolve) => setTimeout(() => resolve(jiraProjects[a.connection] ?? []), 300))
      case 'ticket_mentions':
        return { RING: [38, 4], TMA: [23, 4], ORB: [3, 0], UTF: [2, 0] }
      case 'ticket_sprint':
        return new Promise((resolve) =>
          setTimeout(
            () => resolve({ tickets: sprintTickets.map(([key, title, status, who]) => ({ connection: 'akki', key, title, status, assignee: who ? person(who) : null })), sprints: ['RING Sprint 42'], errors: [] }),
            600,
          ),
        )
      case 'ticket_get':
        return new Promise((resolve) => setTimeout(() => resolve(fullTicket(a.key)), 300))
      case 'ticket_context':
        return `<ticket source="jira" key="${a.key}">\n# ${a.key}\n</ticket>`
      case 'ticket_brief':
        return new Promise((resolve) =>
          setTimeout(
            () =>
              resolve({
                prompt: `Ajouter un export CSV des appels manqués sur le tableau de bord superviseur (${a.key}).\n- Colonnes : date, numéro, ligne, ligne SVI, durée d’attente.\n- Respecter les filtres actifs et le fuseau du compte (Account::timezone, cf. RING-301).`,
                goal: `Les critères d’acceptation de ${a.key} sont couverts par des tests fonctionnels.`,
              }),
            1800,
          ),
        )
      case 'forge_accounts':
        return { bitbucket: localStorage.getItem('bitbucket'), bitbucket_user: true, github: 'octo' }
      case 'bitbucket_sign_in':
        if (a.token === 'bad') throw 'Bitbucket refused this email and token.'
        localStorage.setItem('bitbucket', a.email)
        return
      case 'bitbucket_sign_out':
        localStorage.removeItem('bitbucket')
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
          { id: 1, author: 'Amandine T.', body: 'Can the channel stay required here? Every caller has one.', path: 'src/Segmentation/ChannelSegmentResolver.php', line: 13, removed: false, reply_to: null, at: '2026-10-23T09:10:00Z', kind: 'line' },
          { id: 2, author: 'Quentin Briais', body: 'The CLI import has none.', path: 'src/Segmentation/ChannelSegmentResolver.php', line: 13, removed: false, reply_to: 1, at: '2026-10-23T09:20:00Z' },
          { id: 3, author: 'Amandine T.', body: 'Looks good once the test covers a missing channel.', path: null, line: null, removed: false, reply_to: null, at: '2026-10-23T09:30:00Z' },
        ]
      case 'pull_reply':
        return
      case 'merge_pull':
        return
      case 'post_remarks':
        return { done: (args as { drafts: unknown[] }).drafts.length, error: null }
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
      case 'project_files':
        return ['composer.json', 'composer.lock', 'config/', 'config/services.yaml', 'resume.txt', 'src/', 'src/Segmentation/', 'src/Segmentation/ChannelSegmentResolver.php', 'src/Segmentation/SegmentRepository.php', 'tests/', 'tests/ResolverTest.php']
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
      case 'context_usage':
        setTimeout(() => {
          const response = { totalTokens: Number(localStorage.getItem('context-demo') ?? 65_000), maxTokens: 1_000_000, autoCompactThreshold: 967_000, isAutoCompactEnabled: true }
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
