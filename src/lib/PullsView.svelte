<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import { agents, clock, find, show } from './agents.svelte'
  import { ui } from './actions.svelte'
  import type { Project } from './projects.svelte'
  import { approve, hostName, lists, merge, lookUpLists, moved, progress, pulls, reopen, reviewWithAgent, sinceOf, standing, waitsFor, type Pull } from './pulls.svelte'
  import Avatar from './Avatar.svelte'
  import Reviewers from './Reviewers.svelte'
  import DiffPanel from './DiffPanel.svelte'
  import { workdir, type Comment } from './stream'
  import Trans from './Trans.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, onclose }: { project: Project; onclose: () => void } = $props()

  let refreshing = $state(false)
  let opened = $state<Pull | null>(null)
  let file = $state('')
  let notes = $state<Comment[]>([])
  let target = $state<Comment | null>(null)
  const found = $derived(lists[project.path])

  $effect(() => {
    const wanted = reopen.number
    const match = wanted !== null && found && [...found.review, ...found.mine].find((p) => p.number === wanted)
    if (!match) return
    opened = match
    reopen.number = null
  })

  let launched = $state<{ id: string; number: number } | null>(null)
  const launchedAgent = $derived(launched && find(launched.id))

  async function askAgent(pull: Pull) {
    const before = agents.current[project.path]
    const id = await reviewWithAgent(project.path, pull)
    if (before) agents.current[project.path] = before
    else delete agents.current[project.path]
    launched = { id, number: pull.number }
  }
  const waiting = $derived(found?.review.filter((p) => waitsFor(project.path, p)) ?? [])
  const reviewed = $derived(found?.review.filter((p) => !waitsFor(project.path, p)) ?? [])

  let busy = $state<{ number: number; doing: 'approve' | 'merge' } | null>(null)
  let failure = $state('')

  async function act(pull: Pull, doing: 'approve' | 'merge', work: () => Promise<unknown>) {
    busy = { number: pull.number, doing }
    failure = ''
    try {
      await work()
      await lookUpLists(project.path, true)
    } catch (e) {
      failure = `#${pull.number}: ${e}`
    } finally {
      busy = null
    }
  }

  const approvable = (pull: Pull) => pull.reviewed !== 'approved' || moved(project.path, pull)

  async function mergeIt(pull: Pull) {
    const confirmed = await ask(t('review.mergeAsk', { number: pull.number, title: pull.title, base: pull.base, host: hostName(pull) }), {
      title: t('review.mergeTitle'),
      kind: 'warning',
      okLabel: t('review.merge'),
      cancelLabel: t('common.cancel'),
    })
    if (confirmed) await act(pull, 'merge', () => merge(project.path, pull))
  }

  async function refresh() {
    refreshing = true
    await lookUpLists(project.path, true)
    refreshing = false
  }
  $effect(() => {
    lookUpLists(project.path)
  })

  const agentOn = (pull: Pull) => agents.list.find((a) => a.project === project.path && pulls[workdir(a)]?.number === pull.number)

  function since(at: string) {
    const min = Math.max(0, Math.floor((clock.now - Date.parse(at)) / 60_000))
    if (min < 60) return min < 1 ? t('format.justNow') : t('format.minutesAgo', { count: min })
    if (min < 48 * 60) return t('format.hoursAgo', { count: Math.floor(min / 60) })
    return t('review.daysAgo', { count: Math.floor(min / 1440) })
  }

  function verdict(pull: Pull): [string, string] {
    if (!pull.reviewed) return ['act-review', t('review.waitingForYou')]
    if (moved(project.path, pull)) return ['act-recheck', t('review.newChanges')]
    if (pull.reviewed === 'approved') return ['done-ok', t('review.youApproved')]
    if (pull.reviewed === 'changes_requested') return ['wait-bad', t('review.youAskedForChanges')]
    return ['quiet', t('review.youCommented')]
  }
</script>

