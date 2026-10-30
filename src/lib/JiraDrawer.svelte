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
  import { fly } from 'svelte/transition'
  import Avatar from './Avatar.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { FACETS, filterTickets, inSource, kindTone, sourceId, statusColumns, URGENCY, values, type Facet } from './jira'
  import type { Project } from './projects.svelte'
  import { loadSprint, sameTicket, tickets, type Summary } from './tickets.svelte'

  let {
    project,
    linked,
    viewing,
    onview,
    oncreate,
    onclose,
  }: {
    project: Project
    linked: (ticket: Summary) => boolean
    viewing: { connection: string; key: string } | null
    onview: (ticket: Summary) => void
    oncreate: (ticket: Summary) => void
    onclose: () => void
  } = $props()

  const FACET_LABELS: Record<Facet, Key> = { status: 'tickets.status', kind: 'tickets.kind', priority: 'tickets.priority', epic: 'tickets.epic' }

  const path = untrack(() => project.path)
  let f = $state(load(path))
  let menu = $state<Facet | null>(null)
  let search = $state<HTMLInputElement>()

  $effect(() => keep(path, $state.snapshot(f)))

  const sprint = $derived(tickets.sprints[project.path])
  const all = $derived(sprint?.tickets ?? [])
  const source = $derived(project.tickets.find((s) => sourceId(s) === f.source))
  const scope = $derived(source ? all.filter((x) => inSource(x, source)) : all)
  const shown = $derived(filterTickets(all, f, project.tickets, linked))
  const columns = $derived(statusColumns(shown))
  const people = $derived(
    [...new Map(scope.filter((x) => x.assignee).map((x) => [x.assignee!.name, x.assignee!])).values()].sort((a, b) => a.name.localeCompare(b.name)),
  )
  const filtered = $derived(f.mine || f.free || !!f.people.length || !!f.query.trim() || FACETS.some((facet) => f.hidden[facet].length))

  const count = (s: (typeof project.tickets)[number]) => all.filter((x) => inSource(x, s) && x.category !== 'done').length
  const urgencyOf = (priority: string) => all.find((x) => x.priority === priority)?.urgency ?? 0
  const valueLabel = (facet: Facet, value: string) => value || (facet === 'epic' ? t('tickets.noEpic') : facet === 'priority' ? t('tickets.noPriority') : '—')

  function toggle(list: string[], value: string) {
    return list.includes(value) ? list.filter((v) => v !== value) : [...list, value]
  }

  function reset() {
    f = { ...emptyFilters(), source: f.source }
  }

  function keys(event: KeyboardEvent) {
    const target = event.target as HTMLElement
    if (event.key === 'Escape' && menu) {
      event.preventDefault()
      menu = null
    } else if (event.key === '/' && !target.closest('input, textarea, [contenteditable]') && !document.querySelector('dialog[open]')) {
      event.preventDefault()
      search?.focus()
    }
  }
</script>

<svelte:window onkeydown={keys} onclick={(e) => !(e.target as HTMLElement).closest('[data-facet]') && (menu = null)} />

