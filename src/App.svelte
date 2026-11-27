<script lang="ts">
  import { untrack } from 'svelte'
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
  import AskpassDialog from './lib/AskpassDialog.svelte'
  import StartAgentDialog from './lib/StartAgentDialog.svelte'
  import { loadSettings, settings, shortcut } from './lib/settings.svelte'
  import { agents, archive, archiveIdle, closeAll, create, find, followWaits, rescan, restore, selected, shelveHidden, show, sleepIdle, waitingDetail } from './lib/agents.svelte'
  import { run, ui } from './lib/actions.svelte'
  import { showWindow, tray } from './lib/tray'
  import Customize from './lib/Customize.svelte'
  import { loadLayout } from './lib/layout-slots.svelte'
  import { checkUpdates, currentEffect, loadExtensions } from './lib/extensions.svelte'
  import Effect from './lib/Effect.svelte'
import './lib/extensionFeed.svelte'
import './lib/extensionWindows.svelte'
  import { sweep } from './lib/cleanup.svelte'
  import { layout, setPanel } from './lib/layout.svelte'
  import { trackHistory } from './lib/history.svelte'
  import './lib/pilot.svelte'
  import Board from './lib/Board.svelte'
  import { follow, mirror, type Actions } from './lib/board'
  import Overview from './lib/Overview.svelte'
  import Usage from './lib/Usage.svelte'
  import ProjectsPanel from './lib/ProjectsPanel.svelte'
  import WhatsNew from './lib/WhatsNew.svelte'
  import StatusBar from './lib/StatusBar.svelte'
  import { activity } from './lib/stream'
  import { lookForUpdate, showChangelogOnce } from './lib/update.svelte'
  import { followPullWaits, lookUpAll, lookUpLists } from './lib/pulls.svelte'
  import { watchRuns } from './lib/pipelines.svelte'
  import { watchNotifications } from './lib/notify.svelte'
  import { followKey, syncFollows } from './lib/plugins.svelte'
  import PullsView from './lib/PullsView.svelte'
  import SidePanel from './lib/SidePanel.svelte'
  import { t } from './lib/i18n/index.svelte'
  import { ask } from './lib/asking.svelte'
  import PanelButtons from './lib/PanelButtons.svelte'
  import Splash from './lib/Splash.svelte'
  import TourOverlay from './lib/TourOverlay.svelte'
  import Welcome from './lib/Welcome.svelte'
  import { tour } from './lib/tour.svelte'
  import { boot, finished } from './lib/boot.svelte'
  import { current, inspect, load, popOut, projects, setCards } from './lib/projects.svelte'
  import { isDemo } from './lib/demoId'

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
      showWindow()
    },
    diff(id) {
      const agent = find(id)
      if (!agent) return
      show(agent)
      setPanel(agent.project, 'diff')
      showWindow()
    },
    archive: (ids) => Promise.all(ids.filter(find).map((id) => archive(id))),
  }

  const boardState = () => ({ projects: $state.snapshot(projects.list).filter((p) => !isDemo(p.path)), current: projects.current, agents: mirrored.filter((a) => !isDemo(a.id)) })

  $effect(() => {
    emitTo('board', 'board-state', boardState())
  })

  const followed = $derived(boot.settings && boot.projects ? followKey() : '')
  $effect(() => {
    if (followed) untrack(syncFollows)
  })

  $effect(() => {
    if (!settings.usage) projects.usage = false
  })

  $effect(() => {
    document.documentElement.style.setProperty('--conversation-width', settings.conversation_width ? `${settings.conversation_width}px` : '100%')
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

  loadSettings().finally(() => (boot.settings = true))
  loadLayout()
  loadExtensions().then(checkUpdates)
  setInterval(checkUpdates, 6 * 3600_000)
  setTimeout(sweep, 60_000)
  setInterval(sweep, 6 * 3600_000)
  load()
    .then(() => ((boot.projects = true), restore()))
    .then(() => {
      boot.agents = agents.list.length
      trackHistory()
      archiveIdle()
      setInterval(() => (rescan(), archiveIdle(), sleepIdle(), shelveHidden()), 15_000)
      finished.then(showChangelogOnce)
      finished.then(() => {
        if (!settings.tour_offered && projects.list.length === 0) tour.welcome = true
      })
      lookForUpdate()
      setInterval(lookForUpdate, 10 * 60_000)
      const lookUpPulls = () => (lookUpAll(), Promise.allSettled(projects.list.map((p) => lookUpLists(p.path))))
      lookUpPulls().then(() => (boot.pulls = true))
      setInterval(lookUpPulls, 3 * 60_000)
      setInterval(() => (followPullWaits(), followWaits()), 20_000)
      watchRuns()
      watchNotifications()
    })

  $effect(() => {
    const appWindow = getCurrentWindow()
    let asking = false
    const unlisten = appWindow.onCloseRequested(async (event) => {
      event.preventDefault()
      const quitting = ui.quitting
      ui.quitting = false
      if (settings.background && !quitting) return appWindow.hide()
      if (asking) return
      const busy = agents.list.filter((a) => a.status === 'working')
      asking = busy.length > 0
      if (busy.length) await showWindow().catch(() => {})
      if (
        busy.length &&
        !(await ask(t('shell.quitAsk', { count: busy.length }), {
          title: t('shell.quitTitle'),
          kind: 'warning',
          okLabel: t('shell.quit'),
        }).finally(() => (asking = false)))
      )
        return
      await closeAll().catch(() => {})
      await appWindow.destroy()
    })
    return () => {
      unlisten.then((stop) => stop())
    }
  })

  $effect(() => {
    if (!settings.background) return
    const closing = tray({ show: t('shell.showWindow'), quit: t('shell.quit') }, () => run('quit', {}))
    return () => {
      closing.then((close) => close())
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

  const SCOPED = ['newAgent', 'interrupt', 'editor', 'terminal', 'diff', 'sidebar', 'find']

  function shortcuts(event: KeyboardEvent) {
    if (ui.customizing && event.key === 'Escape') return (ui.customizing = false)
    if (ui.projects && event.key === 'Escape' && !document.querySelector('dialog[open]')) return (ui.projects = false)
    if ((event.target as HTMLElement).closest?.('.xterm, .cm-editor')) return
    const action = shortcut(event)
    if (!action || (action !== 'quit' && document.querySelector('dialog[open]'))) return
    const inside = !projects.overview ? project : undefined
    if (SCOPED.includes(action) && !inside) return
    if (action !== 'interrupt') event.preventDefault()
    run(action, { project: inside, agent: inside && selected(inside.path) })
  }
</script>

<svelte:window onkeydown={shortcuts} onfocus={rescan} />

<div class="app">
  <Header onadd={() => (ui.adding = '')} />
  {#if projects.usage}
    <Usage />
  {:else if projects.overview}
    <Overview />
  {:else if projects.list.length === 0}
    <EmptyProjects onadd={() => (ui.adding = '')} />
  {:else if project}
    {#key project.path}
      <ProjectView {project}>
        {#snippet overlay()}
          {#if projects.board}
            <SidePanel kind="kanban" title={t('shell.kanban')} onclose={() => (projects.board = false)}>
              <Board {project} agents={mirrored} {actions} onpopout={popOut}>
                {#snippet controls()}<PanelButtons kind="kanban" onclose={() => (projects.board = false)} />{/snippet}
              </Board>
            </SidePanel>
          {:else if projects.pulls}
            <SidePanel kind="pulls" title={t('shell.pullRequests')} onclose={() => (projects.pulls = false)}>
              <PullsView {project}>
                {#snippet controls()}<PanelButtons kind="pulls" onclose={() => (projects.pulls = false)} />{/snippet}
              </PullsView>
            </SidePanel>
          {/if}
        {/snippet}
      </ProjectView>
    {/key}
  {/if}
  <StatusBar />
</div>

{#if currentEffect()}
  <Effect fx={currentEffect()!} />
{/if}

{#if ui.projects}
  <ProjectsPanel />
{/if}

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
<AskpassDialog />
<StartAgentDialog />
<TourOverlay />
<Welcome />
<Splash />

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
</style>
