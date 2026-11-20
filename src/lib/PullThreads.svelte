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
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { agents } from './agents.svelte'
  import { can } from './forges.svelte'
  import { hostName, rounds, type Pull } from './pulls.svelte'
  import type { Comment } from './stream'
  import { date, errorText, t, type Key } from './i18n/index.svelte'

  let {
    pull,
    folder,
    reviewKey,
    threads,
    drafts,
    onfresh,
    ongo,
    ondraft,
    ondrop,
  }: {
    pull: Pull
    folder: string
    reviewKey: string
    threads: Thread[]
    drafts: Comment[]
    onfresh: () => void
    ongo: (thread: Thread) => void
    ondraft: (c: Comment) => void
    ondrop: (c: Comment) => void
  } = $props()


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
          <p class="note">{c.note}</p>
          <div class="actions">
            <button class="btn btn-secondary" onclick={() => ondraft(c)}>{t('review.showInDiff')}</button>
            <button class="btn btn-secondary" onclick={() => ondrop(c)}>{t('common.delete')}</button>
          </div>
        </div>
      {/each}
    {/if}

    {#if closed.length}
      <h3 class="caps">{t('review.closedThreads')} <span class="n">{closed.length}</span></h3>
      {#each closed as th (th.root.id)}
        {@const state = states.get(th.root.id)!}
        <div class="card line {state}">
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
  <div class="card {state}">
    <div class="top">
      <span class="pill {state}">{label(state)}</span>
      {#if th.root.path}<button class="path" onclick={() => ongo(th)}>{where(th.root)}</button>{:else}<span class="general">{where(th.root)}</span>{/if}
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
    box-shadow:
      inset 3px 0 0 var(--err),
      inset 0 0 0 1px var(--border);
  }

  .card.answered {
    box-shadow:
      inset 3px 0 0 var(--remote-dot),
      inset 0 0 0 1px var(--border);
  }

  .card.fixed {
    box-shadow:
      inset 3px 0 0 var(--ok),
      inset 0 0 0 1px var(--border);
  }

  .card.draft {
    box-shadow:
      inset 3px 0 0 var(--accent),
      inset 0 0 0 1px var(--border);
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

  .path:hover {
    color: var(--accent-link);
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

  .note {
    margin: 0;
    white-space: pre-wrap;
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
