import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { find } from './agents.svelte'
import { t } from './i18n/index.svelte'
import { calls } from './stream'

export type Question = { id: string; agent: string; prompt: string; command: string }

const key = (q: { agent: string; prompt: string; command: string }) => `${q.agent}\n${q.command || q.prompt}`

export const asked = $state({ list: [] as Question[] })

const REMEMBER = 5 * 60_000
const remembered = new Map<string, { secret: string; until: number }>()

listen<Question>('askpass-request', ({ payload }) => {
  const kept = remembered.get(key(payload))
  if (kept && kept.until > Date.now()) return void invoke('answer_askpass', { id: payload.id, secret: kept.secret })
  const agent = find(payload.agent)
  if (!payload.command) payload.command = String((agent && calls(agent).findLast((c) => c.name === 'Bash' && !c.result))?.input.command ?? '')
  asked.list.push(payload)
  if (agent) agent.attention = t('shell.passwordWaiting')
})

export function reply(question: Question, secret: string | null, remember = false) {
  asked.list = asked.list.filter((q) => q.id !== question.id)
  if (secret !== null && remember) remembered.set(key(question), { secret, until: Date.now() + REMEMBER })
  const agent = find(question.agent)
  if (agent && !asked.list.some((q) => q.agent === question.agent)) agent.attention = ''
  return invoke('answer_askpass', { id: question.id, secret }).catch(() => {})
}

export function forget(id: string) {
  asked.list = asked.list.filter((q) => q.agent !== id)
  const agent = find(id)
  if (agent?.attention === t('shell.passwordWaiting')) agent.attention = ''
}
