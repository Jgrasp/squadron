<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Avatar from './Avatar.svelte'
  import { date, errorText, t } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'
  import { getTicket, type Comment, type Ticket } from './tickets.svelte'

  let {
    connection,
    ticketKey,
    linked,
    oncreate,
    onclose,
  }: { connection: string; ticketKey: string; linked: boolean; oncreate: (ticket: Ticket) => void; onclose: () => void } = $props()

  let ticket = $state<Ticket | null>(null)
  let error = $state('')
  let tab = $state<'description' | 'comments' | 'links'>('description')
  let query = $state('')
  let newest = $state(localStorage.getItem('squadron.ticket-sort') !== 'oldest')
  let thread = $state<HTMLElement>()

  $effect(() => {
    const [c, k] = [connection, ticketKey]
    ticket = null
    error = ''
    getTicket(c, k)
      .then((found) => (ticket = found))
      .catch((e) => (error = errorText(e)))
  })

  const day = (iso: string) => (iso ? date(new Date(iso).getTime(), { day: 'numeric', month: 'short', year: 'numeric' }) : '')

  const threads = $derived.by(() => {
    if (!ticket) return []
    const all = ticket.comments
    const byId = new Map(all.map((c) => [c.id, c]))
    const rootOf = (c: Comment): Comment => {
      let at = c
      for (let i = 0; at.parent && byId.has(at.parent) && i < 50; i++) at = byId.get(at.parent)!
      return at
    }
    const roots = all.filter((c) => rootOf(c) === c)
    return roots.map((root) => ({ root, replies: all.filter((c) => c !== root && rootOf(c) === root) }))
  })

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const list = q ? threads.filter((th) => [th.root, ...th.replies].some((c) => `${c.author.name}\n${c.body}`.toLowerCase().includes(q))) : threads
    return newest ? [...list].reverse() : list
  })

  function sort(next: boolean) {
    newest = next
    localStorage.setItem('squadron.ticket-sort', next ? 'newest' : 'oldest')
  }

  $effect(() => {
    const q = query.trim().toLowerCase()
    void shown
    const highlights = (globalThis.CSS as unknown as { highlights?: Map<string, unknown> })?.highlights
    const Highlight = (globalThis as unknown as { Highlight?: new (...r: Range[]) => unknown }).Highlight
    if (!highlights || !Highlight) return
    highlights.delete('ticket-search')
    if (!q || !thread) return
    const ranges: Range[] = []
    requestAnimationFrame(() => {
      if (!thread) return
      const walker = document.createTreeWalker(thread, NodeFilter.SHOW_TEXT)
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const text = node.textContent?.toLowerCase() ?? ''
        for (let at = text.indexOf(q); at >= 0; at = text.indexOf(q, at + q.length)) {
          const range = new Range()
          range.setStart(node, at)
          range.setEnd(node, at + q.length)
          ranges.push(range)
        }
      }
      highlights.set('ticket-search', new Highlight(...ranges))
    })
    return () => highlights.delete('ticket-search')
  })

  const links = $derived(ticket ? (ticket.epic ? 1 : 0) + ticket.subtasks.length + ticket.links.length : 0)
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !document.querySelector('dialog[open]') && onclose()} />

