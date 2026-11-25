export const OPTIONAL_TOOLS = ['Artifact', 'Workflow', 'ScheduleWakeup', 'ReportFindings', 'ShareOnboardingGuide', 'ListAgents']

const WORKFLOW = [
  /\bultracode\b/i,
  /\b(use|run|start|launch)\s+(a|an)\s+(multi-agent\s+)?workflow\b/i,
  /\b(lance[rzs]?|utilise[rzs]?|fais|faire|démarre[rzs]?|passe[rzs]?\s+par)\s+(un|une)\s+workflow\b/i,
  /\bfan\s+out\s+(the\s+)?agents\b/i,
  /\borchestrat\w*\b.{0,40}\bsub-?agents?\b/i,
]

export function toolsFor(text: string) {
  const said = text.trimStart()
  const wanted: string[] = []
  if (/^\/loop(\s|$)/.test(said)) wanted.push('ScheduleWakeup')
  if (/^\/code-review(\s|$)/.test(said)) wanted.push('ReportFindings')
  if (WORKFLOW.some((re) => re.test(text))) wanted.push('Workflow')
  return wanted
}

export const missing = (enabled: string[] | undefined, wanted: string[]) => wanted.filter((t) => !enabled?.includes(t))
