<script lang="ts">
  import { emitTo } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import ArtifactsPanel from './lib/ArtifactsPanel.svelte'
  import { refresh } from './lib/artifacts.svelte'
  import type { Agent } from './lib/stream'

  const query = new URLSearchParams(location.search)
  const agent = $state({ id: query.get('agent') ?? '', name: query.get('name') ?? '', entries: [], subagents: [] } as unknown as Agent)
  const relay = (kind: 'point' | 'share') => (text: string) => emitTo('main', 'artifact-do', { agent: agent.id, kind, text })

  refresh(agent.id)
</script>

<ArtifactsPanel {agent} windowed onpoint={relay('point')} onshare={relay('share')} onclose={() => getCurrentWindow().close()} />