<aside class="panel" aria-label={ticketKey}>
  <header>
    <div class="row">
      <span class="logo" aria-hidden="true">J</span>
      <span class="key">{ticketKey}</span>
      {#if ticket}<span class="faint">· {ticket.kind}{ticket.sprint ? ` · ${ticket.sprint}` : ''}</span>{/if}
      <span class="spacer"></span>
      {#if ticket}<button class="link" onclick={() => invoke('open_link', { url: ticket!.url })}>↗ {t('tickets.openInJira')}</button>{/if}
      <button class="close" aria-label={t('common.close')} onclick={onclose}>✕</button>
    </div>
    {#if ticket}
      <h2>{ticket.title}</h2>
      <dl>
        <dt>{t('tickets.status')}</dt>
        <dd><span class="chip status">{ticket.status}</span></dd>
        <dt>{t('tickets.assignee')}</dt>
        <dd>{ticket.assignee?.name ?? t('tickets.unassigned')}</dd>
        {#if ticket.priority}<dt>{t('tickets.priority')}</dt><dd>{ticket.priority}</dd>{/if}
        {#if ticket.labels.length}<dt>{t('tickets.labels')}</dt><dd>{#each ticket.labels as label (label)}<span class="chip">{label}</span>{/each}</dd>{/if}
        {#each ticket.custom.filter((f) => f.value.length < 40) as field (field.name)}<dt>{field.name}</dt><dd>{field.value}</dd>{/each}
        <dt>{t('tickets.updated')}</dt>
        <dd>{day(ticket.updated)}</dd>
      </dl>
    {/if}
  </header>

  {#if error}
    <p class="bad" role="alert">{error}</p>
  {:else if !ticket}
    <p class="faint pad">{t('common.loading')}</p>
  {:else}
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === 'description'} onclick={() => (tab = 'description')}>{t('tickets.tabDescription')}</button>
      <button role="tab" aria-selected={tab === 'comments'} onclick={() => (tab = 'comments')}>{t('tickets.tabComments', { count: ticket.comments.length })}</button>
      <button role="tab" aria-selected={tab === 'links'} onclick={() => (tab = 'links')}>{t('tickets.tabLinks', { count: links })}</button>
    </div>
    {#if tab === 'comments' && ticket.comments.length}
      <div class="tools">
        <input class="field" type="search" placeholder={t('tickets.searchComments')} aria-label={t('tickets.searchComments')} bind:value={query} />
        <span class="seg" role="group">
          <button aria-pressed={newest} onclick={() => sort(true)}>{t('tickets.newest')}</button>
          <button aria-pressed={!newest} onclick={() => sort(false)}>{t('tickets.oldest')}</button>
        </span>
      </div>
    {/if}
    <div class="body" bind:this={thread}>
      {#if tab === 'description'}
        {#if ticket.description}<Markdown text={ticket.description} />{:else}<p class="faint">{t('tickets.noDescription')}</p>{/if}
        {#if ticket.attachments.length}
          <h4>{t('tickets.attachments')}</h4>
          <p class="files">{#each ticket.attachments as file (file)}<span class="chip">{file}</span>{/each}</p>
        {/if}
      {:else if tab === 'comments'}
        {#if !ticket.comments.length}
          <p class="faint">{t('tickets.noComment')}</p>
        {:else}
          {#if query.trim()}<span class="faint small">{t('tickets.threadsShown', { shown: shown.length, total: threads.length })}</span>{/if}
          {#each shown as { root, replies } (root.id)}
            {@render comment(root)}
            {#if replies.length}
              <div class="replies">{#each replies as reply (reply.id)}{@render comment(reply)}{/each}</div>
            {/if}
          {/each}
        {/if}
      {:else}
        {#if ticket.epic}
          <h4>{t('tickets.parent')}</h4>
          {@render item(ticket.epic.key, ticket.epic.title, ticket.epic.status, ticket.epic.relation)}
        {/if}
        {#if ticket.subtasks.length}
          <h4>{t('tickets.subtasks')}</h4>
          {#each ticket.subtasks as s (s.key)}{@render item(s.key, s.title, s.status, '')}{/each}
        {/if}
        {#if ticket.links.length}
          <h4>{t('tickets.linked')}</h4>
          {#each ticket.links as l, i (i)}{@render item(l.key, l.title, l.status, l.relation)}{/each}
        {/if}
        {#if !links}<p class="faint">{t('tickets.noLink')}</p>{/if}
      {/if}
    </div>
    <footer>
      <span class="faint small">🔒 {t('tickets.readOnly')}</span>
      <span class="spacer"></span>
      {#if !linked}<button class="btn btn-primary" onclick={() => oncreate(ticket!)}>{t('tickets.createTask')}</button>{/if}
    </footer>
  {/if}
</aside>

{#snippet comment(c: Comment)}
  <div class="comment">
    <Avatar name={c.author.name} src={c.author.avatar} size={24} />
    <div class="bubble">
      <div class="who"><strong>{c.author.name}</strong> · {day(c.created)}</div>
      <Markdown text={c.body} />
    </div>
  </div>
{/snippet}

{#snippet item(key: string, title: string, status: string, relation: string)}
  <div class="item">
    {#if relation}<span class="faint">{relation}</span>{/if}
    <span class="key">{key}</span>
    <span class="title">{title}</span>
    {#if status}<span class="chip">{status}</span>{/if}
  </div>
{/snippet}

<style>
  .panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 4;
    width: min(560px, 92%);
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border-strong);
    background: var(--bg-tabs);
    box-shadow: -18px 0 40px rgba(0, 0, 0, 0.35);
  }

  header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 18px 22px 14px;
    border-bottom: 1px solid var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .logo {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    background: var(--jira);
    color: #fff;
    font: 700 9.5px var(--sans);
  }

  .key {
    font: 600 12px var(--mono);
    color: var(--ticket-text);
    white-space: nowrap;
  }

  .faint {
    color: var(--text-faint);
  }

  .small {
    font-size: 12px;
  }

  .pad {
    padding: 16px 22px;
  }

  .link {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .link:hover {
    color: var(--text-strong);
  }

  .close {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .close:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  h2 {
    margin: 0;
    font-size: 17px;
    line-height: 1.35;
  }

  dl {
    display: grid;
    grid-template-columns: auto 1fr auto 1fr;
    gap: 6px 12px;
    margin: 0;
    font-size: 12.5px;
  }

  dt {
    color: var(--text-faint);
  }

  dd {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 0;
    min-width: 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-muted);
    font: 500 11px var(--mono);
    white-space: nowrap;
  }

  .chip.status {
    font: 600 11.5px var(--sans);
    color: var(--text-2);
  }

  .tabs {
    display: flex;
    gap: 18px;
    padding: 0 22px;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: 10px 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-dim);
  }

  .tabs [aria-selected='true'] {
    color: var(--text-strong);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .tools {
    display: flex;
    gap: 8px;
    padding: 12px 22px 0;
  }

  .tools .field {
    flex: 1;
    height: 30px;
  }

  .seg {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    background: var(--bg-sidebar);
  }

  .seg button {
    padding: 0 10px;
    border-radius: 3px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .seg [aria-pressed='true'] {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 22px 18px;
    line-height: 1.55;
  }

  h4 {
    margin: 6px 0 0;
    font-size: 13px;
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
  }

  .comment {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 10px;
  }

  .bubble {
    min-width: 0;
    padding: 9px 12px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .who {
    margin-bottom: 3px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .who strong {
    color: var(--text);
    font-weight: 600;
  }

  .replies {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: -4px 0 0 34px;
    padding-left: 12px;
    border-left: 2px solid var(--border-strong);
  }

  .replies .bubble {
    background: var(--bg-main);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .item .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 22px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 34px;
  }

  .bad {
    margin: 16px 22px;
    color: var(--err-text);
  }

  :global(::highlight(ticket-search)) {
    background: rgba(224, 138, 91, 0.38);
    color: var(--text-strong);
  }
</style>
