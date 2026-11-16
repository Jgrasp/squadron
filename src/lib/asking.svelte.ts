type Kind = 'info' | 'warning' | 'error'

type Check = { label: string; hint?: string; onchange: (checked: boolean) => void }

type Options = { title?: string; kind?: Kind; icon?: 'github'; okLabel?: string; cancelLabel?: string; lines?: string[]; check?: Check; danger?: boolean }

export type Question = Options & { text: string; choice: boolean; answer: (yes: boolean) => void }

export const questions = $state<Question[]>([])

function show(text: string, options: Options | string | undefined, choice: boolean) {
  const opts = typeof options === 'string' ? { title: options } : (options ?? {})
  return new Promise<boolean>((answer) => questions.push({ ...opts, text, choice, answer }))
}

export const ask = (text: string, options?: Options | string) => show(text, options, true)

export const message = async (text: string, options?: Options | string) => {
  await show(text, options, false)
}

type Place = { name: string; color: string }

export type StartRequest = {
  caller: string
  from: Place
  to: Place
  name: string
  mode: 'acceptEdits' | 'auto'
  persona?: string
  model: string
  effort: string
  worktree?: string
  card?: string
  prompt: string
  goal: string
  answer: (yes: boolean) => void
}

export const starts = $state<StartRequest[]>([])

export const askStart = (request: Omit<StartRequest, 'answer'>) => new Promise<boolean>((answer) => starts.push({ ...request, answer }))
