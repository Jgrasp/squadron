import type { Run } from './pipelines.svelte'
import type { Pull } from './pulls.svelte'

const EVENTS = new Set(['pull_request', 'pull_request_target', 'merge_request_event', 'pullRequest', 'pullrequest'])

export const pullOf = (run: Run, pulls: Pull[]) => pulls.find((p) => (run.pull ? p.number === run.pull : !!run.branch && p.branch === run.branch)) ?? null

export const isCheck = (run: Run, pulls: Pull[]) => !!run.pull || EVENTS.has(run.event) || !!pullOf(run, pulls)
