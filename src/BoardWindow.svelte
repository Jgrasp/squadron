<script lang="ts">
  import { emitTo, listen } from '@tauri-apps/api/event'
  import Board from './lib/Board.svelte'
  import type { Actions, Mirror } from './lib/board'
  import { current, projects, type Project } from './lib/projects.svelte'
  import { loadSettings } from './lib/settings.svelte'

  let agents = $state<Mirror[]>([])
  const project = $derived(current())
  const actions = new Proxy({} as Actions, {
    get: (_, name) => (...args: unknown[]) => emitTo('main', 'board-do', { name, args }),
  })

  loadSettings()
  listen('settings-changed', () => loadSettings())
  listen<{ projects: Project[]; current: string; agents: Mirror[] }>('board-state', ({ payload }) => {
    projects.list = payload.projects
    if (!current()) projects.current = payload.current
    agents = payload.agents
  }).then(() => emitTo('main', 'board-ready'))
</script>

<div class="app">
  {#if project}
    <Board {project} {agents} {actions} popout />
  {/if}
</div>

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
</style>