<div class="page">
<main>
  <div class="top">
    <div class="heading">
      <button class="back" aria-label={t('review.backToConversation')} title={t('review.backToConversation')} onclick={onclose}>
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M19 12H5M11 18l-6-6 6-6" /></svg>
      </button>
      <h1>{t('review.pullRequests')} · <span class="square" aria-hidden="true" style:background={project.color}></span>{project.name}</h1>
    </div>
    <button class="btn btn-secondary" disabled={refreshing} onclick={refresh}>{refreshing ? t('review.refreshing') : t('review.refresh')}</button>
  </div>

  {#if launched && launchedAgent}
    <div class="flash" role="status">
      <span class="dot" aria-hidden="true"></span>
      <span class="grow"
        ><Trans
          k={launchedAgent.status === 'working'
            ? 'review.launchedWorking'
            : launchedAgent.status === 'waiting'
              ? 'review.launchedWaiting'
              : launchedAgent.status === 'error'
                ? 'review.launchedError'
                : launchedAgent.status === 'interrupted'
                  ? 'review.launchedInterrupted'
                  : 'review.launchedDone'}
          >{#snippet name()}<strong>{launchedAgent.name}</strong>{/snippet}</Trans
        ></span
      >
      <button class="btn btn-secondary" onclick={() => show(launchedAgent)}>{t('review.openConversation')}</button>
      <button class="close" aria-label={t('review.hideMessage')} title={t('review.hide')} onclick={() => (launched = null)}>×</button>
    </div>
  {/if}

  {#if failure}
    <div class="flash bad" role="alert">
      <span class="dot" aria-hidden="true"></span>
      <span class="grow">{failure}</span>
      <button class="close" aria-label={t('review.hideMessage')} title={t('review.hide')} onclick={() => (failure = '')}>×</button>
    </div>
  {/if}

  <div class="scroll">
    {#if !found}
      <p class="empty">{t('review.lookingForPulls')}</p>
    {:else if found.error}
      <div class="problem">
        <p>{errorText(found.error)}</p>
        {#if /token|scope/i.test(found.error)}
          <button class="btn btn-secondary" onclick={() => (ui.settings = 'hosting')}>{t('review.openHostingSettings')}</button>
        {/if}
      </div>
    {:else}
      <section>
        <h2>{t('review.toReview')} <span class="n">{found.review.length}</span></h2>
        {#each [...waiting, ...reviewed] as pull (pull.number)}
          <article class:done={!waitsFor(project.path, pull)}>
            <span class="verdict {verdict(pull)[0]}">{verdict(pull)[1]}</span>
            {@render about(pull)}
            <div class="actions">
              <button class="btn btn-secondary open" onclick={() => invoke('open_link', { url: pull.url })}>{t('review.openOn', { host: hostName(pull) })}</button>
              {#if approvable(pull)}
                <button class="btn btn-secondary" disabled={!!busy} onclick={() => act(pull, 'approve', () => approve(project.path, pull))}>
                  {busy?.number === pull.number && busy.doing === 'approve' ? t('review.approving') : t('review.verdictApprove')}
                </button>
              {/if}
              {@render merging(pull, 'btn-secondary')}
              <button
                class="btn {waitsFor(project.path, pull) ? 'btn-primary' : 'btn-secondary'}"
                aria-pressed={opened?.number === pull.number}
                onclick={() => ((opened = opened?.number === pull.number ? null : pull), (file = ''))}
              >
                {t('review.review')}
              </button>
            </div>
          </article>
        {:else}
          <p class="empty">{t('review.nobodyWaits', { project: project.name })}</p>
        {/each}
      </section>

      <section>
        <h2>{t('review.yours')} <span class="n">{found.mine.length}</span></h2>
        {#each found.mine as pull (pull.number)}
          {@const agent = agentOn(pull)}
          <article>
            <span class="verdict {standing(pull)[0]}">{standing(pull)[1]}</span>
            {@render about(pull)}
            <div class="actions">
              {#if agent}<button class="btn btn-secondary" title={t('review.agentWorkingOn', { branch: pull.branch })} onclick={() => show(agent)}>{agent.name}</button>{/if}
              <button class="btn btn-secondary open" onclick={() => invoke('open_link', { url: pull.url })}>{t('review.openOn', { host: hostName(pull) })}</button>
              {@render merging(pull, 'btn-primary')}
            </div>
          </article>
        {:else}
          <p class="empty">{t('review.noPullsOfYours', { project: project.name })}</p>
        {/each}
      </section>
    {/if}
  </div>
</main>
{#if opened}
  {#key opened.number}
    <DiffPanel
      agent={null}
      dir={project.path}
      changes={null}
      pull={opened}
      since={opened.reviewed && moved(project.path, opened) ? sinceOf(project.path, opened) : null}
      mode="pull"
      bind:file
      bind:comments={notes}
      bind:target
      onhand={() => {}}
      onreview={() => {}}
      onposted={() => ((opened = null), lookUpLists(project.path, true))}
      reviewer={agents.list.find((a) => a.review?.project === project.path && a.review.number === opened?.number) ?? null}
      onagent={() => askAgent(opened!)}
      onclose={() => (opened = null)}
    />
  {/key}
{/if}
</div>

{#snippet merging(pull: Pull, kind: string)}
  {#if pull.mergeable}
    <button class="btn {kind}" title={t('review.mergeInto', { base: pull.base })} disabled={!!busy} onclick={() => mergeIt(pull)}>
      {busy?.number === pull.number && busy.doing === 'merge' ? t('review.merging') : t('review.merge')}
    </button>
  {/if}
{/snippet}

{#snippet about(pull: Pull)}
  <span class="author" title={t('review.by', { author: pull.author })}><Avatar name={pull.author} src={pull.author_avatar} size={30} /></span>
  <div class="about">
    <strong>{pull.title}</strong>
    <span class="meta">
      <span class="mono">#{pull.number}</span> · {pull.author} · <span class="mono">{pull.branch} → {pull.base}</span> · {t('review.updated', { when: since(pull.updated) })}
    </span>
  </div>
  <span class="facts">
    {#if pull.conflicts}<span class="tag bad" title={t('review.conflictsWith', { base: pull.base })}>{t('review.conflicts')}</span>{/if}
    {#if pull.checks}
      <span class="tag {pull.checks}" title={t('review.checksTitle')}
        ><span class="checks {pull.checks}" aria-hidden="true"></span>{pull.checks === 'passing' ? t('review.buildsPass') : pull.checks === 'failing' ? t('review.buildsFail') : t('review.building')}</span
      >
    {/if}
    {#if pull.reviewers.length || pull.approvals}
      <span class="review" title="{progress(pull)}">
        <Reviewers reviewers={pull.reviewers} />
        <span class="count">{progress(pull)}</span>
      </span>
    {/if}
    {#if pull.comments}
      <span class="talk" title={t('review.commentCount', { count: pull.comments })}
        ><svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>{pull.comments}</span
      >
    {/if}
  </span>
{/snippet}

<style>
  .page {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }

  main {
    min-width: 0;
    container-type: inline-size;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-main);
  }

  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 24px 32px 16px;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  h1 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }

  .square {
    width: 12px;
    height: 12px;
    border-radius: 3px;
  }

  .back {
    width: 32px;
    height: 32px;
    margin-left: -6px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    color: var(--text-2);
  }

  .back:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .back svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .flash {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    margin: 0 32px 16px;
    padding: 8px 8px 8px 14px;
    border-radius: 6px;
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    font-size: 13px;
    color: var(--ok-text);
  }

  .flash.bad {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
  }

  .flash.bad .dot {
    background: var(--err);
  }

  .flash .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 4px;
    background: var(--ok);
  }

  .flash .grow {
    flex: 1 1 180px;
    min-width: 0;
  }

  .flash strong {
    color: var(--text-strong);
  }

  .flash .close {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 4px;
    font-size: 18px;
    line-height: 1;
    color: var(--text-dim);
  }

  .flash .close:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-width: 1100px;
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 6px;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .n {
    padding: 1px 7px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    letter-spacing: 0;
  }

  article {
    display: grid;
    grid-template-columns: 170px auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  article.done {
    background: transparent;
  }

  .verdict {
    justify-self: start;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
  }

  .verdict.act-review {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .verdict.act-recheck {
    background: #60a5fa;
    color: #0d1726;
  }

  .verdict.act-fix {
    background: var(--err);
    color: var(--err-ink);
  }

  .verdict.act-merge {
    background: var(--ok);
    color: var(--ink);
  }

  .verdict.done-ok {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .verdict.wait-bad {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .verdict.wait {
    background: rgba(96, 165, 250, 0.14);
    color: #a9cdfb;
  }

  .about {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .about strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .author {
    display: flex;
  }

  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }

  .facts {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12px;
    white-space: nowrap;
  }

  .tag.failing,
  .tag.bad {
    background: var(--err-bg);
    color: var(--err-text);
  }

  .review {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
  }

  .count {
    font-size: 12px;
    color: var(--text-2);
  }

  .talk {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .talk svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .checks {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .checks.passing {
    background: var(--ok);
  }

  .checks.failing {
    background: var(--err);
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions .btn {
    height: 32px;
    padding: 0 12px;
  }

  .empty {
    margin: 0;
    color: var(--text-dim);
  }

  @container (max-width: 1000px) {
    .open {
      display: none;
    }
  }

  @container (max-width: 760px) {
    .top,
    .scroll {
      padding-left: 20px;
      padding-right: 20px;
    }

    article {
      grid-template-columns: auto minmax(0, 1fr) auto;
      gap: 6px 10px;
    }

    .verdict {
      grid-column: 1 / span 2;
    }

    .author {
      grid-column: 1;
      grid-row: 2;
    }

    .about {
      grid-column: 2;
      grid-row: 2;
    }

    .facts,
    .open {
      display: none;
    }

    .actions {
      grid-column: 3;
      grid-row: 1 / span 2;
    }
  }

  .problem {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    max-width: 640px;
    padding: 16px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .problem p {
    margin: 0;
    color: var(--text-2);
  }
</style>
