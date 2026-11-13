<script lang="ts">
  import type { Snippet } from 'svelte'
  import { agents, find, reply, send, show } from './agents.svelte'
  import type { Project } from './projects.svelte'
  import { fixWithAgent, lists, lookUpLists, moved, pullOf, reviewerOf, sinceOf, type Pull } from './pulls.svelte'
  import DiffPanel from './DiffPanel.svelte'
  import { prompt, type Comment } from './stream'

  let { project, pull, onagent, banner, onclose }: { project: Project; pull: Pull; onagent: () => Promise<unknown>; banner?: Snippet; onclose: () => void } = $props()

  let file = $state('')
  let notes = $state<Comment[]>([])
  let target = $state<Comment | null>(null)

  const agentOn = $derived(agents.list.find((a) => a.project === project.path && pullOf(a)?.number === pull.number))

  async function handOver(summary: string, list: Comment[]) {
    const agent = agentOn
    const on = pull
    onclose()
    if (!agent) return show(find(await fixWithAgent(project.path, on, summary, list))!)
    const message = summary || `Review of pull request #${on.number}`
    if (!reply(agent, prompt(message, list))) send(agent.id, message, list)
    show(agent)
  }
</script>

<DiffPanel
  agent={null}
  dir={project.path}
  changes={null}
  {pull}
  since={pull.reviewed && moved(project.path, pull) ? sinceOf(project.path, pull) : null}
  mode="pull"
  own={lists[project.path]?.mine.some((p) => p.number === pull.number)}
  bind:file
  bind:comments={notes}
  bind:target
  onhand={() => {}}
  onreview={handOver}
  handTo={agentOn ?? 'new'}
  onposted={() => (onclose(), lookUpLists(project.path, true))}
  reviewer={reviewerOf(project.path, pull) ?? null}
  {onagent}
  {banner}
  {onclose}
/>
