type Kind = 'info' | 'warning' | 'error'

type Options = { title?: string; kind?: Kind; okLabel?: string; cancelLabel?: string }

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
