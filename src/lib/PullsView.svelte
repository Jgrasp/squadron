<script lang="ts">
  import type { Snippet } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from './asking.svelte'
  import { agents, clock, find, reply, send, show } from './agents.svelte'
  import { ui } from './actions.svelte'
  import type { Project } from './projects.svelte'
  import { approve, hostName, lists, merge, lookUpLists, moved, progress, pullOf, fixWithAgent, reopen, reviewWithAgent, sinceOf, standing, waitsFor, type Pull } from './pulls.svelte'
  import Avatar from './Avatar.svelte'
  import Reviewers from './Reviewers.svelte'
  import DiffPanel from './DiffPanel.svelte'
  import { prompt, workdir, type Comment } from './stream'
  import Trans from './Trans.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, onclose, controls }: { project: Project; onclose?: () => void; controls?: Snippet } = $props()

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
  async function handOver(pull: Pull, summary: string, list: Comment[]) {
    opened = null
    const agent = agentOn(pull)
    if (!agent) return show(find(await fixWithAgent(project.path, pull, summary, list))!)
    const message = summary || `Review of pull request #${pull.number}`
    if (!reply(agent, prompt(message, list))) send(agent.id, message, list)
    show(agent)
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

  let menu = $state('')

  type Choice = { label: string; run: () => unknown; title?: string; always?: boolean }

  const openOn = (pull: Pull): Choice => ({ label: t('review.openOn', { host: hostName(pull) }), run: () => invoke('open_link', { url: pull.url }) })
  const merging = (pull: Pull): Choice[] =>
    pull.mergeable ? [{ label: busy?.number === pull.number && busy.doing === 'merge' ? t('review.merging') : t('review.merge'), title: t('review.mergeInto', { base: pull.base }), run: () => mergeIt(pull) }] : []
  const approving = (pull: Pull): Choice[] =>
    approvable(pull)
      ? [{ label: busy?.number === pull.number && busy.doing === 'approve' ? t('review.approving') : t('review.verdictApprove'), run: () => act(pull, 'approve', () => approve(project.path, pull)) }]
      : []

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

  const agentOn = (pull: Pull) => agents.list.find((a) => a.project === project.path && pullOf(a)?.number === pull.number)
  const reviewerOf = (pull: Pull) => agents.list.find((a) => a.review?.project === project.path && a.review.number === pull.number)

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

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest('[data-menu]') && (menu = '')} onkeydown={(e) => e.key === 'Escape' && menu && (menu = '')} />

