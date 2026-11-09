<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Avatar from './Avatar.svelte'
  import Dialog from './Dialog.svelte'
  import { date, errorText, number, t } from './i18n/index.svelte'
  import { URGENCY } from './jira'
  import { requestPanel } from './layout.svelte'
  import PanelButtons from './PanelButtons.svelte'
  import SidePanel from './SidePanel.svelte'
  import Markdown from './Markdown.svelte'
  import { getTicket, type Comment, type Ticket, type TicketFile } from './tickets.svelte'

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
  let whole = $state(false)
  let previews = $state<Record<string, string>>({})
  let fileError = $state('')
  let asked = new Set<string>()

  requestPanel('ticket')

  $effect(() => {
    const [c, k] = [connection, ticketKey]
    ticket = null
    error = ''
    previews = {}
    asked = new Set()
    getTicket(c, k)
      .then((found) => (ticket = found))
      .catch((e) => (error = errorText(e)))
  })

  const fetchFile = (file: TicketFile) => invoke<string>('ticket_attachment', { connection, id: file.id })

  async function openFile(file: TicketFile) {
    fileError = ''
    try {
      await invoke('open_attachment', { path: await fetchFile(file) })
    } catch (e) {
      fileError = errorText(e)
    }
  }

  $effect(() => {
    if (!whole || !ticket) return
    for (const file of ticket.attachments) {
      if (!file.mime.startsWith('image/') || file.size > 20 * 1024 ** 2 || asked.has(file.id)) continue
      asked.add(file.id)
      fetchFile(file)
        .then((path) => invoke<string>('attachment_preview', { path }))
        .then((url) => (previews[file.id] = url))
        .catch(() => {})
    }
  })

  const size = (bytes: number) =>
    bytes < 1024 ** 2
      ? number(Math.max(1, Math.round(bytes / 1024)), { style: 'unit', unit: 'kilobyte', unitDisplay: 'short' })
      : number(bytes / 1024 ** 2, { style: 'unit', unit: 'megabyte', unitDisplay: 'short', maximumFractionDigits: 1 })

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

<svelte:window onkeydown={(e) => e.key === 'Escape' && !document.querySelector('dialog[open]') && (e.preventDefault(), onclose())} />

