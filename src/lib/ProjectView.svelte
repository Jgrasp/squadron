<script lang="ts">
  import { ask, open } from '@tauri-apps/plugin-dialog'
  import AgentList from './AgentList.svelte'
  import AgentView from './AgentView.svelte'
  import { ui } from './actions.svelte'
  import { archiveProject, create, ofProject, selected } from './agents.svelte'
  import {
    dragSidebar,
    FOLDED_SIDEBAR,
    layout,
    MAX_SIDEBAR,
    MIN_SIDEBAR,
    resizeSidebar,
    setCollapsed,
  } from './layout.svelte'
  import { hint } from './settings.svelte'
  import NewAgent from './NewAgent.svelte'
  import { load, move, remove, toggleBoard, type Project } from './projects.svelte'

  let { project }: { project: Project } = $props()
  let error = $state('')

  const agent = $derived(selected(project.path))

  async function retire() {
    const running = ofProject(project.path).length
    const confirmed = await ask(
      `Remove ${project.name} from Squadron? The folder is left untouched.${running ? ` Its ${running} agent${running > 1 ? 's' : ''} will be stopped.` : ''}`,
      { title: 'Remove project', kind: 'warning', okLabel: 'Remove', cancelLabel: 'Cancel' },
    )
    if (!confirmed) return
    await archiveProject(project.path)
    await remove(project.path)
  }

  async function relocate() {
    const chosen = await open({ directory: true })
    if (typeof chosen !== 'string') return
    try {
      await move(project.path, chosen)
      error = ''
    } catch (e) {
      error = String(e)
    }
  }

  function drag(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement
    const start = event.clientX - (layout.collapsed ? FOLDED_SIDEBAR : layout.sidebar)
    handle.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => dragSidebar(e.clientX - start)
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
  }

  function nudge(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') {
      if (layout.sidebar <= MIN_SIDEBAR) setCollapsed(true)
      else resizeSidebar(layout.sidebar - 16)
    } else if (event.key === 'ArrowRight') {
      if (layout.collapsed) setCollapsed(false)
      else resizeSidebar(layout.sidebar + 16)
    }
  }


</script>

<div class="body">
  <AgentList {project} current={agent} onnew={() => (ui.creating = true)} />
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="resizer"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize the agent list"
    aria-valuenow={layout.collapsed ? FOLDED_SIDEBAR : layout.sidebar}
    aria-valuemin={FOLDED_SIDEBAR}
    aria-valuemax={MAX_SIDEBAR}
    tabindex="0"
    onpointerdown={drag}
    onkeydown={nudge}
  ></div>

  {#if !project.exists}
    <main class="center">
      <section class="missing" role="alert" aria-labelledby="missing">
        <div class="title">
          <span class="icon" aria-hidden="true">
            <svg width="16" height="16" viewBox="0 0 24 24"><path d="M12 3l9 16H3zM12 10v4M12 17h.01" /></svg>
          </span>
          <h1 id="missing">The {project.name} folder cannot be found</h1>
        </div>
        <code>{project.path}</code>
        <p>
          It may have been moved, renamed, or it lives on an unmounted disk. Point to its new location
          to create agents in {project.name}.
        </p>
        {#if error}<p class="bad">{error}</p>{/if}
        <div class="row">
          <button class="btn btn-primary" onclick={relocate}>Choose the new location…</button>
          <button class="btn btn-danger" onclick={load}>Retry</button>
          <button class="btn btn-danger" onclick={retire}>Remove project</button>
        </div>
      </section>
    </main>
  {:else if agent}
    {#key agent.id}
      <AgentView {agent} />
    {/key}
  {:else}
    <main class="center">
      <div class="box">
        <span class="plus" aria-hidden="true">
          <svg width="24" height="24" viewBox="0 0 24 24"><path d="M12 5v14M5 12h14" /></svg>
        </span>
        <h1>Start the first {project.name} agent</h1>
        <p>Give it a name, pick the model, the effort and the permission mode, then write its first instruction.</p>
        <div class="row">
          <button class="btn btn-primary big" onclick={() => (ui.creating = true)}>New agent</button>
          <kbd>{hint('newAgent')}</kbd>
          <button class="btn btn-secondary big" onclick={toggleBoard}>Kanban</button>
          <kbd>{hint('kanban')}</kbd>
        </div>
      </div>
    </main>
  {/if}
</div>

{#if ui.creating}
  <NewAgent {project} start={(name, options, text, goal, worktree) => create(project.path, name, options, text, { goal, worktree })} onclose={() => (ui.creating = false)} />
{/if}

<style>
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .resizer {
    width: 6px;
    margin: 0 -3px;
    flex: none;
    position: relative;
    z-index: 2;
    cursor: col-resize;
  }

  .resizer:hover,
  .resizer:focus-visible,
  .resizer:active {
    background: var(--border-strong);
    outline: none;
  }

  .center {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    background: var(--bg-main);
  }

  .box {
    width: 440px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
  }

  .plus {
    width: 56px;
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: rgba(224, 138, 91, 0.12);
    box-shadow: inset 0 0 0 1px rgba(224, 138, 91, 0.5);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .plus svg {
    stroke: var(--accent-soft);
  }

  h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.6;
    text-wrap: pretty;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .big {
    height: 44px;
    padding: 0 22px;
  }

  kbd {
    font: 12.5px var(--mono);
    color: var(--text-dim);
  }

  .missing {
    width: 560px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 22px;
    border-radius: 6px;
    background: var(--err-bg);
    box-shadow: 0 0 0 1px var(--err-border);
  }

  .missing .title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .missing h1 {
    font-size: 16px;
    color: var(--text-strong);
  }

  .missing .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--err);
    color: var(--err-ink);
  }

  .missing code {
    padding: 8px 12px;
    border-radius: 4px;
    background: var(--err-pre);
    color: var(--err-text);
    font-size: 12.5px;
  }

  .missing p {
    color: var(--err-muted);
  }

  .missing .btn {
    height: 36px;
  }

  .btn-danger {
    background: #3a2420;
    color: #f8d5cc;
  }

  .bad {
    color: var(--err-text);
  }
</style>
