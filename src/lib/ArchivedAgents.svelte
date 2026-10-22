<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import { archived, forget, unarchive, type Archived } from './agents.svelte'
  import { cost } from './format'
  import { home, projects } from './projects.svelte'

  let list = $state<Archived[] | null>(null)
  let error = $state('')

  const refresh = async () => (list = await archived())
  refresh()

  const projectOf = (a: Archived) => projects.list.find((p) => p.path === a.project)
  const when = (at?: number) =>
    at ? new Date(at).toLocaleString('en-GB', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }) : 'some time ago'

  async function restore(a: Archived) {
    error = ''
    try {
      await unarchive(a.id)
      projects.overview = false
      projects.board = false
      projects.current = a.project
      ui.settings = null
    } catch (e) {
      error = String(e)
    }
  }

  async function remove(a: Archived) {
    const sure = await ask(`Delete ${a.name} for good? Squadron forgets it; Claude Code keeps the session in its own history.`, {
      title: 'Delete an archived agent',
      kind: 'warning',
      okLabel: 'Delete',
      cancelLabel: 'Keep it',
    })
    if (!sure) return
    await forget(a.id)
    refresh()
  }
</script>

<p class="intro">Archived agents are stopped and kept here. Restoring one brings it back to its project with its conversation; your next message resumes its session.</p>
{#if list && !list.length}
  <p class="empty">No archived agents.</p>
{/if}
<ul>
  {#each list ?? [] as a (a.id)}
    {@const project = projectOf(a)}
    <li>
      <span class="who">
        <strong>{a.name}</strong>
        <small>{project?.name ?? home(a.project)} · archived {when(a.archived)} · {cost(a.cost)}</small>
      </span>
      <button class="btn btn-secondary" disabled={!project} title={project ? `Back to ${project.name}` : 'Its project was removed'} onclick={() => restore(a)}>Restore</button>
      <button class="remove" onclick={() => remove(a)}>Delete</button>
    </li>
  {/each}
</ul>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<style>
  .intro,
  .empty {
    margin: 0 0 14px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .remove {
    height: 30px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--err-text);
  }

  .remove:hover {
    background: var(--err-bg);
  }

  .bad {
    color: var(--err-text);
  }
</style>