<SidePanel kind="ticket" title={ticketKey} {onclose}>
  <header>
    <div class="row">
      <span class="logo" aria-hidden="true">J</span>
      <span class="key">{ticketKey}</span>
      {#if ticket}<span class="faint meta">· {ticket.kind}{ticket.sprint ? ` · ${ticket.sprint}` : ''}</span>{/if}
      <span class="spacer"></span>
      {#if ticket}
        <button class="open-jira" title={t('tickets.viewAllHint')} aria-label={t('tickets.viewAll')} onclick={() => (whole = true)}>
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 14v6h6M20 10V4h-6M4 20l7-7M20 4l-7 7" /></svg><span class="label">{t('tickets.viewAll')}</span>
        </button>
        <button class="open-jira" title={ticket.url} aria-label={t('tickets.openInJira')} onclick={() => invoke('open_link', { url: ticket!.url })}>
          <span class="logo" aria-hidden="true">J</span><span class="label">{t('tickets.openInJira')}</span>
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></svg>
        </button>
      {/if}
      <PanelButtons kind="ticket" {onclose} />
    </div>
    {#if ticket}
      <h2>{ticket.title}</h2>
      <dl>{@render fields(false)}</dl>
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
          <p class="files">{#each ticket.attachments as file (file.id)}<button class="chip" title={size(file.size)} onclick={() => openFile(file)}>{file.name}</button>{/each}</p>
          {#if fileError}<p class="bad" role="alert">{fileError}</p>{/if}
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
        {@render related()}
      {/if}
    </div>
    <footer>
      <span class="faint small">🔒 {t('tickets.readOnly')}</span>
      <span class="spacer"></span>
      {#if !linked}<button class="btn btn-primary" onclick={() => oncreate(ticket!)}>{t('tickets.createTask')}</button>{/if}
    </footer>
  {/if}
</SidePanel>

{#if whole && ticket}
  {@const all = ticket}
  <Dialog title="{ticketKey} · {all.title}" full onclose={() => (whole = false)}>
    <div class="whole">
      <div class="main">
        <section>
          <h4>{t('tickets.tabDescription')}</h4>
          {#if all.description}<Markdown text={all.description} />{:else}<p class="faint">{t('tickets.noDescription')}</p>{/if}
        </section>
        {#if all.attachments.length}
          <section>
            <h4>{t('tickets.attachments')} · {all.attachments.length}</h4>
            {#if fileError}<p class="bad" role="alert">{fileError}</p>{/if}
            <div class="grid">
              {#each all.attachments as file (file.id)}
                <button class="file" title={file.name} onclick={() => openFile(file)}>
                  <span class="thumb">
                    {#if previews[file.id]}<img src={previews[file.id]} alt="" />{:else}<span class="ext">{file.name.split('.').pop()}</span>{/if}
                  </span>
                  <span class="name">{file.name}</span>
                  <span class="faint small">{size(file.size)}{file.author ? ` · ${file.author}` : ''} · {day(file.created)}</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}
        <section>
          <h4>{t('tickets.tabComments', { count: all.comments.length })}</h4>
          {#if !all.comments.length}<p class="faint">{t('tickets.noComment')}</p>{/if}
          {#each threads as { root, replies } (root.id)}
            {@render comment(root)}
            {#if replies.length}
              <div class="replies">{#each replies as reply (reply.id)}{@render comment(reply)}{/each}</div>
            {/if}
          {/each}
        </section>
      </div>
      <aside>
        <dl>{@render fields(true)}</dl>
        <section>
          <h4>{t('tickets.tabLinks', { count: links })}</h4>
          {@render related()}
        </section>
        <button class="open-jira" title={all.url} onclick={() => invoke('open_link', { url: all.url })}>
          <span class="logo" aria-hidden="true">J</span>{t('tickets.openInJira')}
        </button>
      </aside>
    </div>
  </Dialog>
{/if}

{#snippet fields(every: boolean)}
  {#if ticket}
    <dt>{t('tickets.status')}</dt>
    <dd><span class="jira-status {ticket.category}">{ticket.status}</span></dd>
    <dt>{t('tickets.assignee')}</dt>
    <dd>
      {#if ticket.assignee}<Avatar name={ticket.assignee.name} src={ticket.assignee.avatar} size={20} />{ticket.assignee.name}{:else}<span class="nobody" aria-hidden="true">?</span><span class="faint">{t('tickets.unassigned')}</span>{/if}
    </dd>
    {#if ticket.priority}<dt>{t('tickets.priority')}</dt><dd>{#if ticket.urgency}<span class="jira-urgency u{ticket.urgency}" aria-hidden="true">{URGENCY[ticket.urgency]}</span>{/if}{ticket.priority}</dd>{/if}
    {#if ticket.epic}<dt>{t('tickets.epic')}</dt><dd><span class="epic" title={ticket.epic.key}>{ticket.epic.title}</span></dd>{/if}
    {#if ticket.labels.length}<dt>{t('tickets.labels')}</dt><dd>{#each ticket.labels as label (label)}<span class="chip">{label}</span>{/each}</dd>{/if}
    {#if every}<dt>{t('tickets.kind')}</dt><dd>{ticket.kind}</dd>{/if}
    {#if every && ticket.sprint}<dt>{t('tickets.sprint')}</dt><dd>{ticket.sprint}</dd>{/if}
    {#if every && ticket.reporter}<dt>{t('tickets.reporter')}</dt><dd><Avatar name={ticket.reporter.name} src={ticket.reporter.avatar} size={20} />{ticket.reporter.name}</dd>{/if}
    {#if every && ticket.components.length}<dt>{t('tickets.fieldComponents')}</dt><dd>{#each ticket.components as c (c)}<span class="chip">{c}</span>{/each}</dd>{/if}
    {#if every && ticket.versions.length}<dt>{t('tickets.fieldVersions')}</dt><dd>{#each ticket.versions as v (v)}<span class="chip">{v}</span>{/each}</dd>{/if}
    {#each ticket.custom.filter((f) => every || f.value.length < 40) as field (field.name)}<dt>{field.name}</dt><dd>{field.value}</dd>{/each}
    <dt>{t('tickets.updated')}</dt>
    <dd>{day(ticket.updated)}</dd>
  {/if}
{/snippet}

{#snippet related()}
  {#if ticket}
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
{/snippet}

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
  header {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 18px 22px 14px;
    border-bottom: 1px solid var(--border);
    container-type: inline-size;
  }

  .meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .open-jira {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 10px 0 6px;
    border: 1px solid var(--ticket-border);
    border-radius: 6px;
    background: var(--ticket-bg);
    color: var(--ticket-text);
    flex: none;
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  @container (max-width: 560px) {
    .open-jira .label {
      display: none;
    }

    .open-jira {
      padding: 0 7px;
    }
  }

  .open-jira:hover {
    border-color: var(--ticket-border-hover);
    background: var(--ticket-card-hover);
  }

  .open-jira .logo {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    font-size: 10px;
  }

  .open-jira svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .open-jira svg {
    opacity: 0.7;
  }

  .nobody {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--text-faint);
    border-radius: 50%;
    font-size: 10px;
    color: var(--text-faint);
  }

  .jira-urgency {
    width: 20px;
    text-align: center;
    font-size: 13px;
  }

  .epic {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 1px 8px;
    border-radius: 4px;
    background: rgba(135, 119, 217, 0.18);
    color: #cdb8ff;
    font-size: 12px;
    font-weight: 600;
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
    align-items: center;
    gap: 8px 12px;
    margin: 0;
    font-size: 12.5px;
  }

  dt {
    color: var(--text-faint);
  }

  dd {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 8px;
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

  button.chip:hover {
    color: var(--text);
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

  .body .bad,
  .whole .bad {
    margin: 0;
  }

  .whole {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    border-top: 1px solid var(--border);
  }

  .whole .main,
  .whole aside {
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 22px;
    padding: 20px 24px 28px;
    line-height: 1.55;
  }

  .whole aside {
    align-items: flex-start;
    border-left: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .whole section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    align-self: stretch;
  }

  .whole dl {
    grid-template-columns: auto 1fr;
    align-self: stretch;
  }

  .whole dd {
    overflow-wrap: anywhere;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 12px;
  }

  .file {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-1);
    text-align: left;
  }

  .file:hover {
    border-color: var(--border-strong);
  }

  .thumb {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 110px;
    margin-bottom: 4px;
    overflow: hidden;
    border-radius: 4px;
    background: var(--bg-main);
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .ext {
    color: var(--text-dim);
    font: 600 12px var(--mono);
    text-transform: uppercase;
  }

  .file .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  :global(::highlight(ticket-search)) {
    background: rgba(224, 138, 91, 0.38);
    color: var(--text-strong);
  }
</style>
