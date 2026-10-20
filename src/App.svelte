<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import AddProject from './lib/AddProject.svelte'
  import EmptyProjects from './lib/EmptyProjects.svelte'
  import Header from './lib/Header.svelte'
  import ProjectView from './lib/ProjectView.svelte'
  import { current, load, projects } from './lib/projects.svelte'

  let adding: string | null = $state(null)
  const project = $derived(current())

  load()

  $effect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'drop' && event.payload.paths.length) adding = event.payload.paths[0]
    })
    return () => {
      unlisten.then((stop) => stop())
    }
  })

  function links(event: MouseEvent) {
    const link = (event.target as HTMLElement).closest('a[href]')
    const href = link?.getAttribute('href')
    if (!href || href.startsWith('#')) return
    event.preventDefault()
    if (/^https?:\/\//.test(href)) openUrl(href)
  }

  function shortcuts(event: KeyboardEvent) {
    if (event.ctrlKey && event.key === 'q') getCurrentWindow().close()
  }
</script>

<svelte:window onkeydown={shortcuts} onclick={links} />

<div class="app">
  <Header onadd={() => (adding = '')} />
  {#if projects.list.length === 0}
    <EmptyProjects onadd={() => (adding = '')} />
  {:else if project}
    {#key project.path}
      <ProjectView {project} />
    {/key}
  {/if}
</div>

{#if adding !== null}
  <AddProject path={adding} onclose={() => (adding = null)} />
{/if}

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
</style>
