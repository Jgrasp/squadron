<script lang="ts">
  import { perform } from './actions.svelte'
  import { agents, archived, find, ofProject, selected, show, unarchive, type Archived } from './agents.svelte'
  import { ago } from './format'
  import { t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'
  import { projects, type Project } from './projects.svelte'
  import { lists, needsYou } from './pulls.svelte'
  import { hint } from './settings.svelte'
  import { match, STARTERS, type Row } from './spotlight'
  import type { Agent } from './stream'

  let { onclose }: { onclose: () => void } = $props()

  type Entry = { agent: Agent | Archived; archived: boolean; text: string; hay: string }

  const textOf = (agent: Agent) =>
    agent.entries
      .map((e) => (e.kind === 'user' ? e.text : e.kind === 'agent' ? e.blocks.map((b) => (b.type === 'text' ? b.text : '')).join('\n') : ''))
      .join('\n')

  const entry = (agent: Agent | Archived, isArchived: boolean): Entry => {
    const text = textOf(agent)
    return { agent, archived: isArchived, text, hay: text.toLowerCase() }
  }

  let pool = $state<Entry[]>(agents.list.map((a) => entry(a, false)))
  archived().then((list) => (pool = [...pool, ...list.map((a) => entry(a, true))]))

  let query = $state('')
  let active = $state(0)
  let dialog: HTMLDialogElement
  let field: HTMLInputElement

  const projectOf = (path: string) => [...projects.list, ...projects.archived].find((p) => p.path === path)

  function snippet(text: string, at: number, length: number) {
    const from = Math.max(0, at - 50)
    const to = Math.min(text.length, at + length + 90)
    return (from ? '…' : '') + text.slice(from, to).replace(/\s+/g, ' ') + (to < text.length ? '…' : '')
  }

  const world = $derived({
    projects: projects.list,
    current: projects.list.find((p) => p.path === projects.current),
    agents: agents.list,
    shown: projects.overview || projects.board || projects.pulls ? undefined : selected(projects.current),
    priority: (p: Project) => needsYou(p.path),
  })
  const rows = $derived(match(query, world))
  const rest = $derived(query.trim().split(/\s+/).slice(1).join(' '))
  const term = $derived(rows.length && rest ? rest : query.trim())

  const hits = $derived.by(() => {
    const q = term.toLowerCase()
    if (!q) return pool.filter((e) => !e.archived).sort((a, b) => b.agent.since - a.agent.since).slice(0, 12).map((e) => ({ ...e, excerpt: '' }))
    const out: (Entry & { excerpt: string; named: boolean })[] = []
    for (const e of pool) {
      const named = `${e.agent.name} ${projectOf(e.agent.project)?.name ?? ''}`.toLowerCase().includes(q)
      const at = q.length > 1 ? e.hay.indexOf(q) : -1
      if (named || at >= 0) out.push({ ...e, named, excerpt: at >= 0 ? snippet(e.text, at, q.length) : '' })
    }
    return out
      .sort((a, b) => Number(b.named) - Number(a.named) || Number(a.archived) - Number(b.archived) || b.agent.since - a.agent.since)
      .slice(0, 50)
  })

  $effect(() => {
    void query
    active = 0
  })

  $effect(() => {
    dialog.showModal()
    field.focus()
  })

  function parts(text: string) {
    const q = term
    if (!q) return [{ text, hit: false }]
    return text.split(new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'i')).map((piece, i) => ({ text: piece, hit: i % 2 === 1 }))
  }

  async function choose(index: number) {
    const row = rows[index]
    if (row) {
      onclose()
      await perform(row)
      return
    }
    const hit = hits[index - rows.length]
    if (!hit) return
    onclose()
    if (hit.archived) await unarchive(hit.agent.id)
    const agent = find(hit.agent.id)
    if (agent) show(agent)
  }

  function keys(event: KeyboardEvent) {
    const size = rows.length + hits.length
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      active = (active + (event.key === 'ArrowDown' ? 1 : size - 1)) % Math.max(1, size)
      dialog.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') {
      event.preventDefault()
      choose(active)
    } else if (event.key === 'Tab' && rows[active]) {
      event.preventDefault()
      query = rows[active].complete
    }
  }

  function start(word: string) {
    query = `${word} `
    field.focus()
  }

  function meta(row: Row): { tone: 'wait' | 'quiet' | 'dim'; text: string } | null {
    const path = row.project?.path
    if (!path) return null
    if (row.verb.id === 'archiveProject') return { tone: 'quiet', text: t('spotlight.agentCount', { count: ofProject(path).length }) }
    if (row.verb.id !== 'pulls') return null
    const found = lists[path]
    if (!found) return null
    if (found.error) return { tone: 'dim', text: t('spotlight.unavailable') }
    const waiting = needsYou(path)
    if (waiting) return { tone: 'wait', text: t('spotlight.toReview', { count: waiting }) }
    const open = found.review.length + found.mine.length
    return open ? { tone: 'quiet', text: t('spotlight.openPulls', { count: open }) } : { tone: 'dim', text: t('spotlight.noPull') }
  }

  function shortcut(row: Row) {
    const action = row.verb.action
    if (!action) return ''
    const here = row.agent ? row.agent.id === world.shown?.id : !row.project || row.project.path === projects.current
    return here ? hint(action) : ''
  }
</script>

<dialog
  bind:this={dialog}
  class="spotlight"
  aria-label={t('shell.searchTitle')}
  oncancel={(e) => (e.preventDefault(), onclose())}
  onclick={(e) => e.target === dialog && onclose()}
