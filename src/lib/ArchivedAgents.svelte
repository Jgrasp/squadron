<script lang="ts">
  import { ask } from './asking.svelte'
  import { ui } from './actions.svelte'
  import { archived, forget, unarchive, type Archived } from './agents.svelte'
  import { home, projects } from './projects.svelte'
  import { date, errorText, t } from './i18n/index.svelte'

  let list = $state<Archived[] | null>(null)
  let error = $state('')

  const refresh = async () => (list = await archived())
  refresh()

  const projectOf = (a: Archived) => projects.list.find((p) => p.path === a.project)
  const line = (a: Archived, project?: string) => {
    const values = { project: project ?? home(a.project) }
    return a.archived
      ? t('settings.archivedOn', { ...values, when: date(a.archived, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }) })
      : t('settings.archivedLongAgo', values)
  }

  async function restore(a: Archived) {
    error = ''
    try {
      await unarchive(a.id)
      projects.overview = false
      projects.board = false
      projects.current = a.project
      ui.settings = null
    } catch (e) {
      error = errorText(e)
    }
  }

  async function remove(a: Archived) {
    const sure = await ask(t('settings.deleteAsk', { name: a.name }), {
      title: t('settings.deleteTitle'),
      kind: 'warning',
      okLabel: t('common.delete'),
      cancelLabel: t('settings.keepIt'),
    })
    if (!sure) return
    await forget(a.id)
    refresh()
  }
</script>

<p class="intro">{t('settings.archivedIntro')}</p>
{#if list && !list.length}
  <p class="empty">{t('settings.noArchived')}</p>
{/if}
<ul>
  {#each list ?? [] as a (a.id)}
    {@const project = projectOf(a)}
    <li>
      <span class="who">
        <strong>{a.name}</strong>
        <small>{line(a, project?.name)}</small>
      </span>
      <button class="btn btn-secondary" disabled={!project} title={project ? t('settings.backTo', { project: project.name }) : t('settings.projectRemoved')} onclick={() => restore(a)}>{t('common.restore')}</button>
      <button class="remove" onclick={() => remove(a)}>{t('common.delete')}</button>
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
