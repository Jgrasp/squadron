const LIMIT = 30_000

export function plain(raw: string) {
  return raw
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, '')
    .replace(/\x1b\[[0-9;?<=>]*[ -/]*[@-~]/g, '')
    .replace(/\x1b[()][0-9A-Za-z]|\x1b[=>78DEHM]/g, '')
    .split('\n')
    .map((line) => line.replace(/\r+$/, '').split('\r').at(-1)!)
    .join('\n')
    .replace(/[\x00-\x08\x0b-\x1f\x7f]/g, '')
    .trimEnd()
}

export function clip(text: string) {
  if (text.length <= LIMIT) return text
  return `${text.slice(0, LIMIT / 3)}\n… ${text.length - LIMIT} characters cut …\n${text.slice(-(LIMIT * 2) / 3)}`
}

export type Ran = { command: string; output: string; code?: number | null; state: string }

export function said(runs: Ran[]) {
  return runs
    .map((r) => {
      const status = r.state === 'stopped' ? 'Stopped by the user' : r.code ? `Exit code ${r.code}` : ''
      return `<bash-input>${r.command}</bash-input>\n<bash-stdout>${clip(r.output)}</bash-stdout><bash-stderr>${status}</bash-stderr>`
    })
    .join('\n\n')
}