>
  <div class="box">
    <input bind:this={field} bind:value={query} class="query" placeholder={t('spotlight.placeholder')} spellcheck="false" onkeydown={keys} />
    {#if !query.trim()}
      <div class="starters">
        <span>{t('spotlight.actions')}</span>
        {#each STARTERS as word (word)}<button type="button" onclick={() => start(word)}>{word}</button>{/each}
      </div>
    {/if}
    <ul role="listbox" aria-label={t('shell.searchTitle')}>
      {#if rows.length}
        <li class="section" role="presentation">{t('spotlight.actions')}</li>
        {#each rows as row, i (row.key)}
          {@const note = meta(row)}
          {@const key = shortcut(row)}
          <li role="option" aria-selected={i === active} data-index={i}>
            <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
              <span class="icon"><Icon name={row.verb.icon} /></span>
              <span class="title">{row.title}</span>
              {#if row.agent}
                <span class="target agent" style:--project={row.project?.color}><i></i>{row.agent.name}</span>
              {:else if row.project}
                <span class="target" style:--project={row.project.color}><i></i>{row.project.name}</span>
              {/if}
              <span class="grow"></span>
              {#if note}<span class="note {note.tone}">{note.text}</span>{/if}
              {#if i === active}<kbd class="enter">↵</kbd>{:else if key}<kbd>{key}</kbd>{/if}
            </button>
          </li>
        {/each}
      {/if}
      {#if hits.length && (rows.length || !query.trim())}
        <li class="section" role="presentation">{rows.length ? t('spotlight.agents') : t('spotlight.recent')}</li>
      {/if}
      {#each hits as hit, j (hit.agent.id)}
        {@const i = rows.length + j}
        {@const project = projectOf(hit.agent.project)}
        <li role="option" aria-selected={i === active} data-index={i}>
          <button class="hit" onclick={() => choose(i)} onmousemove={() => (active = i)}>
            <span class="line" style:--project={project?.color}>
              <span class="square" aria-hidden="true"></span>
              <strong>{#each parts(hit.agent.name) as part, p (p)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}</strong>
              <span class="where">{project?.name ?? ''}</span>
              {#if hit.archived}<span class="tag">{t('shell.searchArchivedTag')}</span>{/if}
              <span class="when">{ago(hit.agent.since, Date.now())}</span>
            </span>
            {#if hit.excerpt}
              <span class="excerpt">{#each parts(hit.excerpt) as part, p (p)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}</span>
            {/if}
          </button>
        </li>
      {:else}
        {#if !rows.length}<li class="none">{t('shell.searchNoResult')}</li>{/if}
      {/each}
    </ul>
    <div class="hint">{active < rows.length ? t('spotlight.hintActions') : t('shell.searchHint')}</div>
  </div>
</dialog>

<style>
  .spotlight {
    width: 680px;
    max-width: calc(100vw - 48px);
    margin: 14vh auto auto;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: var(--bg-main);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
  }

  .spotlight::backdrop {
    background: rgba(0, 0, 0, 0.45);
  }

  .box {
    display: flex;
    flex-direction: column;
  }

  .query {
    height: 52px;
    padding: 0 18px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-strong);
    font-size: 16px;
    outline: none;
  }

  ul {
    max-height: 58vh;
    overflow-y: auto;
    margin: 0;
    padding: 6px;
    list-style: none;
  }

  li button {
    width: 100%;
    padding: 9px 12px;
    border-radius: 6px;
    text-align: left;
  }

  .hit {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .action {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
  }

  .starters {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-dim);
  }

  .starters span {
    margin-right: 4px;
  }

  .starters button {
    height: 22px;
    padding: 0 8px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 500 12px var(--mono);
    color: var(--text-2);
  }

  .section {
    padding: 8px 12px 3px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-muted);
  }

  li[aria-selected='true'] .icon {
    background: rgba(224, 138, 91, 0.16);
    box-shadow: inset 0 0 0 1px rgba(224, 138, 91, 0.4);
    color: var(--accent-soft);
  }

  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .target {
    flex: none;
    max-width: 240px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 60%, transparent);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .target i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .target.agent i {
    border-radius: 50%;
  }

  .grow {
    flex: 1;
  }

  .note {
    flex: none;
    font-size: 12px;
    color: var(--text-dim);
  }

  .note.wait,
  .note.quiet {
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11.5px;
  }

  .note.wait {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    font-weight: 700;
    color: var(--accent-soft);
  }

  .note.quiet {
    background: var(--surface-3);
    color: var(--text-muted);
  }

  kbd {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 500 11px var(--mono);
    color: var(--text-dim);
  }

  kbd.enter {
    background: var(--accent);
    box-shadow: none;
    color: var(--accent-ink);
  }

  li[aria-selected='true'] button {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .square {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project, var(--text-faint));
  }

  strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .where,
  .when {
    flex: none;
    font-size: 12px;
    color: var(--text-dim);
  }

  .when {
    margin-left: auto;
  }

  .tag {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 11px;
    color: var(--text-muted);
  }

  .excerpt {
    padding-left: 17px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  mark {
    border-radius: 2px;
    background: rgba(224, 138, 91, 0.35);
    color: var(--text-strong);
  }

  .none {
    padding: 14px 12px;
    color: var(--text-dim);
  }

  .hint {
    padding: 8px 18px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--text-dim);
  }
</style>
