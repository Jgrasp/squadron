<script lang="ts">
  import { ask } from '@tauri-apps/plugin-dialog'
  import { archived, forget, unarchive, type Archived } from './agents.svelte'
  import Conversation from './Conversation.svelte'
  import Dialog from './Dialog.svelte'
  import { projects, type Project } from './projects.svelte'
  import { newAgent } from './stream'
  import { date, errorText, t } from './i18n/index.svelte'

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
    at ? date(at, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }) : '—'

  async function restore(a: Archived) {
    error = ''
    try {
      await unarchive(a.id)
      projects.overview = false
      projects.board = false
      projects.current = a.project
      onclose()
    } catch (e) {
      error = errorText(e)
    }
  }

  async function remove(a: Archived) {
    const sure = await ask(t('shell.deleteArchivedAsk', { name: a.name }), {
      title: t('shell.deleteArchivedTitle'),
      kind: 'warning',
      okLabel: t('common.delete'),
      cancelLabel: t('shell.keepIt'),
    })
    if (!sure) return
    await forget(a.id)
    refresh()
  }
</script>

<Dialog title={t('shell.archivedAgentsDialog', { name: project.name })} full {onclose}>
  <div class="archive">
    <aside>
      <input class="field" type="search" placeholder={list ? t('shell.searchArchived', { count: list.length }) : t('shell.searchArchivedAll')} bind:value={search} />
      <ul>
        {#each shown as a (a.id)}
          <li>
            <button aria-current={a.id === chosen ? 'true' : undefined} onclick={() => (chosen = a.id)}>
              <strong>{a.name}</strong>
              <small>{[t('shell.archivedWhen', { when: when(a.archived) }), t('shell.messageCount', { count: messages(a) })].join(' · ')}</small>
            </button>
          </li>
        {/each}
      </ul>
      {#if list && !list.length}<p class="empty">{t('shell.noArchived', { name: project.name })}</p>{/if}
    </aside>
    <section>
      {#if current && preview}
        <header>
          <div class="who">
            <h2>{current.name}</h2>
            <small>{t('shell.archivedDates', { since: when(current.since), archived: when(current.archived) })}</small>
          </div>
          {#if error}<span class="bad" role="alert">{error}</span>{/if}
          <button class="btn btn-danger" onclick={() => remove(current)}>{t('common.deleteEllipsis')}</button>
          <button class="btn btn-primary" onclick={() => restore(current)}>{t('common.restore')}</button>
        </header>
        <div class="scroller">
          {#key current.id}
            <div class="column"><Conversation agent={preview} ondiff={() => {}} /></div>
          {/key}
        </div>
      {:else if list}
        <p class="empty center">{t('shell.pickArchived')}</p>
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
