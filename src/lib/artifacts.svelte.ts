import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
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

export async function migrate(agent: Agent) {
  for (const a of agent.artifacts ?? []) {
    const id = a.id.replace(/[^\w.-]/g, '-') || 'artifact'
    await invoke('write_artifact', { agent: agent.id, id, kind: a.kind, content: a.content })
    if (a.link) await invoke('set_artifact_link', { agent: agent.id, id, link: a.link })
  }
  agent.artifacts = undefined
  await refresh(agent.id)
}
