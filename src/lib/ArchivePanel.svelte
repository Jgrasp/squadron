<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog'
  import { archived, forget, unarchive, type Archived } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import Dialog from './Dialog.svelte'
  import { cost } from './format'
  import { projects, type Project } from './projects.svelte'
  import { newAgent } from './stream'

  let { project, onclose }: { project: Project; onclose: () => void } = $props()

  let list = $state<Archived[] | null>(null)
  let chosen = $state('')
  let search = $state('')
  let error = $state('')

  async function refresh() {
    list = (await archived()).filter((a) => a.project === project.path).sort((a, b) => (b.archived ?? 0) - (a.archived ?? 0))
    if (!list.some((a) => a.id === chosen)) chosen = list[0]?.id ?? ''
  }
  refresh()

  const shown = $derived((list ?? []).filter((a) => a.name.toLowerCase().includes(search.trim().toLowerCase())))
  const current = $derived(list?.find((a) => a.id === chosen))
  const preview = $derived(current && { ...newAgent(current.id, current.project, current.name, current.options), ...current, asks: [], alive: false })
  const messages = (a: Archived) => a.entries.filter((e) => e.kind === 'user').length
  const when = (at?: number) =>
    at ? new Date(at).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }) : '—'

  async function restore(a: Archived) {
    error = ''
    try {
      await unarchive(a.id)
      projects.overview = false
      projects.board = false
      projects.current = a.project
      onclose()
    } catch (e) {
      error = String(e)
    }
  }

  async function remove(a: Archived) {
    const sure = await ask(`Delete “${a.name}” for good? Squadron forgets it; Claude Code keeps the session in its own history.`, {
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

<Dialog title="Archived agents · {project.name}" full {onclose}>
  <div class="archive">
    <aside>
      <input class="field" type="search" placeholder="Search {list?.length ?? ''} archived agents" bind:value={search} />
      <ul>
        {#each shown as a (a.id)}
          <li>
            <button aria-current={a.id === chosen ? 'true' : undefined} onclick={() => (chosen = a.id)}>
              <strong>{a.name}</strong>
              <small>Archived {when(a.archived)} · {messages(a)} message{messages(a) > 1 ? 's' : ''}{a.cost ? ` · ${cost(a.cost)}` : ''}</small>
            </button>
          </li>
        {/each}
      </ul>
      {#if list && !list.length}<p class="empty">No archived agents in {project.name}.</p>{/if}
    </aside>
    <section>
      {#if current && preview}
        <header>
          <div class="who">
            <h2>{current.name}</h2>
            <small>Last activity {when(current.since)} · archived {when(current.archived)}</small>
          </div>
          {#if error}<span class="bad" role="alert">{error}</span>{/if}
          <button class="btn btn-danger" onclick={() => remove(current)}>Delete…</button>
          <button class="btn btn-primary" onclick={() => restore(current)}>Restore</button>
        </header>
        <div class="scroller">
          {#key current.id}
            <div class="column"><Conversation agent={preview} ondiff={() => {}} /></div>
          {/key}
        </div>
      {:else if list}
        <p class="empty center">Pick an archived agent to read its conversation.</p>
      {/if}
    </section>
  </div>
</Dialog>

<style>
  .archive {
    flex: 1;
    min-height: 0;
    display: flex;
    border-top: 1px solid var(--border);
    margin-top: 14px;
  }

  aside {
    flex: none;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li button {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px;
    border-radius: 6px;
    text-align: left;
  }

  li button:hover {
    background: var(--surface-1);
  }

  li button[aria-current='true'] {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  li strong {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  li small,
  .who small {
    font-size: 12px;
    color: var(--text-dim);
  }

  section {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 22px;
    border-bottom: 1px solid var(--border);
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  h2 {
    margin: 0;
    font-size: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .scroller {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 20px 60px;
  }

  .column {
    max-width: 740px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .empty {
    margin: 0;
    color: var(--text-muted);
  }

  .empty.center {
    margin: auto;
  }

  .bad {
    color: var(--err-text);
    font-size: 12.5px;
  }
</style>
