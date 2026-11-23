<script lang="ts" module>
  import { judgementOf, type Remark } from './pulls.svelte'

  export type Thread = { root: Remark; all: Remark[] }
  export type ThreadState = 'open' | 'answered' | 'fixed' | 'resolved' | 'outdated'

  export function stateOf(key: string, th: Thread): ThreadState {
    if (th.root.resolved) return 'resolved'
    if (th.root.path && th.root.line === null) return 'outdated'
    const judged = judgementOf(key, th.root.id)
    if (judged?.state === 'fixed') return 'fixed'
    if (judged?.state === 'answered') return 'answered'
    if (!judged && th.all.length > 1 && th.all.at(-1)!.author !== th.root.author) return 'answered'
    return 'open'
  }
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { agents } from './agents.svelte'
  import { can } from './forges.svelte'
  import { hostName, rounds, type Pull } from './pulls.svelte'
  import type { Comment } from './stream'
  import { isUnread, markSeen } from './unread.svelte'
  import { date, errorText, t, type Key } from './i18n/index.svelte'

  let {
    pull,
    folder,
    reviewKey,
    threads,
    self,
    drafts,
    onfresh,
    ongo,
    ondraft,
    ondrop,
    onedit,
  }: {
    pull: Pull
    folder: string
    reviewKey: string
    threads: Thread[]
    self: string
    drafts: Comment[]
    onfresh: () => void
    ongo: (thread: Thread) => void
    ondraft: (c: Comment) => void
    ondrop: (c: Comment) => void
    onedit: (c: Comment, note: string) => void
  } = $props()

  let editing = $state<Comment | null>(null)
  let text = $state('')

  function save(event: Event) {
    event.preventDefault()
    if (!editing || !text.trim()) return
    onedit(editing, text.trim())
    editing = null
  }


  const STATES: [ThreadState, Key][] = [
    ['open', 'review.threadOpen'],
    ['answered', 'review.threadAnswered'],
    ['fixed', 'review.threadFixed'],
    ['resolved', 'review.threadResolved'],
    ['outdated', 'review.threadOutdated'],
  ]
  const label = (state: ThreadState) => t(STATES.find(([s]) => s === state)![1])

  const states = $derived(new Map(threads.map((th) => [th.root.id, stateOf(reviewKey, th)])))
  const order: ThreadState[] = ['open', 'answered', 'fixed']
  const needs = $derived(threads.filter((th) => order.includes(states.get(th.root.id)!)).sort((a, b) => order.indexOf(states.get(a.root.id)!) - order.indexOf(states.get(b.root.id)!)))
  const closed = $derived(threads.filter((th) => !order.includes(states.get(th.root.id)!)))
  const counts = $derived(STATES.map(([state]) => [state, threads.filter((th) => states.get(th.root.id) === state).length] as const).filter(([, n]) => n))

  const history = $derived(rounds[reviewKey] ?? [])
  const same = (a: string, b: string) => !!a && !!b && (a.startsWith(b) || b.startsWith(a))
  const pushed = $derived(!!history.length && !same(history.at(-1)!.head, pull.head))
  const short = (sha: string) => sha.slice(0, 7)
  const day = (at: number | string) => date(new Date(at), { day: 'numeric', month: 'short' })
  const when = (at: string) => date(new Date(at), { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
  const byName = (id: string) => (id ? (agents.list.find((a) => a.id === id)?.name ?? t('review.agent')) : t('common.you'))
  const VERDICT: Record<string, [string, Key]> = {
    approve: ['ok', 'review.approved'],
    request_changes: ['err', 'review.changesRequested'],
    '': ['plain', 'review.commented'],
  }
  const where = (c: { path: string | null; line: number | null }) => (c.path ? `${c.path}${c.line !== null ? `:${c.line}` : ''}` : t('review.pullRequest'))

  let replying = $state<string | null>(null)
  let answer = $state('')
  let busy = $state<string | null>(null)
  let failure = $state<{ id: string; text: string } | null>(null)

  async function reply(th: Thread, event: Event) {
    event.preventDefault()
    if (!answer.trim()) return
    busy = th.root.id
    failure = null
    try {
      await invoke('pull_reply', { dir: folder, number: pull.number, to: th.root.id, kind: th.root.kind, body: answer })
      replying = null
      answer = ''
      onfresh()
    } catch (e) {
      failure = { id: th.root.id, text: String(e) }
    } finally {
      busy = null
    }
  }

  async function resolve(th: Thread) {
    busy = th.root.id
    failure = null
    try {
      await invoke('pull_resolve', { dir: folder, number: pull.number, to: th.root.id, kind: th.root.kind, resolved: !th.root.resolved })
      onfresh()
    } catch (e) {
      failure = { id: th.root.id, text: String(e) }
    } finally {
      busy = null
    }
  }

  let listed = $state<string[]>([])
  $effect(() => {
    const ids = threads.flatMap((th) => th.all.filter((r) => isUnread(reviewKey, r, self)).map((r) => r.id))
    untrack(() => {
      const added = ids.filter((id) => !listed.includes(id))
      if (added.length) listed = [...listed, ...added]
    })
  })
  const feed = $derived(
    threads
      .flatMap((th) => th.all.filter((r) => listed.includes(r.id)).map((r) => ({ r, th })))
      .sort((a, b) => b.r.at.localeCompare(a.r.at)),
  )
  const unread = $derived(feed.filter(({ r }) => isUnread(reviewKey, r, self)).length)
  const did = (r: Remark, th: Thread): Key =>
    r === th.root ? (r.path ? 'review.remarkOpened' : 'review.remarkCommented') : th.root.author === self ? 'review.remarkAnsweredYou' : 'review.remarkReplied'

  let picked = $state('')
  function go(th: Thread, ids = th.all.map((r) => r.id)) {
    picked = th.root.id
    markSeen(reviewKey, ids)
    ongo(th)
  }
  function goFrom(th: Thread, event: MouseEvent) {
    if ((event.target as HTMLElement).closest('button, a, textarea, form') || getSelection()?.toString()) return
    go(th)
  }

  const resolvable = (th: Thread) => th.root.resolved !== undefined && th.root.resolved !== null && can(pull.host, 'resolve')
</script>

<div class="rounds">
  <ol class="timeline" aria-label={t('review.rounds')}>
    {#each history as round (round.n)}
      {@const [tone, key] = VERDICT[round.verdict ?? ''] ?? VERDICT['']}
      <li
        class="{round.posted ? tone : ''}"
        class:now={!round.posted && !pushed}
        title={[short(round.head), day(round.posted ?? round.started), byName(round.by), round.comments ? t('review.commentCount', { count: round.comments }) : '', round.posted ? t(key) : t('review.roundInProgress')].filter(Boolean).join(' · ')}
      >
        {#if round.posted}<Icon name="check" size={10} />{/if}{t('review.roundN', { n: round.n })}
      </li>
    {/each}
    {#if pushed}
      <li class="push" title={[short(pull.head), pull.head_at ? day(pull.head_at) : ''].filter(Boolean).join(' · ')}>
        <Icon name="pull" size={10} />{t('review.newCommitsPushed', { author: pull.author })}
      </li>
    {/if}
    {#if !history.length}
      <li class="empty">{t('review.noRoundYet')}</li>
    {/if}
  </ol>

  <div class="body">
    {#if unread}
      <h3 class="caps">{t('review.newRemarks')} <span class="n wait">{unread}</span><button class="link" onclick={() => markSeen(reviewKey, listed)}>{t('review.markAllRead')}</button></h3>
      <div class="feed">
        {#each feed as { r, th } (r.id)}
          <button class="item" class:seen={!isUnread(reviewKey, r, self)} class:on={picked === th.root.id} onclick={() => go(th, [r.id])}>
            <span class="dot" aria-hidden="true"></span>
            <Avatar name={r.author} src={r.avatar} size={20} />
            <span class="itxt">
              <span class="what"><strong>{r.author}</strong> {t(did(r, th))} · {when(r.at)}</span>
              <span class="said">{r.body.split('\n')[0]}</span>
              <span class={th.root.path ? 'path' : 'general'}>{where(th.root)}</span>
            </span>
            <svg class="chev" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        {/each}
      </div>
    {/if}

    {#if counts.length}
      <div class="counts" aria-label={t('review.threadStates')}>
        {#each counts as [state, n] (state)}
          <span class="count"><span class="tone {state}" aria-hidden="true"></span>{n} {label(state).toLowerCase()}</span>
        {/each}
      </div>
    {/if}

    {#if needs.length}
      <h3 class="caps">{t('review.needsYou')} <span class="n">{needs.length}</span></h3>
      {#each needs as th (th.root.id)}
        {@render card(th)}
      {/each}
    {/if}

    {#if drafts.length}
      <h3 class="caps">{t('review.draftsOfReview')} <span class="n">{drafts.length}</span><span class="help">{t('review.notPostedYet')}</span></h3>
      {#each drafts as c, i (i)}
        <div class="card draft">
          <div class="top">
            <span class="pill draft">{c.round ? t('review.roundDraft', { n: c.round }) : t('review.draft')}</span>
            <button class="path" onclick={() => ondraft(c)}>{where(c)}</button>
          </div>
          {#if editing === c}
            <form class="reply-form" onsubmit={save}>
              <label for="draft-edit" class="sr-only">{t('common.edit')}</label>
              <!-- svelte-ignore a11y_autofocus -->
              <textarea
                id="draft-edit"
                class="field"
                rows="5"
                autofocus
                bind:value={text}
                onkeydown={(e) => {
                  if (e.key === 'Escape') (e.stopPropagation(), (editing = null))
                  else if (e.ctrlKey && e.key === 'Enter') save(e)
                }}
              ></textarea>
              <span class="keys">{t('review.saveOrCancel')}</span>
              <div class="actions">
                <button type="button" class="btn btn-secondary" onclick={() => (editing = null)}>{t('common.cancel')}</button>
                <button class="btn btn-primary" disabled={!text.trim()}>{t('common.save')}</button>
              </div>
            </form>
          {:else}
            <Markdown text={c.note} />
            <div class="actions">
              <button class="btn btn-secondary" onclick={() => ((editing = c), (text = c.note))}>{t('common.edit')}</button>
              <button class="btn btn-secondary" onclick={() => ondraft(c)}>{t('review.showInDiff')}</button>
              <button class="btn btn-secondary" onclick={() => ondrop(c)}>{t('common.delete')}</button>
            </div>
          {/if}
        </div>
      {/each}
    {/if}

    {#if closed.length}
      <h3 class="caps">{t('review.closedThreads')} <span class="n">{closed.length}</span></h3>
      {#each closed as th (th.root.id)}
        {@const state = states.get(th.root.id)!}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="card line go {state}" class:on={picked === th.root.id} onclick={(e) => goFrom(th, e)}>
          <span class="pill {state}">{label(state)}</span>
          <span class="mono path-text">{where(th.root)}</span>
          <span class="said">{th.root.author}: {th.root.body.split('\n')[0]}</span>
          {#if resolvable(th) && state === 'resolved'}
            <button class="btn btn-secondary small" disabled={busy === th.root.id} onclick={() => resolve(th)}>{t('review.reopen')}</button>
          {/if}
        </div>
      {/each}
    {/if}

    {#if !threads.length && !drafts.length}
      <p class="none">{t('review.noThreads')}</p>
    {/if}
  </div>
</div>

{#snippet card(th: Thread)}
  {@const state = states.get(th.root.id)!}
  {@const judged = judgementOf(reviewKey, th.root.id)}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="card go {state}" class:on={picked === th.root.id} onclick={(e) => goFrom(th, e)}>
    <div class="top">
      <span class="pill {state}">{label(state)}</span>
      <button class="to" title={t('review.goToThread')} onclick={() => go(th)}>
        <span class={th.root.path ? 'path' : 'general'}>{where(th.root)}</span>
        <span class="jump" aria-hidden="true"><Icon name="jump" size={13} /></span>
      </button>
    </div>
    {#each th.all as r (r.id)}
      <div class="remark" class:reply={r !== th.root}>
        <Avatar name={r.author} src={r.avatar} size={20} />
        <div class="said-block">
          <span class="who"><strong>{r.author}</strong> · {when(r.at)}</span>
          <Markdown text={r.body} />
        </div>
      </div>
    {/each}
    {#if judged}
      <div class="judged">
        <Icon name="persona" size={13} />
        <span><strong>{t('review.roundN', { n: judged.round })}</strong>{' · '}{#if judged.commit}{t('review.fixedIn')} <span class="mono">{short(judged.commit)}</span>{' · '}{/if}{judged.note}</span>
      </div>
    {/if}
    {#if replying === th.root.id}
      <form class="reply-form" onsubmit={(e) => reply(th, e)}>
        <label for="thread-reply-{th.root.id}" class="sr-only">{t('review.replyTo', { author: th.root.author })}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea
          id="thread-reply-{th.root.id}"
          class="field"
          rows="2"
          autofocus
          bind:value={answer}
          placeholder={t('review.replyTo', { author: th.root.author })}
          onkeydown={(e) => {
            if (e.key === 'Escape') (e.stopPropagation(), (replying = null))
            else if (e.ctrlKey && e.key === 'Enter') reply(th, e)
          }}
        ></textarea>
        <div class="actions">
          <button type="button" class="btn btn-secondary" onclick={() => (replying = null)}>{t('common.cancel')}</button>
          <button class="btn btn-primary" disabled={!answer.trim() || busy === th.root.id}>{busy === th.root.id ? t('review.replying') : t('review.replyOn', { host: hostName(pull) })}</button>
        </div>
      </form>
    {:else}
      <div class="actions">
        {#if failure?.id === th.root.id}<span class="bad" role="alert">{errorText(failure.text)}</span>{/if}
        {#if can(pull.host, 'reply')}
          <button class="btn btn-secondary" onclick={() => ((replying = th.root.id), (answer = ''), (failure = null))}>{t('review.reply')}</button>
        {/if}
        {#if resolvable(th)}
          <button class="btn btn-secondary" disabled={busy === th.root.id} onclick={() => resolve(th)}><Icon name="check" size={12} />{t('review.resolve')}</button>
        {/if}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .rounds {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .timeline {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 12px 14px 0;
    list-style: none;
  }

  .timeline li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    background: var(--surface-3);
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .timeline li.ok {
    color: var(--ok-text);
  }

  .timeline li.err {
    color: var(--err-text);
  }

  .timeline li.now {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .timeline li.push {
    color: var(--remote-text);
  }

  .timeline li.empty {
    height: auto;
    padding: 0;
    background: none;
    font-size: 12.5px;
  }

  .mono {
    font: 11.5px var(--mono);
    color: var(--text-3);
  }

  .body {
    flex: 1;
    min-height: 0;
    min-width: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px 24px;
  }

  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .count {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 9px;
    border-radius: 4px;
    background: var(--surface-2);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .tone {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .tone.open {
    background: var(--err);
  }

  .tone.answered {
    background: var(--remote-dot);
  }

  .tone.fixed {
    background: var(--ok);
  }

  .tone.outdated {
    background: var(--surface-5);
  }

  .caps {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 0 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .caps .n {
    padding: 0 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 600 11px var(--mono);
    letter-spacing: 0;
    color: var(--text-2);
  }

  .caps .help {
    font-size: 12px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-faint);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow:
      inset 3px 0 0 var(--text-faint),
      inset 0 0 0 1px var(--border);
    font: 13px/1.5 var(--sans);
    color: var(--text);
  }

  .card.open {
    --tone: var(--err);
    box-shadow:
      inset 3px 0 0 var(--err),
      inset 0 0 0 1px var(--border);
  }

  .card.answered {
    --tone: var(--remote-dot);
    box-shadow:
      inset 3px 0 0 var(--remote-dot),
      inset 0 0 0 1px var(--border);
  }

  .card.fixed {
    --tone: var(--ok);
    box-shadow:
      inset 3px 0 0 var(--ok),
      inset 0 0 0 1px var(--border);
  }

  .card.draft {
    box-shadow:
      inset 3px 0 0 var(--accent),
      inset 0 0 0 1px var(--border);
  }

  .card.on {
    box-shadow:
      inset 3px 0 0 var(--tone, var(--text-faint)),
      inset 0 0 0 1px var(--border-strong),
      0 0 0 2px var(--accent-soft);
  }

  .card.line {
    flex-direction: row;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
  }

  .card.line .said {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    color: var(--text-dim);
  }

  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .pill {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 600 11px var(--sans);
    color: var(--text-2);
  }

  .pill.open,
  .pill.err {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .pill.answered {
    background: color-mix(in srgb, var(--remote-dot) 16%, transparent);
    color: var(--remote-text);
  }

  .pill.fixed,
  .pill.ok {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .pill.draft {
    background: var(--wait-note);
    color: var(--accent-soft);
  }

  .pill.outdated {
    color: var(--text-faint);
  }

  .caps .n.wait {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .caps .link {
    margin-left: auto;
    font: 500 12px var(--sans);
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-dim);
  }

  .caps .link:hover {
    color: var(--accent-link);
  }

  .feed {
    flex: none;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .item {
    display: grid;
    grid-template-columns: 7px 20px minmax(0, 1fr) 14px;
    align-items: start;
    gap: 2px 8px;
    padding: 10px 10px 10px 12px;
    border-top: 1px solid var(--border);
    text-align: left;
    color: var(--text);
  }

  .item:first-child {
    border-top: none;
  }

  .item:hover {
    background: var(--surface-2);
  }

  .item.on {
    background: var(--surface-3);
  }

  .item.seen {
    opacity: 0.6;
  }

  .dot {
    width: 7px;
    height: 7px;
    margin-top: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .item.seen .dot {
    background: transparent;
  }

  .itxt {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .what {
    font-size: 12px;
    color: var(--text-dim);
  }

  .what strong {
    font-weight: 600;
    color: var(--text-2);
  }

  .itxt .said {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
  }

  .itxt .path,
  .itxt .general {
    font-size: 11.5px;
  }

  .item:hover .path,
  .item:hover .general {
    color: var(--accent-link);
  }

  .chev {
    margin-top: 3px;
    color: var(--text-faint);
  }

  .card.go {
    cursor: pointer;
  }

  .card.go:hover {
    background: var(--surface-2);
  }

  .to {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }

  .to .path,
  .to .general {
    flex: 1;
  }

  .jump {
    flex: none;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .card.go:hover .jump {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .card.go:hover .path,
  .card.go:hover .general {
    color: var(--accent-link);
  }

  .path,
  .path-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 12px var(--mono);
    color: var(--text-3);
  }

  .general {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .remark {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr);
    gap: 0 8px;
  }

  .remark.reply {
    margin-left: 28px;
    padding-left: 14px;
    border-left: 2px solid var(--border-strong);
  }

  .said-block {
    min-width: 0;
  }

  .who {
    display: block;
    margin-bottom: 2px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .who strong {
    color: var(--text-2);
  }

  .judged {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    background: var(--persona-bg);
    box-shadow: inset 0 0 0 1px var(--persona-line);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .judged :global(svg) {
    flex: none;
    margin-top: 3px;
    color: var(--persona);
  }

  .judged strong {
    color: var(--persona);
  }

  .keys {
    font-size: 12px;
    color: var(--text-faint);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions .btn,
  .btn.small {
    height: 30px;
    padding: 0 12px;
    gap: 6px;
  }

  .bad {
    margin-right: auto;
    font-size: 12.5px;
    color: var(--err-text);
  }

  .reply-form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .reply-form textarea {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    font: inherit;
  }

  .none {
    margin: 24px 0;
    text-align: center;
    font-size: 12.5px;
    color: var(--text-dim);
  }
</style>
