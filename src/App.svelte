<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { emitTo, listen } from '@tauri-apps/api/event'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AddProject from './lib/AddProject.svelte'
  import EmptyProjects from './lib/EmptyProjects.svelte'
  import Header from './lib/Header.svelte'
  import ProjectView from './lib/ProjectView.svelte'
  import Settings from './lib/SettingsDialog.svelte'
  import Spotlight from './lib/Spotlight.svelte'
  import Confirm from './lib/Confirm.svelte'
  import Viewer from './lib/Viewer.svelte'
  import Password from './lib/Password.svelte'
  import { loadSettings, shortcut } from './lib/settings.svelte'
  import { agents, archiveIdle, closeAll, create, find, rescan, restore, selected, show, sleepIdle, waitingDetail } from './lib/agents.svelte'
  import { run, ui } from './lib/actions.svelte'
  import Customize from './lib/Customize.svelte'
  import { loadLayout } from './lib/layout-slots.svelte'
  import { setPanel } from './lib/layout.svelte'
  import { trackHistory } from './lib/history.svelte'
  import './lib/pilot.svelte'
  import Board from './lib/Board.svelte'
  import { follow, mirror, type Actions } from './lib/board'
  import Overview from './lib/Overview.svelte'
  import WhatsNew from './lib/WhatsNew.svelte'
  import StatusBar from './lib/StatusBar.svelte'
  import { activity } from './lib/stream'
  import { lookForUpdate, showChangelogOnce } from './lib/update.svelte'
  import { lookUpAll, lookUpLists } from './lib/pulls.svelte'
  import { watchRuns } from './lib/pipelines.svelte'
  import PullsView from './lib/PullsView.svelte'
  import { current, inspect, load, popOut, projects, setCards } from './lib/projects.svelte'

  const project = $derived(current())
  const mirrored = $derived(
    agents.list.map((a) => mirror(a, a.status === 'waiting' ? waitingDetail(a) : a.status === 'working' ? activity(a) : a.attention)),
  )

  const actions: Actions = {
    cards: setCards,
    async start(path, card, name, options, text, goal, worktree, files) {
      const id = await create(path, name, options, text, { goal, worktree, files })
      const cards = projects.list.find((p) => p.path === path)?.cards ?? []
      setCards(path, cards.map((c) => (c.id === card ? { ...c, agent: id, column: 'progress' } : c)))
    },
    show(id) {
      const agent = find(id)
      if (!agent) return
      show(agent)
      getCurrentWindow().setFocus()
    },
    diff(id) {
      const agent = find(id)
      if (!agent) return
      show(agent)
      setPanel(agent.project, 'diff')
      getCurrentWindow().setFocus()
    },
  }

  const boardState = () => ({ projects: $state.snapshot(projects.list), current: projects.current, agents: mirrored })

  $effect(() => {
    emitTo('board', 'board-state', boardState())
  })

  $effect(() => {
    for (const p of projects.list) {
      const cards = p.cards.map((c) => follow(c, mirrored.find((a) => a.id === c.agent)))
      if (cards.some((c, i) => c !== p.cards[i])) setCards(p.path, $state.snapshot(cards))
    }
  })

  listen('board-ready', () => emitTo('board', 'board-state', boardState()))
  listen<{ name: keyof Actions; args: unknown[] }>('board-do', ({ payload }) =>
    (actions[payload.name] as (...args: unknown[]) => void)(...payload.args),
  )

  loadSettings()
  loadLayout()
  load()
    .then(() => restore())
    .then(() => {
      trackHistory()
      archiveIdle()
      setInterval(() => (rescan(), archiveIdle(), sleepIdle()), 15_000)
      showChangelogOnce()
      lookForUpdate()
      setInterval(lookForUpdate, 10 * 60_000)
      const lookUpPulls = () => (lookUpAll(), projects.list.forEach((p) => lookUpLists(p.path)))
      lookUpPulls()
      setInterval(lookUpPulls, 3 * 60_000)
      watchRuns()
    })

  $effect(() => {
    const appWindow = getCurrentWindow()
    const unlisten = appWindow.onCloseRequested(async (event) => {
      event.preventDefault()
      await closeAll().catch(() => {})
      await appWindow.destroy()
    })
    return () => {
      unlisten.then((stop) => stop())
    }
  })

  $effect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== 'drop' || !event.payload.paths.length || ui.drops) return
      const [path] = event.payload.paths
      inspect(path).then((info) => info.exists && ui.adding === null && (ui.adding = path))
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
    if (/^https?:\/\//.test(href)) invoke('open_link', { url: href })
    else if (/^(\/|file:\/\/)/.test(href)) invoke('open_attachment', { path: decodeURI(href.replace(/^file:\/\//, '')) }).catch(() => {})
  }

  const SCOPED = ['newAgent', 'interrupt', 'editor', 'terminal', 'diff', 'sidebar']

  function shortcuts(event: KeyboardEvent) {
    if (ui.customizing && event.key === 'Escape') return (ui.customizing = false)
    if ((event.target as HTMLElement).closest?.('.xterm, .cm-editor')) return
    const action = shortcut(event)
    if (!action || (action !== 'quit' && document.querySelector('dialog[open]'))) return
    const inside = !projects.overview && !projects.board ? project : undefined
    if (SCOPED.includes(action) && !inside) return
    if (action !== 'interrupt') event.preventDefault()
    run(action, { project: inside, agent: inside && selected(inside.path) })
  }
</script>

<svelte:window onkeydown={shortcuts} onclick={links} onfocus={rescan} />

<div class="app">
  <Header onadd={() => (ui.adding = '')} />
  {#if projects.overview}
    <Overview />
  {:else if projects.list.length === 0}
    <EmptyProjects onadd={() => (ui.adding = '')} />
  {:else if project}
    {#key project.path}
      {#if projects.board}
        <Board {project} agents={mirrored} {actions} onclose={() => (projects.board = false)} onpopout={popOut} />
      {:else if projects.pulls}
        <PullsView {project} onclose={() => (projects.pulls = false)} />
      {:else}
        <ProjectView {project} />
      {/if}
    {/key}
  {/if}
  <StatusBar />
</div>

<WhatsNew />

{#if ui.settings}
  <Settings onclose={() => (ui.settings = null)} />
{/if}

{#if ui.customizing}
  <Customize />
{/if}

{#if ui.adding !== null}
  <AddProject path={ui.adding} onclose={() => (ui.adding = null)} />
{/if}

{#if ui.searching}
  <Spotlight onclose={() => (ui.searching = false)} />
{/if}

<Confirm />
<Viewer />
<Password />

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
</style>
