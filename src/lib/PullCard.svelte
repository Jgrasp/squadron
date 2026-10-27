<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from './agents.svelte'
  import Avatar from './Avatar.svelte'
  import { ago } from './format'
  import { t } from './i18n/index.svelte'
  import { lookUpLink, showPulls, type Linked } from './linked.svelte'
  import { hostName, openReview, progress, stateLabel } from './pulls.svelte'

  let { url, inline = false, onready }: { url: string; inline?: boolean; onready?: () => void } = $props()

  let linked = $state<Linked | null>(null)
  let failed = $state(false)
  let hover = $state(false)
  let leave = 0

  $effect(() => {
    lookUpLink(url)
      .then((found) => ((linked = found), onready?.()))
      .catch(() => (failed = true))
  })

  const pull = $derived(linked?.pull)
  const trouble = $derived(!!pull && pull.state === 'open' && (pull.changes_requested || pull.checks === 'failing' || pull.conflicts))
  const tone = $derived(
    !pull ? 'loading' : pull.state === 'merged' ? 'merged' : pull.state !== 'open' ? 'closed' : trouble ? 'bad' : pull.draft ? 'draft' : 'ok',
  )
  const badge = $derived(pull ? (pull.state === 'open' && pull.changes_requested ? t('review.changesRequested') : stateLabel(pull)) : '')
  const host = $derived(pull ? hostName(pull) : '')

  function stop(event: Event) {
    event.preventDefault()
    event.stopPropagation()
  }

  function review(event: Event) {
    stop(event)
    if (linked?.project && pull) openReview(linked.project, pull.number)
  }

  function squadron(event: Event) {
    stop(event)
    if (linked?.project) showPulls(linked.project)
  }

  function outside(event: Event) {
    stop(event)
    invoke('open_link', { url: pull?.url || url })
  }

  function enter() {
    clearTimeout(leave)
    hover = true
  }

  function away() {
    leave = window.setTimeout(() => (hover = false), 180)
  }
</script>

