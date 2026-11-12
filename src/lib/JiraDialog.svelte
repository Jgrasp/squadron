<script module lang="ts">
  import { emptyFilters, type Filters } from './jira'

  const STORE = 'squadron.jira-filters'

  function stored(): Record<string, Filters> {
    try {
      return JSON.parse(localStorage.getItem(STORE) ?? '{}')
    } catch {
      return {}
    }
  }

  function load(path: string): Filters {
    const empty = emptyFilters()
    const found = stored()[path]
    return found ? { ...empty, ...found, hidden: { ...empty.hidden, ...found.hidden } } : empty
  }

  function keep(path: string, filters: Filters) {
    try {
      localStorage.setItem(STORE, JSON.stringify({ ...stored(), [path]: filters }))
    } catch {}
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte'
  import Avatar from './Avatar.svelte'
  import { COLUMNS, type Column } from './board'
  import Dialog from './Dialog.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { FACETS, filterTickets, inSource, kindTone, sourceId, statusColumns, URGENCY, values, type Facet } from './jira'
  import type { Project } from './projects.svelte'
  import { loadSprint, sameTicket, tickets, type Summary } from './tickets.svelte'

  let {
    project,
    task,
    viewing,
    onview,
    oncreate,
    onclose,
  }: {
    project: Project
    task: (ticket: Summary) => Column | null
    viewing: { connection: string; key: string } | null
    onview: (ticket: Summary) => void
    oncreate: (ticket: Summary) => void
    onclose: () => void
  } = $props()

  const FACET_LABELS: Record<Facet, Key> = { status: 'tickets.status', kind: 'tickets.kind', priority: 'tickets.priority', epic: 'tickets.epic' }
  const COLUMN_LABELS = Object.fromEntries(COLUMNS) as Record<Column, Key>

  const path = untrack(() => project.path)
  let f = $state(load(path))
  let search = $state<HTMLInputElement>()
  let root = $state<HTMLElement>()

  $effect(() => keep(path, $state.snapshot(f)))
  $effect(() => search?.focus())

  const sprint = $derived(tickets.sprints[project.path])
  const all = $derived(sprint?.tickets ?? [])
  const source = $derived(project.tickets.find((s) => sourceId(s) === f.source))
  const scope = $derived(source ? all.filter((x) => inSource(x, source)) : all)
  const shown = $derived(filterTickets(all, f, project.tickets, (x) => !!task(x)))
  const columns = $derived(statusColumns(shown))
  const statuses = $derived(statusColumns(scope))
  const people = $derived(
    [...new Map(scope.filter((x) => x.assignee).map((x) => [x.assignee!.name, x.assignee!])).values()].sort((a, b) => a.name.localeCompare(b.name)),
  )
  const names = $derived([...people.map((p) => p.name), ...(scope.some((x) => !x.assignee) ? [''] : [])])
  const filtered = $derived(f.mine || f.free || !!f.people.length || !!f.query.trim() || FACETS.some((facet) => f.hidden[facet].length))

  const count = (s: (typeof project.tickets)[number]) => all.filter((x) => inSource(x, s)).length
  const tally = (match: (x: Summary) => boolean) => scope.filter(match).length
  const urgencyOf = (priority: string) => all.find((x) => x.priority === priority)?.urgency ?? 0
  const valueLabel = (facet: Facet, value: string) => value || (facet === 'epic' ? t('tickets.noEpic') : facet === 'priority' ? t('tickets.noPriority') : '—')
  const listed = (name: string) => !f.people.length || f.people.includes(name)

  function toggle(list: string[], value: string) {
    return list.includes(value) ? list.filter((v) => v !== value) : [...list, value]
  }

  function togglePerson(name: string) {
    const next = toggle(f.people.length ? f.people : names, name)
    f.people = next.length === names.length ? [] : next
  }

  function reset() {
    f = { ...emptyFilters(), source: f.source }
  }

  function keys(event: KeyboardEvent) {
    const target = event.target as HTMLElement
    const mine = root?.closest('dialog')
    if (event.key !== '/' || !mine || target.closest('dialog') !== mine || target.closest('input, textarea, [contenteditable]')) return
    event.preventDefault()
    search?.focus()
  }
</script>

<svelte:window onkeydown={keys} />

<Dialog title={t('tickets.jiraTickets')} width={1180} {onclose}>
  <div class="body" bind:this={root}>
    <p class="help">
      <span>{source ? source.name || source.key : project.tickets.length > 1 ? t('tickets.allProjects') : project.tickets[0]?.name || project.tickets[0]?.key}</span>
      {#if sprint?.sprints.length}<span>· {sprint.sprints.join(' · ')}</span>{/if}
      <button class="refresh" title={t('tickets.refresh')} aria-label={t('tickets.refresh')} disabled={sprint?.loading} onclick={() => loadSprint(project, true)}>
        <svg class:spin={sprint?.loading} width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" /></svg>
      </button>
    </p>

    <div class="split">
      <aside class="rail" aria-label={t('tickets.filters')}>
        {#if project.tickets.length > 1}
          <section>
            <h3>{t('tickets.project')}</h3>
            <button class="opt" aria-pressed={!source} onclick={() => (f.source = '')}>
              {t('tickets.allSources')}<small>{all.length}</small>
            </button>
            {#each project.tickets as s (sourceId(s))}
              <button class="opt" aria-pressed={source === s} onclick={() => (f.source = sourceId(s))}>
                <span class="label">{s.name || s.key}</span> <span class="tag">{s.key}</span><small>{count(s)}</small>
              </button>
            {/each}
          </section>
        {/if}

        <section>
          <h3>{t('tickets.show')}</h3>
          <label class="opt"><input type="checkbox" bind:checked={f.mine} />{t('tickets.mineOnly')}<small>{tally((x) => x.mine)}</small></label>
          <label class="opt"><input type="checkbox" bind:checked={f.free} />{t('tickets.withoutTask')}<small>{tally((x) => !task(x))}</small></label>
        </section>

        <section>
          <h3>{t('tickets.status')}</h3>
          {#each statuses as s (s.status)}
            <label class="opt">
              <input type="checkbox" checked={!f.hidden.status.includes(s.status)} onchange={() => (f.hidden.status = toggle(f.hidden.status, s.status))} />
              <span class="jira-status {s.category}">{s.status}</span>
              <small>{s.tickets.length}</small>
            </label>
          {/each}
        </section>

        {#if names.length}
          <section>
            <h3>{t('tickets.assignee')}</h3>
            {#each people as p (p.name)}
              <label class="opt">
                <input type="checkbox" checked={listed(p.name)} onchange={() => togglePerson(p.name)} />
                <Avatar name={p.name} src={p.avatar} size={20} /><span class="label">{p.name}</span>
                <small>{tally((x) => x.assignee?.name === p.name)}</small>
              </label>
            {/each}
            {#if names.includes('')}
              <label class="opt">
                <input type="checkbox" checked={listed('')} onchange={() => togglePerson('')} />
                <span class="nobody" aria-hidden="true">?</span>{t('tickets.unassigned')}
                <small>{tally((x) => !x.assignee)}</small>
              </label>
            {/if}
          </section>
        {/if}

        {#each FACETS.filter((facet) => facet !== 'status') as facet (facet)}
          {@const options = values(scope, facet)}
          {#if options.length}
            <section>
              <h3>{t(FACET_LABELS[facet])}</h3>
              {#each options as value (value)}
                <label class="opt">
                  <input type="checkbox" checked={!f.hidden[facet].includes(value)} onchange={() => (f.hidden[facet] = toggle(f.hidden[facet], value))} />
                  {#if facet === 'kind'}<span class="jira-kind {kindTone(value)}" aria-hidden="true">{value[0]?.toUpperCase()}</span>{/if}
                  {#if facet === 'priority' && urgencyOf(value)}<span class="jira-urgency u{urgencyOf(value)}" aria-hidden="true">{URGENCY[urgencyOf(value)]}</span>{/if}
                  {#if facet === 'epic' && value}<span class="epic">{value}</span>{:else}<span class="label">{valueLabel(facet, value)}</span>{/if}
                  <small>{tally((x) => x[facet] === value)}</small>
                </label>
              {/each}
            </section>
          {/if}
        {/each}
      </aside>

      <div class="main">
        <div class="bar">
          <label class="search">
            <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg>
            <input bind:this={search} bind:value={f.query} placeholder={t('tickets.filterTickets')} aria-label={t('tickets.filterTickets')} />
            <kbd>/</kbd>
          </label>
          <span class="spacer"></span>
          {#if filtered}
            <span class="shown">{t('tickets.ticketsShown', { shown: shown.length, total: scope.length })}</span>
            <button class="reset" onclick={reset}>{t('tickets.resetFilters')}</button>
          {:else if scope.length}
            <span class="shown">{t('tickets.ticketCount', { count: scope.length })}</span>
          {/if}
        </div>

        {#each sprint?.errors ?? [] as error, i (i)}<p class="note bad" role="alert">{errorText(error)}</p>{/each}
        {#if sprint && !sprint.loading && !all.length && !sprint.errors.length}
          <p class="empty"><strong>{t('tickets.noSprint')}</strong></p>
        {:else if all.length && !shown.length}
          <div class="empty">
            <strong>{t('tickets.noMatch')}</strong>
            <button class="btn btn-secondary" onclick={reset}>{t('tickets.resetFilters')}</button>
          </div>
        {:else}
          <div class="columns">
            {#each columns as column (column.status)}
              <section aria-label={column.status}>
                <h4><span class="jira-status {column.category}">{column.status}</span><span class="count">{column.tickets.length}</span></h4>
                <ul>
                  {#each column.tickets as ticket (`${ticket.connection}:${ticket.key}`)}
                    {@const state = task(ticket)}
                    <li class="ticket" class:linked={!!state} class:open={sameTicket(viewing, ticket)}>
                      <button class="ticket-open" title={ticket.title} onclick={() => onview(ticket)}>
                        <span class="meta">
                          <span class="jira-kind {kindTone(ticket.kind)}" title={ticket.kind} aria-hidden="true">{ticket.kind[0]?.toUpperCase()}</span>
                          <span class="key">{ticket.key}</span>
                          {#if ticket.urgency}<span class="jira-urgency u{ticket.urgency}" title={ticket.priority} aria-hidden="true">{URGENCY[ticket.urgency]}</span>{/if}
                          {#if ticket.epic}<span class="epic">{ticket.epic}</span>{/if}
                        </span>
                        <span class="name">{ticket.title}</span>
                      </button>
                      <span class="foot">
                        {#if state}
                          <span class="state"><span class="dot {state}" aria-hidden="true"></span>{t(COLUMN_LABELS[state])}</span>
                        {:else}
                          <button class="create" onclick={() => oncreate(ticket)}>+ {t('tickets.createTask')}</button>
                        {/if}
                        <span class="assignee" title={ticket.assignee?.name ?? t('tickets.unassigned')}>
                          {#if ticket.assignee}<Avatar name={ticket.assignee.name} src={ticket.assignee.avatar} size={22} />{:else}<span class="nobody" aria-hidden="true">?</span>{/if}
                        </span>
                      </span>
                    </li>
                  {/each}
                </ul>
              </section>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</Dialog>

<style>
  .body {
    height: min(720px, calc(100vh - 150px));
    display: flex;
    flex-direction: column;
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .help {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 0 22px 14px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
    color: var(--text-dim);
  }

  .refresh {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .refresh:not(:disabled):hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .spin {
    animation: spin 1s linear infinite;
  }

  .split {
    flex: 1;
    min-height: 0;
    display: flex;
  }

  .rail {
    width: 232px;
    flex: none;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px 12px;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .rail section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  h3 {
    margin: 0;
    padding: 0 8px 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .opt {
    min-height: 30px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    border-radius: 4px;
    font-size: 12.5px;
    color: var(--text-2);
    text-align: left;
    cursor: pointer;
  }

  .opt:hover {
    background: var(--surface-2);
  }

  .opt[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .opt small {
    margin-left: auto;
    padding-left: 8px;
    font: 500 11px var(--mono);
    color: var(--text-faint);
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    font: 500 11px var(--mono);
    color: var(--text-faint);
  }

  .opt .jira-status {
    height: 20px;
    font-size: 11.5px;
  }

  input[type='checkbox'] {
    flex: none;
    position: relative;
    width: 14px;
    height: 14px;
    margin: 0;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--border-focus);
    appearance: none;
    cursor: pointer;
  }

  input[type='checkbox']:checked {
    background: var(--text);
    box-shadow: none;
  }

  input[type='checkbox']:checked::after {
    content: '';
    position: absolute;
    left: 4.5px;
    top: 1.5px;
    width: 3px;
    height: 7px;
    border: solid var(--ink);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  input[type='checkbox']:focus-visible {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
  }

  .spacer {
    flex: 1;
  }

  .search {
    width: 320px;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-dim);
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font: 12.5px var(--sans);
  }

  kbd {
    padding: 0 5px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 500 10.5px var(--mono);
    color: var(--text-dim);
  }

  .shown {
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .reset {
    font-size: 12px;
    color: var(--text-dim);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .reset:hover {
    color: var(--text);
  }

  .note {
    margin: 0 16px 8px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .note.bad {
    color: var(--err-text);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    margin: 0;
    padding-bottom: 40px;
  }

  .empty strong {
    font-size: 15px;
    font-weight: 600;
  }

  .empty .btn {
    height: 34px;
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 288px;
    justify-content: start;
    gap: 12px;
    padding: 0 16px 16px;
    overflow-x: auto;
  }

  .columns section {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: 8px;
    background: var(--bg-sidebar);
  }

  h4 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 12px 12px 8px;
  }

  .count {
    font: 500 11.5px var(--mono);
    color: var(--text-faint);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0 8px 10px;
    overflow-y: auto;
    list-style: none;
  }

  .ticket {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .ticket:hover {
    background: var(--surface-2);
  }

  .ticket.linked {
    background: none;
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .ticket.linked .name {
    color: var(--text-dim);
  }

  .ticket.open {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .ticket-open {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    text-align: left;
  }

  .meta {
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
  }

  .key {
    font: 600 11.5px var(--mono);
    color: var(--text-3);
    white-space: nowrap;
  }

  .epic {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 5px;
    border-radius: 3px;
    background: rgba(135, 119, 217, 0.18);
    color: #cdb8ff;
    font-size: 10.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .name {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 13px;
    line-height: 1.4;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .assignee {
    margin-left: auto;
    line-height: 0;
  }

  .nobody {
    flex: none;
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--text-faint);
    font-size: 10px;
    line-height: 1;
    color: var(--text-faint);
  }

  .assignee .nobody {
    width: 22px;
    height: 22px;
  }

  .create {
    height: 24px;
    padding: 0 8px;
    border-radius: 4px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .create:hover {
    background: var(--surface-4);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--text-faint);
  }

  .dot.progress {
    background: var(--sub-running);
  }

  .dot.review {
    background: var(--accent);
  }

  .dot.done {
    background: var(--done);
  }
</style>