<div class="drawer" role="region" aria-label={t('tickets.jira')} transition:fly={{ y: -12, duration: 160 }}>
  <div class="head">
    <span class="logo" aria-hidden="true">J</span>
    {#if project.tickets.length > 1}
      <div class="sources" role="tablist">
        <button role="tab" aria-selected={!source} onclick={() => (f.source = '')}>{t('tickets.allSources')}</button>
        {#each project.tickets as s (sourceId(s))}
          <button role="tab" aria-selected={source === s} onclick={() => (f.source = sourceId(s))}>
            {s.name || s.key} <small>{s.key}</small> <span class="n">{count(s)}</span>
          </button>
        {/each}
      </div>
    {:else}
      <strong class="title">{project.tickets[0]?.name || project.tickets[0]?.key}</strong>
    {/if}
    <span class="spacer"></span>
    {#if filtered}<span class="shown">{t('tickets.ticketsShown', { shown: shown.length, total: scope.length })}</span>{/if}
    <button class="sprint" title={t('tickets.refresh')} aria-label={t('tickets.refresh')} disabled={sprint?.loading} onclick={() => loadSprint(project, true)}>
      {sprint?.sprints.join(' · ') ?? ''} <span class:spin={sprint?.loading} aria-hidden="true">↻</span>
    </button>
    <button class="icon" aria-label={t('common.close')} title={t('common.close')} onclick={onclose}>
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
    </button>
  </div>

  <div class="filters">
    <label class="search">
      <svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-4-4" /></svg>
      <input bind:this={search} bind:value={f.query} placeholder={t('tickets.filterTickets')} aria-label={t('tickets.filterTickets')} />
      <kbd>/</kbd>
    </label>
    <button class="chip" aria-pressed={f.mine} onclick={() => (f.mine = !f.mine)}>{t('tickets.mine')}</button>
    <span class="people">
      {#each people as p (p.name)}
        <button class="who" aria-pressed={f.people.includes(p.name)} title={p.name} aria-label={p.name} onclick={() => (f.people = toggle(f.people, p.name))}>
          <Avatar name={p.name} src={p.avatar} size={24} />
        </button>
      {/each}
      {#if scope.some((x) => !x.assignee)}
        <button class="who" aria-pressed={f.people.includes('')} title={t('tickets.unassigned')} aria-label={t('tickets.unassigned')} onclick={() => (f.people = toggle(f.people, ''))}>
          <span class="nobody" aria-hidden="true">?</span>
        </button>
      {/if}
    </span>
    <button class="chip" aria-pressed={f.free} onclick={() => (f.free = !f.free)}>{t('tickets.withoutTask')}</button>
    {#each FACETS as facet (facet)}
      {@const options = values(scope, facet)}
      <span class="facet" data-facet>
        <button class="chip drop" class:on={f.hidden[facet].length > 0} aria-haspopup="true" aria-expanded={menu === facet} onclick={() => (menu = menu === facet ? null : facet)}>
          {t(FACET_LABELS[facet])}
          <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
        </button>
        {#if menu === facet}
          <div class="menu">
            {#each options as value (value)}
              <label>
                <input type="checkbox" checked={!f.hidden[facet].includes(value)} onchange={() => (f.hidden[facet] = toggle(f.hidden[facet], value))} />
                {#if facet === 'kind'}<span class="jira-kind {kindTone(value)}" aria-hidden="true">{value[0]?.toUpperCase()}</span>{/if}
                {#if facet === 'priority' && urgencyOf(value)}<span class="jira-urgency u{urgencyOf(value)}" aria-hidden="true">{URGENCY[urgencyOf(value)]}</span>{/if}
                {valueLabel(facet, value)}
                <small>{scope.filter((x) => x[facet] === value).length}</small>
              </label>
            {/each}
          </div>
        {/if}
      </span>
    {/each}
    {#if filtered}<button class="reset" onclick={reset}>{t('tickets.resetFilters')}</button>{/if}
  </div>

  <div class="columns">
    {#each sprint?.errors ?? [] as error, i (i)}<p class="note bad" role="alert">{errorText(error)}</p>{/each}
    {#if sprint && !sprint.loading && !all.length && !sprint.errors.length}
      <p class="note">{t('tickets.noSprint')}</p>
    {:else if all.length && !shown.length}
      <p class="note">{t('tickets.noMatch')}</p>
    {/if}
    {#each columns as column (column.status)}
      <section aria-label={column.status}>
        <h3><span class="jira-status {column.category}">{column.status}</span><span class="count">{column.tickets.length}</span></h3>
        <ul>
          {#each column.tickets as ticket (`${ticket.connection}:${ticket.key}`)}
            {@const done = linked(ticket)}
            <li class="ticket" class:linked={done} class:open={sameTicket(viewing, ticket)}>
              <button class="ticket-open" title={ticket.title} onclick={() => onview(ticket)}>
                <span class="meta">
                  <span class="jira-kind {kindTone(ticket.kind)}" title={ticket.kind} aria-hidden="true">{ticket.kind[0]?.toUpperCase()}</span>
                  <span class="key">{ticket.key}</span>
                  {#if ticket.urgency}<span class="jira-urgency u{ticket.urgency}" title={ticket.priority} aria-hidden="true">{URGENCY[ticket.urgency]}</span>{/if}
                  {#if ticket.epic}<span class="epic">{ticket.epic}</span>{/if}
                </span>
                <span class="name">{ticket.title}</span>
              </button>
              <span class="assignee" title={ticket.assignee?.name ?? t('tickets.unassigned')}>
                {#if ticket.assignee}<Avatar name={ticket.assignee.name} src={ticket.assignee.avatar} size={22} />{:else}<span class="nobody" aria-hidden="true">?</span>{/if}
              </span>
              {#if !done}<button class="create" onclick={() => oncreate(ticket)}>+ {t('tickets.createTask')}</button>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
</div>

<style>
  .drawer {
    position: absolute;
    top: 0;
    left: 20px;
    right: 20px;
    z-index: 3;
    height: 62%;
    min-height: 340px;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--ticket-border-hover);
    border-radius: 10px;
    background: var(--ticket-bg);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.55);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 6px 14px;
  }

  .logo {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: var(--jira);
    color: #fff;
    font: 700 10px var(--sans);
  }

  .title {
    font-size: 13px;
    color: var(--ticket-text);
  }

  .sources {
    display: flex;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
  }

  .sources button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 5px;
    color: var(--ticket-muted);
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .sources button:hover {
    color: var(--ticket-text);
  }

  .sources [aria-selected='true'] {
    background: var(--ticket-button);
    color: var(--ticket-text);
  }

  .sources small {
    font: 500 10.5px var(--mono);
    opacity: 0.8;
  }

  .n {
    font-size: 11px;
    font-weight: 500;
    color: var(--ticket-muted);
  }

  .spacer {
    flex: 1;
  }

  .shown {
    font-size: 12px;
    color: var(--ticket-muted);
    white-space: nowrap;
  }

  .sprint {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 3px 7px;
    border-radius: 4px;
    font-size: 12px;
    color: var(--ticket-muted);
  }

  .sprint:not(:disabled):hover,
  .icon:hover {
    background: var(--ticket-button);
    color: var(--ticket-text);
  }

  .spin {
    display: inline-block;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .icon {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: var(--ticket-muted);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 0 12px 10px;
  }

  .search {
    width: 240px;
    height: 28px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border: 1px solid var(--ticket-border);
    border-radius: 5px;
    background: var(--ticket-card);
    color: var(--ticket-muted);
  }

  .search:focus-within {
    border-color: var(--ticket-border-hover);
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
    padding: 0 4px;
    border: 1px solid var(--ticket-border);
    border-radius: 3px;
    font: 500 10px var(--mono);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 10px;
    border-radius: 999px;
    background: var(--ticket-button);
    color: var(--ticket-muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .chip:hover {
    color: var(--ticket-text);
  }

  .chip[aria-pressed='true'] {
    background: var(--remote-dot);
    color: #0e1a27;
  }

  .chip.on {
    background: var(--ticket-card-hover);
    color: var(--ticket-text);
    box-shadow: inset 0 0 0 1px var(--remote-dot);
  }

  .people {
    display: inline-flex;
    align-items: center;
  }

  .who {
    margin-left: -5px;
    padding: 0;
    border: 2px solid var(--ticket-bg);
    border-radius: 50%;
    line-height: 0;
  }

  .who:first-child {
    margin-left: 0;
  }

  .who:hover {
    position: relative;
    z-index: 1;
  }

  .who[aria-pressed='true'] {
    position: relative;
    z-index: 2;
    border-color: var(--remote-dot);
  }

  .nobody {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--text-faint);
    border-radius: 50%;
    background: var(--ticket-bg);
    font-size: 10px;
    line-height: 1;
    color: var(--text-faint);
  }

  .facet {
    position: relative;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 5;
    min-width: 190px;
    max-height: 300px;
    overflow-y: auto;
    padding: 4px;
    border: 1px solid var(--ticket-border-hover);
    border-radius: 6px;
    background: #1a2633;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  }

  .menu label {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 4px;
    font-size: 12.5px;
    color: var(--ticket-text);
    white-space: nowrap;
  }

  .menu label:hover {
    background: var(--ticket-button);
  }

  .menu small {
    margin-left: auto;
    padding-left: 12px;
    color: var(--ticket-muted);
  }

  .reset {
    margin-left: 4px;
    font-size: 12px;
    color: var(--ticket-muted);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .reset:hover {
    color: var(--ticket-text);
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(230px, 1fr);
    gap: 10px;
    padding: 0 10px 10px;
    overflow-x: auto;
  }

  .note {
    grid-column: 1 / -1;
    margin: 4px 4px 0;
    font-size: 12.5px;
    color: var(--ticket-muted);
  }

  .note.bad {
    color: var(--err-text);
  }

  section {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.18);
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 8px 6px;
  }

  .count {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--ticket-muted);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
    padding: 0 6px 6px;
    overflow-y: auto;
    list-style: none;
  }

  .ticket {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    column-gap: 8px;
    border: 1px solid var(--ticket-border);
    border-radius: 5px;
    background: var(--ticket-card);
  }

  .ticket:hover {
    border-color: var(--ticket-border-hover);
    background: var(--ticket-card-hover);
  }

  .ticket.open {
    border-color: var(--remote-dot);
    box-shadow: 0 0 0 1px var(--remote-dot);
  }

  .ticket.linked:not(:hover):not(.open) {
    opacity: 0.5;
  }

  .ticket-open {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 7px 0 7px 10px;
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
    font: 600 11px var(--mono);
    color: var(--ticket-text);
    white-space: nowrap;
  }

  .epic {
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
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .assignee {
    padding-right: 8px;
    line-height: 0;
  }

  .create {
    grid-column: 1 / -1;
    justify-self: start;
    margin: 0 8px 8px 10px;
    padding: 3px 9px;
    border-radius: 4px;
    background: var(--ticket-button);
    color: var(--ticket-text);
    font-size: 12px;
    font-weight: 600;
  }

  .create:hover {
    background: var(--ticket-border-hover);
  }
</style>
