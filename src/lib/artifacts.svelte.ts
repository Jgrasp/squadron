import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { agents, create, find, persist, send } from './agents.svelte'
import { defaultsFor } from './projects.svelte'
import type { ArtifactInfo } from './artifacts'
import type { Agent } from './stream'

export const shelf = $state<{ root: string; of: Record<string, ArtifactInfo[]> }>({ root: '', of: {} })

invoke<string>('artifacts_dir')
  .then((root) => (shelf.root = root))
  .catch(() => {})

export async function refresh(agent: string) {
  shelf.of[agent] = await invoke<ArtifactInfo[]>('list_artifacts', { agent }).catch(() => [])
}

listen<string>('artifacts-changed', ({ payload }) => refresh(payload))

export const pointed = $state<Record<string, string[]>>({})

export type Work = { destination: string; verb: 'share' | 'update' | 'unshare'; by: string; since: number; helper?: string }

export const working = $state<Record<string, Work>>({})

export const workKey = (agent: string, artifact: string) => `${agent}/${artifact}`

export type Delegation = { agent: string; artifact: string; destination: string; name: string; text: string }

export async function delegate({ agent, artifact, destination, name, text }: Delegation) {
  const owner = find(agent)
  if (!owner) return
  const helper = agents.list.find((a) => a.sharing?.agent === agent && a.sharing.artifact === artifact && a.sharing.destination === destination)
  if (helper) return send(helper.id, text), helper.id
  const id = await create(owner.project, name, defaultsFor(owner.project), text, { from: { by: owner.name }, dir: owner.worktree?.path })
  const made = find(id)
  if (!made) return
  made.sharing = { agent, artifact, destination }
  persist(made)
  return id
}

listen<{ agent: string; kind: 'point' | 'share' | 'delegate'; text: string; delegation?: Delegation }>('artifact-do', ({ payload: { agent, kind, text, delegation } }) => {
  if (getCurrentWindow().label !== 'main') return
  if (kind === 'delegate' && delegation) delegate(delegation)
  else if (kind === 'share') send(agent, text)
  else pointed[agent] = [...(pointed[agent] ?? []), text]
})

export async function migrate(agent: Agent) {
  for (const a of agent.artifacts ?? []) {
    const id = a.id.replace(/[^\w.-]/g, '-') || 'artifact'
    await invoke('write_artifact', { agent: agent.id, id, kind: a.kind, content: a.content })
    if (a.link) await invoke('set_artifact_link', { agent: agent.id, id, link: a.link })
  }
  agent.artifacts = undefined
  await refresh(agent.id)
}
