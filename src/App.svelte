<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import AddProject from './lib/AddProject.svelte'
  import EmptyProjects from './lib/EmptyProjects.svelte'
  import Header from './lib/Header.svelte'
  import ProjectView from './lib/ProjectView.svelte'
  import Settings, { type Tab } from './lib/Settings.svelte'
  import { loadSettings } from './lib/settings.svelte'
  import { jumpToWaiting, loadToday, restore, saveAll } from './lib/agents.svelte'
  import Overview from './lib/Overview.svelte'
  import { current, load, projects } from './lib/projects.svelte'

  let adding: string | null = $state(null)
  let settingsTab = $state<Tab | null>(null)
  const project = $derived(current())

  loadSettings()
  loadToday()
  load().then(restore)

  $effect(() => {
    const appWindow = getCurrentWindow()
    const unlisten = appWindow.onCloseRequested(async (event) => {
      event.preventDefault()
      await saveAll().catch(() => {})
      await appWindow.destroy()
    })
    return () => {
      unlisten.then((stop) => stop())
    }
  })

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
    else if (event.ctrlKey && event.key === 'j' && !document.querySelector('dialog[open]')) {
      event.preventDefault()
      if (!jumpToWaiting()) projects.overview = true
    }
  }
</script>

<svelte:window onkeydown={shortcuts} onclick={links} />

<div class="app">
  <Header onadd={() => (adding = '')} onsettings={(tab) => (settingsTab = tab)} />
  {#if projects.overview}
    <Overview />
  {:else if projects.list.length === 0}
    <EmptyProjects onadd={() => (adding = '')} />
  {:else if project}
    {#key project.path}
      <ProjectView {project} />
    {/key}
  {/if}
</div>

{#if settingsTab !== null}
  <Settings bind:tab={settingsTab} onclose={() => (settingsTab = null)} />
{/if}

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