{#snippet icon()}
  {#if pull?.host === 'bitbucket' || url.includes('bitbucket.org')}
    <svg class="host" viewBox="0 0 16 16" aria-hidden="true"><path d="M.78 1.5a.5.5 0 0 0-.5.58l2.06 12.5c.05.31.32.54.64.55h10.03a.5.5 0 0 0 .5-.42L15.6 2.08a.5.5 0 0 0-.5-.58H.78zm8.86 9.04H6.42l-.87-4.58h4.87l-.78 4.58z" /></svg>
  {:else}
    <svg class="host" viewBox="0 0 16 16" aria-hidden="true"
      ><path
        d="M8 0C3.58 0 0 3.58 0 8a8 8 0 0 0 5.47 7.59c.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.06-.49.06-.49.8.06 1.23.83 1.23.83.72 1.22 1.87.87 2.33.66.07-.52.28-.87.5-1.07-1.78-.2-3.65-.89-3.65-3.95 0-.87.31-1.59.83-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8z"
      /></svg
    >
  {/if}
{/snippet}

{#snippet card()}
  <div class="card {tone}">
    <div class="body">
      <div class="top">
        {@render icon()}
        <span class="repo">{linked?.repo ?? ''}</span>
        <span class="num">#{pull?.number ?? url.match(/(\d+)\D*$/)?.[1]}</span>
        {#if pull}<span class="state">{badge}</span>{/if}
      </div>
      {#if pull}
        <div class="title">{pull.title}</div>
        <div class="meta">
          <span class="who"><Avatar name={pull.author} src={pull.author_avatar} size={18} />{pull.author}</span>
          {#if pull.base && pull.branch}<span class="branch">{pull.base} ← {pull.branch}</span>{/if}
          {#if pull.updated}<span>{t('review.updated', { when: ago(Date.parse(pull.updated), clock.now) })}</span>{/if}
        </div>
        {#if pull.state === 'open'}
          <div class="facts">
            {#if pull.checks}
              <span class="fact {pull.checks}">{pull.checks === 'passing' ? '✓ ' + t('review.buildsPass') : pull.checks === 'failing' ? '✕ ' + t('review.buildsFail') : '◌ ' + t('review.building')}</span>
            {/if}
            {#if pull.conflicts}<span class="fact failing">⚠ {t('review.conflicts')}</span>{/if}
            <span class="fact">
              {#if pull.reviewers.length}<span class="faces">{#each pull.reviewers.slice(0, 3) as r (r.name)}<Avatar name={r.name} src={r.avatar} size={16} />{/each}</span>{/if}
              {progress(pull)}
            </span>
            {#if pull.comments}<span class="fact">{t('review.commentCount', { count: pull.comments })}</span>{/if}
            {#if linked?.files}
              <span class="fact mono">{t('review.diffSize', { added: String(linked.additions ?? 0), removed: String(linked.deletions ?? 0), count: linked.files })}</span>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="bar wide"></div>
        <div class="bar"></div>
      {/if}
    </div>
    {#if pull}
      <div class="actions">
        {#if linked?.project && pull.state === 'open'}
          <button type="button" class="btn btn-primary" onclick={review}>{t('review.review')}</button>
        {/if}
        {#if linked?.project}
          <button type="button" class="btn btn-secondary" onclick={squadron}>{t('review.openWithSquadron')}</button>
        {/if}
        <button type="button" class="btn btn-ghost out" onclick={outside}>{t('review.openOn', { host })}</button>
      </div>
    {/if}
  </div>
{/snippet}

{#if inline}
  {#if pull}
    <span class="pill-anchor" role="presentation" onmouseenter={enter} onmouseleave={away}>
      <a class="pill {tone}" href={pull.url || url} title={pull.title}>{@render icon()}<span class="n">#{pull.number}</span><span class="t">{pull.title}</span></a>
      {#if hover}<span class="popover">{@render card()}</span>{/if}
    </span>
  {/if}
{:else if !failed}
  {@render card()}
{/if}

<style>
  .card,
  .pill {
    --tone: var(--ok);
  }

  .card {
    position: relative;
    display: block;
    margin: 4px 0 12px;
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow: 0 0 0 1px var(--border-strong);
    overflow: hidden;
    font-size: 13px;
    line-height: 1.5;
  }

  .card::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: var(--tone);
  }

  .bad {
    --tone: var(--err);
  }

  .draft,
  .closed {
    --tone: var(--text-faint);
  }

  .merged {
    --tone: #c4a7f0;
  }

  .loading {
    --tone: var(--surface-5);
  }

  .body {
    padding: 12px 14px 12px 17px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .host {
    width: 14px;
    height: 14px;
    flex: none;
    fill: currentColor;
  }

  .repo,
  .num,
  .branch,
  .mono {
    font-family: var(--mono);
  }

  .num {
    color: var(--text-muted);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--tone) 14%, transparent);
    color: var(--tone);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
  }

  .draft .state,
  .closed .state {
    color: var(--text-muted);
  }

  .state::before {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }

  .title {
    margin: 6px 0 4px;
    font-size: 15px;
    font-weight: 600;
    line-height: 1.35;
    color: var(--text-strong);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .who {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .branch {
    font-size: 12px;
    color: var(--text-muted);
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }

  .fact {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--surface-2);
    font-size: 12px;
    color: var(--text-2);
  }

  .fact.passing {
    color: var(--ok);
  }

  .fact.failing {
    background: var(--err-bg);
    color: var(--err);
  }

  .fact.pending {
    color: var(--accent-link);
  }

  .faces {
    display: inline-flex;
  }

  .faces :global(> *:not(:first-child)) {
    margin-left: -5px;
  }

  .actions {
    display: flex;
    gap: 6px;
    padding: 8px 14px 10px 17px;
    border-top: 1px solid var(--border);
    background: var(--surface-1);
  }

  .actions .btn {
    height: 28px;
    padding: 0 11px;
    font-size: 12.5px;
  }

  .actions .out {
    margin-left: auto;
  }

  .bar {
    width: 45%;
    height: 10px;
    margin-top: 8px;
    border-radius: 3px;
    background: linear-gradient(90deg, var(--surface-2), var(--surface-4), var(--surface-2));
    background-size: 200% 100%;
    animation: shimmer 1.4s infinite linear;
  }

  .bar.wide {
    width: 70%;
    height: 13px;
    margin-top: 10px;
  }

  @keyframes shimmer {
    to {
      background-position: -200% 0;
    }
  }

  .pill-anchor {
    position: relative;
    display: inline;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 340px;
    height: 21px;
    padding: 0 8px 0 6px;
    vertical-align: -4px;
    border-radius: 11px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text);
    font-size: 12.5px;
    text-decoration: none;
  }

  .pill:hover {
    box-shadow: inset 0 0 0 1px var(--tone);
  }

  .pill .host {
    color: var(--tone);
  }

  .pill .n {
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  .pill .t {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .popover {
    position: absolute;
    top: 22px;
    left: 0;
    z-index: 20;
    display: block;
    width: 460px;
    padding-top: 6px;
  }

  .popover .card {
    margin: 0;
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 16px 40px rgba(0, 0, 0, 0.55);
  }
</style>