<div class="page">
<main>
  <div class="top">
    <div class="heading">
      {#if onclose}
        <button class="back" aria-label={t('review.backToConversation')} title={t('review.backToConversation')} onclick={onclose}>
          <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M19 12H5M11 18l-6-6 6-6" /></svg>
        </button>
      {/if}
      <h1>{t('review.pullRequests')} · <span class="square" aria-hidden="true" style:background={project.color}></span>{project.name}</h1>
    </div>
    <div class="controls">
      <button class="btn btn-secondary" disabled={refreshing} onclick={refresh}>{refreshing ? t('review.refreshing') : t('review.refresh')}</button>
      {@render controls?.()}
    </div>
  </div>

  {#if !opened}{@render flashes()}{/if}

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
          {@const reviewer = reviewerOf(pull)}
          <article class:done={!waitsFor(project.path, pull)}>
            <span class="verdict {verdict(pull)[0]}">{verdict(pull)[1]}</span>
            {@render about(pull)}
            {@render split(
              `review-${pull.number}`,
              { label: t('review.review'), always: true, run: () => ((opened = opened?.number === pull.number ? null : pull), (file = '')) },
              waitsFor(project.path, pull),
              [openOn(pull), ...(reviewer ? [{ label: t('review.goToAgent', { name: reviewer.name }), title: t('review.agentReviewing', { name: reviewer.name }), run: () => show(reviewer) }] : []), ...approving(pull)],
              opened?.number === pull.number,
            )}
          </article>
        {:else}
          <p class="empty">{t('review.nobodyWaits', { project: project.name })}</p>
        {/each}
      </section>

      <section>
        <h2>{t('review.yours')} <span class="n">{found.mine.length}</span></h2>
        {#each found.mine as pull (pull.number)}
          {@const agent = agentOn(pull)}
          {@const others = agent ? [{ label: t('review.goToAgent', { name: agent.name }), title: t('review.agentWorkingOn', { branch: pull.branch }), run: () => show(agent) }] : []}
          {@const reviewing = { label: t('review.review'), always: true, run: () => ((opened = opened?.number === pull.number ? null : pull), (file = '')) }}
          <article>
            <span class="verdict {standing(pull)[0]}">{standing(pull)[1]}</span>
            {@render about(pull)}
            {#if pull.mergeable}
              {@render split(`mine-${pull.number}`, merging(pull)[0], true, [openOn(pull), ...others], false, reviewing, opened?.number === pull.number)}
            {:else}
              {@render split(`mine-${pull.number}`, reviewing, false, [openOn(pull), ...others], opened?.number === pull.number)}
            {/if}
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
      own={found?.mine.some((p) => p.number === opened?.number)}
      bind:file
      bind:comments={notes}
      bind:target
      onhand={() => {}}
      onreview={(summary, list) => handOver(opened!, summary, list)}
      handTo={(opened && agentOn(opened)?.name) || t('review.newAgent')}
      onposted={() => ((opened = null), lookUpLists(project.path, true))}
      reviewer={reviewerOf(opened) ?? null}
      onagent={() => askAgent(opened!)}
      banner={flashes}
      onclose={() => (opened = null)}
    />
  {/key}
{/if}
</div>

{#snippet flashes()}
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
{/snippet}

{#snippet split(key: string, main: Choice, primary: boolean, more: Choice[], pressed?: boolean, lead?: Choice, leadPressed?: boolean)}
  <div class="split" data-menu>
    {#if lead}
      <button class="btn btn-secondary lead" title={lead.title} aria-pressed={leadPressed} onclick={() => ((menu = ''), lead.run())}>{lead.label}</button>
    {/if}
    <button class="btn {primary ? 'btn-primary' : 'btn-secondary'} main" class:alone={!more.length} title={main.title} aria-pressed={pressed} disabled={!!busy && !main.always} onclick={() => ((menu = ''), main.run())}>
      {main.label}
    </button>
    {#if more.length}
      <button
        class="btn {primary ? 'btn-primary' : 'btn-secondary'} more"
        aria-label={t('review.moreActions')}
        title={t('review.moreActions')}
        aria-haspopup="menu"
        aria-expanded={menu === key}
        onclick={() => (menu = menu === key ? '' : key)}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
    {/if}
    {#if menu === key}
      <div class="choices" role="menu" aria-label={t('review.moreActions')}>
        {#each more as choice (choice.label)}
          <button role="menuitem" title={choice.title} disabled={!!busy} onclick={() => ((menu = ''), choice.run())}>{choice.label}</button>
        {/each}
      </div>
    {/if}
  </div>
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

  .controls {
    display: flex;
    align-items: center;
    gap: 8px;
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

  .split {
    position: relative;
    display: inline-flex;
    justify-self: end;
  }

  .split .btn {
    height: 32px;
    padding: 0 12px;
    white-space: nowrap;
  }

  .split .lead {
    margin-right: 8px;
  }

  .main:not(.alone) {
    border-radius: 4px 0 0 4px;
  }

  .more {
    width: 30px;
    margin-left: 1px;
    border-radius: 0 4px 4px 0;
  }

  .split .more {
    padding: 0;
  }

  .more svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .choices {
    position: absolute;
    top: 38px;
    right: 0;
    z-index: 30;
    min-width: 220px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .choices button {
    padding: 8px 10px;
    border-radius: 4px;
    text-align: left;
    white-space: nowrap;
  }

  .choices button:hover:not(:disabled) {
    background: var(--menu-hover);
  }

  .empty {
    margin: 0;
    color: var(--text-dim);
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

    .facts {
      display: none;
    }

    .split {
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
