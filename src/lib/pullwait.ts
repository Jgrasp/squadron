import type { Run } from './pipelines.svelte'
import type { Pull } from './pulls.svelte'
import type { PullWait } from './stream'

export const NO_CHECKS_AFTER = 90_000

export function pullOutcome(wait: PullWait, pull: Pick<Pull, 'state' | 'checks' | 'url' | 'branch'>, now: number, runs: Run[] = []) {
  const n = `PR #${wait.number}`
  if (pull.state === 'merged') return `${n} merged`
  if (pull.state !== 'open') return `${n} closed without merge`
  if (wait.until === 'merged') return null
  if (pull.checks === 'passing') return `${n}: all checks green`
  if (pull.checks === 'failing') {
    const failed = runs.find((r) => r.status === 'failure' && (r.pull === wait.number || (r.pull == null && r.branch === pull.branch)))
    return failed ? `${n}: check ${failed.name} failed: ${failed.url}` : `${n}: check failed: ${pull.url}`
  }
  if (pull.checks == null && now - wait.since >= NO_CHECKS_AFTER) return `${n}: no checks`
  return null
}
