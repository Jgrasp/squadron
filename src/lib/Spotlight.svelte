<script lang="ts">
  import { agents, archived, find, show, unarchive, type Archived } from './agents.svelte'
  import { ago } from './format'
  import { t } from './i18n/index.svelte'
  import { projects } from './projects.svelte'
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

  const hits = $derived.by(() => {
    const q = query.trim().toLowerCase()
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
    const q = query.trim()
    if (!q) return [{ text, hit: false }]
    return text.split(new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'i')).map((piece, i) => ({ text: piece, hit: i % 2 === 1 }))
  }

  async function choose(index: number) {
    const hit = hits[index]
    if (!hit) return
    onclose()
    if (hit.archived) await unarchive(hit.agent.id)
    const agent = find(hit.agent.id)
    if (agent) show(agent)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      active = (active + (event.key === 'ArrowDown' ? 1 : hits.length - 1)) % Math.max(1, hits.length)
      dialog.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') {
      event.preventDefault()
      choose(active)
    }
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
    <input bind:this={field} bind:value={query} class="query" placeholder={t('shell.searchPlaceholder')} spellcheck="false" onkeydown={keys} />
    <ul role="listbox" aria-label={t('shell.searchTitle')}>
      {#each hits as hit, i (hit.agent.id)}
        {@const project = projectOf(hit.agent.project)}
        <li role="option" aria-selected={i === active} data-index={i}>
          <button onclick={() => choose(i)} onmousemove={() => (active = i)}>
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
        <li class="none">{t('shell.searchNoResult')}</li>
      {/each}
    </ul>
    <div class="hint">{t('shell.searchHint')}</div>
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
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 9px 12px;
    border-radius: 6px;
    text-align: left;
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
