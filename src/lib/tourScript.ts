import { t } from './i18n/index.svelte'
import type { Options, Status } from './stream'

export type Director = {
  agent(id: string, name: string, status: Status, options?: Partial<Options>): void
  user(id: string, text: string): void
  say(id: string, key: string, text: string, typing?: number): void
  call(id: string, call: string, name: string, input: Record<string, unknown>): void
  result(id: string, call: string, text: string): void
  ask(id: string, ask: string, tool: string, input: Record<string, unknown>, call: string): void
  status(id: string, status: Status, action?: string): void
  allow(id: string, ask: string): void
  select(id: string): void
  type(text: string, typing?: number): void
  mode(id: string, mode: Options['mode']): void
  tokens(id: string, tokens: number): void
}

export type Event = [at: number, run: (d: Director) => void, gated?: true]

export const A1 = 'demo:a1'
export const A2 = 'demo:a2'
export const A3 = 'demo:a3'
export const ASK = 'demo:ask-stripe'

const file = (path: string) => `demo:acme-shop/${path}`

export const SEGMENTS: Record<string, [from: number, to: number]> = {
  tabs: [0, 2700],
  list: [2700, 5300],
  stream: [5300, 12200],
  copper: [12200, 13900],
  perm: [14000, 19600],
  composer: [19600, 26200],
  options: [26200, 29100],
  context: [29100, 32000],
}

export const GATE = 14000

export const END = 40000

export const EVENTS: Event[] = [
  [600, (d) => {
    d.agent(A2, t('tour.demoA2'), 'done')
    d.user(A2, t('tour.demoA2Ask'))
    d.say(A2, 'a2-s1', t('tour.demoA2Done'))
    d.status(A2, 'done')
  }],
  [1000, (d) => {
    d.agent(A1, t('tour.demoA1'), 'working')
    d.user(A1, t('tour.demoA1Ask'))
    d.select(A1)
    d.call(A1, 'demo:c1', 'Read', { file_path: file('src/orders/page.tsx') })
  }],
  [1400, (d) => {
    d.agent(A3, t('tour.demoA3'), 'working')
    d.user(A3, t('tour.demoA3Ask'))
    d.say(A3, 'a3-s1', t('tour.demoA3Plan'))
    d.call(A3, 'demo:c20', 'Read', { file_path: file('package.json') })
    d.tokens(A1, 41_000)
  }],
  [3300, (d) => {
    d.result(A1, 'demo:c1', '412 lines')
    d.call(A1, 'demo:c2', 'Read', { file_path: file('src/lib/table.ts') })
  }],
  [4300, (d) => {
    d.result(A3, 'demo:c20', '64 lines')
    d.call(A3, 'demo:c21', 'Read', { file_path: file('src/api/webhook.ts') })
  }],
  [5500, (d) => {
    d.result(A1, 'demo:c2', '88 lines')
    d.say(A1, 'a1-s1', t('tour.demoA1Plan'), 1300)
  }],
  [7000, (d) => d.call(A1, 'demo:c3', 'Grep', { pattern: 'toCsv', path: file('src') })],
  [7600, (d) => {
    d.result(A1, 'demo:c3', 'No matches')
    d.call(A1, 'demo:c4', 'Write', { file_path: file('src/lib/csv.ts'), content: 'export function toCsv(rows: string[][]) {\n  return rows.map((r) => r.join(",")).join("\\n")\n}\n' })
  }],
  [8800, (d) => {
    d.result(A1, 'demo:c4', 'File created')
    d.call(A1, 'demo:c5', 'Edit', { file_path: file('src/orders/page.tsx'), old_string: '<Filters />', new_string: '<Filters />\n<ExportCsv rows={rows} />' })
  }],
  [9800, (d) => {
    d.result(A1, 'demo:c5', 'File updated')
    d.say(A1, 'a1-s2', t('tour.demoA1Done'), 1600)
    d.tokens(A1, 62_000)
  }],
  [11600, (d) => d.call(A1, 'demo:c6', 'Bash', { command: 'pnpm test orders', description: 'Run the orders tests' })],
  [12400, (d) => {
    d.result(A3, 'demo:c21', '120 lines')
    d.call(A3, 'demo:c22', 'Bash', { command: 'pnpm add stripe@17', description: 'Install Stripe SDK 17' })
    d.ask(A3, ASK, 'Bash', { command: 'pnpm add stripe@17', description: 'Install Stripe SDK 17' }, 'demo:c22')
  }],
  [13200, (d) => d.result(A1, 'demo:c6', '12 passed')],
  [14000, (d) => d.select(A3)],
  [14100, (d) => d.allow(A3, ASK), true],
  [15200, (d) => {
    d.result(A3, 'demo:c22', 'added 1 package in 2.1s')
    d.say(A3, 'a3-s2', t('tour.demoA3Next'), 1500)
  }, true],
  [17200, (d) => d.call(A3, 'demo:c23', 'Edit', { file_path: file('src/api/webhook.ts'), old_string: 'constructEvent(', new_string: 'constructEventAsync(' }), true],
  [19800, (d) => d.type('!pnpm test', 900)],
  [22400, (d) => d.type(t('tour.demoTypeMention'), 1300)],
  [26000, (d) => d.type('')],
  [27300, (d) => d.mode(A3, 'plan')],
  [29400, (d) => d.tokens(A3, 96_000)],
  [30200, (d) => d.tokens(A3, 118_000)],
]
