<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { errorText, t } from './i18n/index.svelte'
  import { editProject, type Project } from './projects.svelte'
  import { loadConnections, loadProjects, tickets, type Source } from './tickets.svelte'

  let { project }: { project: Project } = $props()

  let picking = $state(false)
  let query = $state('')
  let error = $state('')
  let mentions = $state<Record<string, [number, number]>>({})
  let hidden = $state(read())
  const dismissed = $derived(hidden[project.path] ?? [])

  function read(): Record<string, string[]> {
    try {
      return JSON.parse(localStorage.getItem('squadron.ticket-dismissed') ?? '{}')
    } catch {
      return {}
    }
  }

  if (!tickets.connections) loadConnections()
  $effect(() => {
    for (const c of tickets.connections ?? []) loadProjects(c.id)
  })
  $effect(() => {
    if (project.git && tickets.connections?.length) invoke<Record<string, [number, number]>>('ticket_mentions', { path: project.path }).then((found) => (mentions = found))
  })

  const name = (id: string) => tickets.connections?.find((c) => c.id === id)?.name ?? ''
  const linked = (connection: string, key: string) => project.tickets.some((s) => s.connection === connection && s.key === key)

  const suggestions = $derived(
    (tickets.connections ?? [])
      .flatMap((c) => (tickets.projects[c.id] ?? []).filter((p) => mentions[p.key]).map((p) => ({ connection: c.id, ...p, seen: mentions[p.key] })))
      .filter((s) => !linked(s.connection, s.key) && !dismissed.includes(`${s.connection}:${s.key}`))
      .sort((a, b) => b.seen[0] + b.seen[1] - a.seen[0] - a.seen[1])
      .slice(0, 3),
  )

  const groups = $derived(
    (tickets.connections ?? []).map((c) => ({
      connection: c,
      list: (tickets.projects[c.id] ?? []).filter((p) => {
        const q = query.trim().toLowerCase()
        return !q || p.key.toLowerCase().includes(q) || p.name.toLowerCase().includes(q)
      }),
    })),
  )

  async function save(next: Source[]) {
    error = ''
    try {
      await editProject(project.path, { tickets: next })
    } catch (e) {
      error = errorText(e)
    }
  }

  function toggle(connection: string, key: string, name: string) {
    save(linked(connection, key) ? project.tickets.filter((s) => !(s.connection === connection && s.key === key)) : [...project.tickets, { connection, key, name }])
  }

  function dismiss(id: string) {
    hidden = { ...read(), [project.path]: [...dismissed, id] }
    localStorage.setItem('squadron.ticket-dismissed', JSON.stringify(hidden))
  }

  function reason(seen: [number, number], key: string) {
    const [commits, branches] = seen
    if (commits && branches) return t('tickets.suggestionWhy', { commits, branches, key })
    return commits ? t('tickets.suggestionCommits', { count: commits, key }) : t('tickets.suggestionBranches', { count: branches, key })
  }
</script>

<svelte:window
  onclick={(e) => !(e.target as HTMLElement).closest(`[data-tickets="${project.path}"]`) && (picking = false)}
  onkeydown={(e) => e.key === 'Escape' && (picking = false)}
/>

{#if tickets.connections?.length}
  <div class="tickets" data-tickets={project.path}>
    <span class="label">{t('tickets.projectTickets')}</span>
    <div class="tags">
      {#each project.tickets as source (`${source.connection}:${source.key}`)}
        <span class="tag">
          <span class="logo" aria-hidden="true">J</span><strong>{source.key}</strong>{source.name}<span class="from">· {name(source.connection)}</span>
          <button aria-label={t('tickets.unlink', { key: source.key })} title={t('tickets.unlink', { key: source.key })} onclick={() => toggle(source.connection, source.key, source.name)}>✕</button>
        </span>
      {/each}
      <span class="pick">
        <button class="btn btn-secondary" aria-haspopup="menu" aria-expanded={picking} onclick={() => ((picking = !picking), (query = ''))}>{t('tickets.addJiraProject')}</button>
        {#if picking}
          <div class="menu" role="menu">
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field" placeholder={t('tickets.searchProject')} bind:value={query} autofocus />
            <div class="scroll">
              {#each groups as { connection, list } (connection.id)}
                <span class="sep">{connection.name}</span>
                {#if !tickets.projects[connection.id]}
                  <span class="empty">{t('common.loading')}</span>
                {:else if !list.length}
                  <span class="empty">{t('tickets.noProject')}</span>
                {/if}
                {#each list as p (p.key)}
                  <button role="menuitemcheckbox" aria-checked={linked(connection.id, p.key)} onclick={() => toggle(connection.id, p.key, p.name)}>
                    <span class="box" aria-hidden="true">{linked(connection.id, p.key) ? '✓' : ''}</span><strong>{p.key}</strong>{p.name}
                  </button>
                {/each}
              {/each}
            </div>
          </div>
        {/if}
      </span>
    </div>
    {#if suggestions.length}
      <div class="suggest">
        <span class="title">{t('tickets.suggestions')}</span>
        {#each suggestions as s (`${s.connection}:${s.key}`)}
          <span class="row">
            <span class="logo" aria-hidden="true">J</span><strong>{s.key}</strong>{s.name}<span class="from">· {name(s.connection)}</span>
            <span class="why">· {reason(s.seen, s.key)}</span>
            <button class="btn btn-secondary small" onclick={() => toggle(s.connection, s.key, s.name)}>{t('tickets.link')}</button>
            <button class="dismiss" aria-label={t('tickets.dismiss')} title={t('tickets.dismiss')} onclick={() => dismiss(`${s.connection}:${s.key}`)}>✕</button>
          </span>
        {/each}
      </div>
    {/if}
    {#if error}<span class="bad" role="alert">{error}</span>{/if}
  </div>
{/if}

<style>
  .tickets {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .label {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 4px 0 9px;
    border-radius: 4px;
    background: var(--surface-3);
    font-size: 12.5px;
  }

  .tag button,
  .dismiss {
    width: 20px;
    height: 20px;
    border-radius: 3px;
    color: var(--text-faint);
    font-size: 11px;
  }

  .tag button:hover,
  .dismiss:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  strong {
    font: 600 12px var(--mono);
  }

  .from {
    color: var(--text-faint);
  }

  .logo {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: 3px;
    background: var(--jira);
    color: #fff;
    font: 700 9px var(--sans);
  }

  .pick {
    position: relative;
  }

  .pick > .btn {
    height: 28px;
  }

  .menu {
    position: absolute;
    top: 34px;
    left: 0;
    z-index: 5;
    width: 330px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .menu .field {
    height: 32px;
  }

  .scroll {
    max-height: 320px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .sep {
    padding: 8px 9px 3px;
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .empty {
    padding: 6px 9px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 9px;
    border-radius: 4px;
    text-align: left;
    font-size: 13px;
  }

  .menu button:hover {
    background: var(--menu-hover);
  }

  .box {
    flex: none;
    width: 14px;
    height: 14px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1.5px solid var(--text-faint);
    border-radius: 3px;
    font-size: 10px;
    font-weight: 800;
  }

  [aria-checked='true'] .box {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-ink);
  }

  .suggest {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 9px 12px;
    border: 1px solid var(--ticket-border);
    border-radius: 6px;
    background: var(--ticket-bg);
  }

  .title {
    font-size: 11.5px;
    color: var(--ticket-muted);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
  }

  .why {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--ticket-muted);
  }

  .small {
    height: 24px;
    padding: 0 10px;
    font-size: 12px;
  }

  .bad {
    color: var(--err-text);
    font-size: 12.5px;
  }
</style>
