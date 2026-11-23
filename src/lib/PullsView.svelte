<script lang="ts">
  import { tick, untrack, type Snippet } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from './asking.svelte'
  import { agents, clock, find, show } from './agents.svelte'
  import { ui } from './actions.svelte'
  import { settings, type Config } from './settings.svelte'
  import type { Project } from './projects.svelte'
  import { layout, requestPanel } from './layout.svelte'
  import { askMerge, awaitsChecks, hostName, ignore, ignored, ignoredUntil, lists, lookUpNumbered, merge, lookUpLists, moved, numbered, refreshPull, progress, pullOf, behind, reviewChanges, reviewerOf, reviewWithAgent, standing, unignore, waitsFor, type Pull } from './pulls.svelte'
  import Avatar from './Avatar.svelte'
  import Icon from './Icon.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import Reviewers from './Reviewers.svelte'
  import PullReview from './PullReview.svelte'
  import type { Agent } from './stream'
  import Trans from './Trans.svelte'
  import { coverage, lookUpCoverage, offerFor, remoteHost } from './forges.svelte'
  import { catalog, extensions, loadCatalog } from './extensions.svelte'
  import type { CatalogEntry } from './extensions'
  import { named } from './extension-pages.svelte'
  import HostLogo from './HostLogo.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import InstallDialog from './InstallDialog.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, onclose, controls }: { project: Project; onclose?: () => void; controls?: Snippet } = $props()

  let refreshing = $state(false)
  let opened = $state<Pull | null>(null)
  const found = $derived(lists[project.path])
  const covered = $derived(coverage[project.path])
  const bare = $derived(!!covered && !covered.by)
  const offer = $derived(bare && covered?.remote ? offerFor(covered.remote) : null)
  let chosen = $state<CatalogEntry | null>(null)

  $effect(() => {
    void extensions.list
    void extensions.unloaded
    void lookUpCoverage([project.path])
  })

  $effect(() => {
    if (bare && !catalog.entries) void loadCatalog()
  })

  let launched = $state<{ id: string; number: number } | null>(null)
  const launchedAgent = $derived(launched && find(launched.id))

  async function review(pull: Pull) {
    const again = opened?.number === pull.number
    if (opened) {
      opened = null
      await tick()
    }
    if (again) return
    requestPanel('diff')
    opened = pull
  }

  const known = (number: number) => {
    const found = lists[project.path]
    return found && ([...found.review, ...found.mine, ...found.others].find((p) => p.number === number) ?? numbered[`${project.path}#${number}`])
  }
  const shownPull = $derived(opened && (known(opened.number) ?? opened))

  $effect(() => {
    const open = layout.review
    if (!open || open.project !== project.path) return
    const pull = known(open.number)
    if (pull) {
      layout.review = null
      if (untrack(() => opened?.number) !== pull.number) void review(pull)
      return
    }
    if (!lists[open.project]) return
    if (!(`${open.project}#${open.number}` in numbered)) return void lookUpNumbered(open.project, open.number)
    layout.review = null
  })

  $effect(() => {
    if (!opened) return
    layout.wider.pulls = 1120
    return () => delete layout.wider.pulls
  })

  $effect(() => {
    const pull = shownPull
    if (!pull || !awaitsChecks(project.path, pull)) return
    const timer = setInterval(() => refreshPull(project.path, pull.number), 20_000)
    return () => clearInterval(timer)
  })

  async function askAgent(pull: Pull) {
    busy = { number: pull.number, doing: 'ask' }
    try {
      launched = { id: await reviewWithAgent(project.path, pull), number: pull.number }
    } finally {
      busy = null
    }
  }
  const starting = (pull: Pull) => busy?.number === pull.number && busy.doing === 'ask'
  const asking = (pull: Pull): Choice => ({
    label: t('review.agentButton'),
    icon: 'persona',
    title: settings.pilot ? t('review.askAgentTitle') : t('review.askAgentNeedsPilot'),
    run: () => (settings.pilot ? askAgent(pull).catch((e) => (failure = `#${pull.number}: ${errorText(e)}`)) : (ui.settings = 'agents')),
  })
  async function askAgain(agent: Agent, pull: Pull) {
    failure = ''
    busy = { number: pull.number, doing: 'ask' }
    try {
      await reviewChanges(agent, pull)
      launched = { id: agent.id, number: pull.number }
    } catch (e) {
      failure = `#${pull.number}: ${e}`
    } finally {
      busy = null
    }
  }
  const shown = (p: Pull) => !ignored(project.path, p)
  const waiting = $derived(found?.review.filter((p) => waitsFor(project.path, p)) ?? [])
  const reviewed = $derived(found?.review.filter((p) => shown(p) && !waitsFor(project.path, p)) ?? [])
  const watched = $derived(found?.others.filter((p) => shown(p) && reviewerOf(project.path, p)) ?? [])
  const others = $derived(found?.others.filter((p) => shown(p) && !p.draft && !reviewerOf(project.path, p)) ?? [])
  const watchNeeds = (p: Pull) => reviewerOf(project.path, p)?.status !== 'working' && (!p.reviewed || moved(project.path, p))
  const mineNeeds = (p: Pull) => p.changes_requested || p.mergeable
  type Entry = { kind: 'review' | 'watched' | 'other' | 'mine'; pull: Pull }
  const entries = (kind: Entry['kind'], pulls: Pull[]): Entry[] => pulls.map((pull) => ({ kind, pull }))
  const needsYou = $derived([
    ...entries('review', waiting),
    ...entries('watched', watched.filter(watchNeeds)),
    ...entries('mine', found?.mine.filter((p) => !p.draft && mineNeeds(p)) ?? []),
  ])
  const waitingOnOthers = $derived([
    ...entries('review', reviewed),
    ...entries('watched', watched.filter((p) => !watchNeeds(p))),
    ...entries('other', others),
    ...entries('mine', found?.mine.filter((p) => !p.draft && !mineNeeds(p)) ?? []),
  ])
  const drafts = $derived(entries('mine', found?.mine.filter((p) => p.draft) ?? []))
  const hidden = $derived(found ? [...found.review, ...found.others].filter((p) => !shown(p)) : [])
  const ignoredGroups = $derived([
    { title: t('review.ignoredUntilCommits'), pulls: hidden.filter((p) => ignoredUntil(project.path, p)) },
    { title: t('review.ignoredForGood'), pulls: hidden.filter((p) => !ignoredUntil(project.path, p)) },
  ])
  const ignoring = (pull: Pull): Choice[] => [
    { label: t('review.ignoreUntilCommits'), hint: t('review.ignoreUntilCommitsHint'), icon: 'eye-off', rule: true, always: true, run: () => ignore(project.path, pull, true) },
    { label: t('review.ignore'), hint: t('review.ignoreHint'), icon: 'eye-off', always: true, run: () => ignore(project.path, pull, false) },
  ]

  let busy = $state<{ number: number; doing: 'merge' | 'ask' } | null>(null)
  let failure = $state('')

  async function act(pull: Pull, doing: 'merge', work: () => Promise<unknown>) {
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

  type Choice = { label: string; run: () => unknown; title?: string; hint?: string; rule?: boolean; always?: boolean; spinning?: boolean; again?: boolean; icon?: string; status?: Agent['status'] }
  type Split = { main: Choice; primary?: boolean; more: Choice[]; pressed?: boolean; lead?: Choice; frozen?: boolean; first?: Choice; firstPrimary?: boolean; firstPressed?: boolean }

  const openOn = (pull: Pull): Choice => ({ label: t('review.openOn', { host: hostName(pull) }), run: () => invoke('open_link', { url: pull.url }) })
  const merging = (pull: Pull): Choice => ({ label: busy?.number === pull.number && busy.doing === 'merge' ? t('review.merging') : t('review.merge'), title: t('review.mergeInto', { base: pull.base }), run: () => mergeIt(pull) })
  const mergesHere = (pull: Pull, where: Config['merge_others']) => pull.mergeable && settings.merge_others === where
  async function mergeIt(pull: Pull) {
    const choice = await askMerge(pull)
    if (choice) await act(pull, 'merge', () => merge(project.path, pull, choice))
  }

  async function refresh() {
    refreshing = true
    await Promise.all([lookUpCoverage([project.path]), lookUpLists(project.path, true)])
    refreshing = false
  }
  const path = $derived(project.path)
  $effect(() => {
    lookUpLists(path, true)
  })

  const agentOn = (pull: Pull) => agents.list.find((a) => a.project === project.path && pullOf(a)?.number === pull.number)

  function since(at: string) {
    const min = Math.max(0, Math.floor((clock.now - Date.parse(at)) / 60_000))
    if (min < 60) return min < 1 ? t('format.justNow') : t('format.minutesAgo', { count: min })
    if (min < 48 * 60) return t('format.hoursAgo', { count: Math.floor(min / 60) })
    return t('review.daysAgo', { count: Math.floor(min / 1440) })
  }

  function verdict(pull: Pull): [string, string] {
    if (!pull.reviewed) return ['act-review', t('review.waitingForYou')]
    if (pull.reviewed === 'approved') return ['done-ok', t('review.youApproved')]
    if (moved(project.path, pull)) return ['act-recheck', t('review.newChanges')]
    if (pull.reviewed === 'changes_requested') return ['wait-bad', t('review.youAskedForChanges')]
    return ['quiet', t('review.youCommented')]
  }

  function agentLabel(status: Agent['status']) {
    if (status === 'working') return t('review.agentButtonWorking')
    if (status === 'waiting') return t('review.agentButtonWaiting')
    if (status === 'error') return t('review.agentButtonError')
    if (status === 'interrupted') return t('review.agentButtonStopped')
    return t('review.agentButtonDone')
  }

  function watching(pull: Pull, agent: Agent): [string, string] {
    if (agent.status === 'working') return ['agent', t('review.agentAtWork')]
    if (agent.status === 'error') return ['wait-bad', t('review.agentFailed')]
    if (agent.status === 'interrupted') return ['quiet', t('review.agentStopped')]
    if (behind(agent, pull)) return ['act-recheck', t('review.newChanges')]
    if (agent.status === 'waiting') return ['agent', t('review.agentWaits')]
    return ['agent', t('review.agentReviewed')]
  }

  function openState(pull: Pull): [string, string] {
    if (pull.changes_requested) return ['wait-bad', t('review.changesRequested')]
    if (pull.reviewers.length && pull.reviewers.every((r) => r.verdict === 'approved')) return ['done-ok', t('review.approved')]
    return ['wait', t('review.inReview')]
  }
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest('[data-menu]') && (menu = '')} onkeydown={(e) => e.key === 'Escape' && menu && (menu = '')} />

<div class="page" style:--panel-top="0px">
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
      <button class="refresh" disabled={refreshing} onclick={refresh}>
        <svg class:spinning={refreshing} width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 12a8 8 0 1 1-2.34-5.66M20 4v5h-5" /></svg>
        {refreshing ? t('review.refreshing') : t('review.refresh')}
      </button>
      {@render controls?.()}
    </div>
  </div>

  {#if !opened}{@render flashes()}{/if}

  <div class="scroll">
    {#if bare && covered}
      {@render noForge(covered.remote)}
    {:else if !found}
      <div class="fetching" role="status">
        <svg class="ring" width="34" height="34" viewBox="0 0 34 34" aria-hidden="true"><circle class="track" cx="17" cy="17" r="14" /><circle class="arc" cx="17" cy="17" r="14" /></svg>
        <strong>{t('review.lookingForPulls')}</strong>
        <span class="why">{t('review.lookingWhy', { project: project.name })}</span>
      </div>
    {:else if found.error}
      <div class="problem">
        <p>{errorText(found.error)}</p>
        {#if /token|scope|account/i.test(found.error)}
          <button class="btn btn-secondary" onclick={() => (ui.settings = 'hosting')}>{t('review.openHostingSettings')}</button>
        {/if}
      </div>
    {:else}
      <section>
        <h2>{t('review.needsYou')} <span class="n">{needsYou.length}</span></h2>
        {#each needsYou as e (e.pull.number)}
          {@render entry(e)}
        {:else}
          <p class="empty">{t('review.nothingNeedsYou', { project: project.name })}</p>
        {/each}
      </section>

      {#each [{ title: t('review.waitingOnOthers'), list: waitingOnOthers }, { title: t('review.drafts'), list: drafts }] as group (group.title)}
        {#if group.list.length}
          <section>
            <h2>{group.title} <span class="n">{group.list.length}</span></h2>
            {#each group.list as e (e.pull.number)}
              {@render entry(e)}
            {/each}
          </section>
        {/if}
      {/each}

      {#if hidden.length}
        <section>
          <h2>{t('review.ignored')} <span class="n">{hidden.length}</span><span class="hint">{t('review.ignoredHint')}</span></h2>
          {#each ignoredGroups as group (group.title)}
            {#if group.pulls.length}
              <h3>{group.title}</h3>
              {#each group.pulls as pull (pull.number)}
                <article class="done ignored">
                  <span class="verdict quiet ignored-verdict"><Icon name="eye-off" size={12} />{t('review.ignoredPull')}</span>
                  {@render about(pull)}
                  <button class="btn btn-ghost unignore" onclick={() => unignore(project.path, pull)}>{t('review.stopIgnoring')}</button>
                </article>
              {/each}
            {/if}
          {/each}
        </section>
      {/if}
    {/if}
  </div>
</main>
{#if opened}
  <PullReview {project} pull={shownPull!} onagent={() => askAgent(opened!)} banner={flashes} onclose={() => (opened = null)} />
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

{#snippet entry(e: Entry)}
  {#if e.kind === 'mine'}
    {@render mineRow(e.pull)}
  {:else if e.kind === 'watched'}
    {@render reviewRow(e.pull, watching(e.pull, reviewerOf(project.path, e.pull)!), false, false)}
  {:else if e.kind === 'other'}
    {@render reviewRow(e.pull, openState(e.pull), false, true)}
  {:else}
    {@render reviewRow(e.pull, verdict(e.pull), waitsFor(project.path, e.pull), !waitsFor(project.path, e.pull))}
  {/if}
{/snippet}

{#snippet mineRow(pull: Pull)}
  {@const agent = agentOn(pull)}
  {@const others = agent ? [{ label: t('review.goToAgent', { name: agent.name }), title: t('review.agentWorkingOn', { branch: pull.branch }), run: () => show(agent) }] : []}
  {@const reviewing = { label: t('review.review'), always: true, run: () => review(pull) }}
  <article>
    <span class="verdict {standing(pull)[0]}">{standing(pull)[1]}</span>
    {@render about(pull)}
    {@render split(`mine-${pull.number}`, { main: reviewing, more: [openOn(pull), ...others], pressed: opened?.number === pull.number, first: pull.mergeable ? merging(pull) : undefined, firstPrimary: true })}
  </article>
{/snippet}

{#snippet reviewRow(pull: Pull, state: [string, string], primary: boolean, done: boolean)}
  {@const reviewer = reviewerOf(project.path, pull)}
  {@const reviewing = { label: t('review.review'), always: true, run: () => review(pull) }}
  {@const status = reviewer && (reviewer.attention && reviewer.status !== 'working' && reviewer.status !== 'error' ? 'waiting' : reviewer.status)}
  {@const again = !!reviewer && moved(project.path, pull) && behind(reviewer, pull) && reviewer.status !== 'working'}
  {@const agentChoice = starting(pull)
    ? { label: t('review.starting'), icon: 'persona', spinning: true, run: () => {} }
    : reviewer && again
      ? { label: t('review.agentButtonChanges'), title: t('review.reviewNewChangesTitle', { name: reviewer.name }), icon: 'persona', again: true, run: () => askAgain(reviewer, pull) }
      : reviewer && status
        ? { label: agentLabel(status), title: t('review.agentReviewing', { name: reviewer.name }), always: true, icon: 'persona', status, run: () => show(reviewer) }
        : asking(pull)}
  {@const more = [openOn(pull), ...(mergesHere(pull, 'menu') ? [merging(pull)] : []), ...(reviewer && again ? [{ label: t('review.goToAgent', { name: reviewer.name }), run: () => show(reviewer) }] : []), ...ignoring(pull)]}
  <article class:done>
    <span class="verdict {state[0]}">{state[1]}</span>
    {@render about(pull)}
    {#if mergesHere(pull, 'second')}
      {@render split(`review-${pull.number}`, { main: merging(pull), more, lead: agentChoice, frozen: starting(pull), first: reviewing, firstPrimary: primary, firstPressed: opened?.number === pull.number })}
    {:else}
      {@render split(`review-${pull.number}`, { main: reviewing, primary, more, pressed: opened?.number === pull.number, lead: agentChoice, frozen: starting(pull) })}
    {/if}
  </article>
{/snippet}

{#snippet split(key: string, { main, primary = false, more, pressed, lead, frozen = false, first, firstPrimary = false, firstPressed }: Split)}
  <div class="split" data-menu>
    {#if lead?.icon}
      <button class="btn lead agent" class:starting={lead.spinning} title={lead.title ?? lead.label} disabled={lead.spinning || (!!busy && !lead.always)} onclick={() => ((menu = ''), lead.run())}>
        <span class="sign">
          {#if lead.spinning}<svg class="spin" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" stroke-dasharray="40 20" /></svg>
          {:else if lead.again}<svg class="again" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 12a8 8 0 1 1-2.34-5.66M20 4v5h-5" /></svg>
          {:else if lead.status}<StatusIcon status={lead.status} />
          {:else}<Icon name={lead.icon} />{/if}
        </span>
        <span class="label">{lead.label}</span>
      </button>
    {/if}
    {#if first}
      <button class="btn {firstPrimary ? 'btn-primary' : 'btn-secondary'} first" title={first.title} aria-pressed={firstPressed} disabled={!!busy && !first.always} onclick={() => ((menu = ''), first.run())}>{first.label}</button>
    {/if}
    <button class="btn {primary ? 'btn-primary' : 'btn-secondary'} main" class:alone={!more.length} title={main.title} aria-pressed={pressed} disabled={frozen || (!!busy && !main.always)} onclick={() => ((menu = ''), main.run())}>
      {#if main.spinning}<svg class="spin" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" stroke-dasharray="40 20" /></svg>{/if}
      {main.label}
    </button>
    {#if more.length}
      <button
        class="btn {primary ? 'btn-primary' : 'btn-secondary'} more"
        disabled={frozen}
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
          {#if choice.rule}<span class="rule" role="separator"></span>{/if}
          <button role="menuitem" class:hinted={!!choice.hint} title={choice.title} disabled={!!busy && !choice.always} onclick={() => ((menu = ''), choice.run())}>
            {#if choice.icon}<span class="choice-icon"><Icon name={choice.icon} /></span>{/if}
            <span>{choice.label}{#if choice.hint}<small>{choice.hint}</small>{/if}</span>
          </button>
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
      <span class="mono">#{pull.number}</span> · {pull.author} · <span class="mono">{pull.branch} → {pull.base}</span> ·
      {#if pull.reviewed && moved(project.path, pull)}<span class="fresh">{pull.reviewed === 'approved' ? t('review.commitsSinceApproval') : t('review.commitsSinceReview')}</span>{:else}{t('review.updated', { when: since(pull.updated) })}{/if}
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

{#snippet noForge(remote: string | null)}
  <div class="no-forge">
    {#if offer && remote}
      {@const name = named(offer.name)}
      <span class="no-forge-logo">
        {#if offer.id === 'github'}<HostLogo host="github" size={26} />{:else}<ExtensionLogo id={offer.id} {name} size={26} />{/if}
      </span>
      <h3>{t('review.onForge', { project: project.name, name })}</h3>
      <p>{t('review.installForge', { name })}</p>
      <div class="no-forge-actions">
        <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: `https://${remote}` })}>{t('review.openOn', { host: name })}</button>
        <button class="btn btn-primary" onclick={() => (chosen = offer)}>{t('settings.forgeInstall', { name })}</button>
      </div>
    {:else}
      <span class="no-forge-logo dim"><Icon name="jump" size={24} /></span>
      <h3>{remote ? t('review.noForgeKnows', { host: remoteHost(remote) }) : t('review.noRemote', { project: project.name })}</h3>
      <p>{t('review.noForgeWhy')}</p>
      <div class="no-forge-actions">
        <button class="btn btn-secondary" onclick={() => (ui.settings = 'hosting')}>{t('review.openHostingSettings')}</button>
        <button class="btn btn-primary" onclick={() => (ui.settings = 'discover')}>{t('review.findForge')}</button>
      </div>
    {/if}
  </div>
  {#if chosen}<InstallDialog entry={chosen} onclose={() => (chosen = null)} />{/if}
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

  .refresh {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
    white-space: nowrap;
  }

  .refresh:hover:not(:disabled) {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .refresh:disabled {
    opacity: 0.6;
  }

  .refresh svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .refresh .spinning {
    animation: spin 0.9s linear infinite;
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

  .hint {
    margin-left: 4px;
    font-size: 12.5px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-faint);
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

  .fresh {
    color: var(--remote-text);
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
    background: var(--remote-dot);
    color: var(--ink);
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
    background: color-mix(in srgb, var(--remote-dot) 14%, transparent);
    color: var(--remote-text);
  }

  .verdict.agent {
    background: var(--persona-bg);
    box-shadow: inset 0 0 0 1px var(--persona-line);
    color: var(--persona);
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

  .split :is(.lead, .first) {
    margin-right: 8px;
  }

  .split .main {
    min-width: 76px;
  }

  .split .agent {
    width: 152px;
    justify-content: flex-start;
    gap: 8px;
    background: var(--persona-bg);
    box-shadow: inset 0 0 0 1px var(--persona-line);
    color: var(--persona);
  }

  .split .agent:hover:not(:disabled) {
    background: color-mix(in srgb, var(--persona) 16%, transparent);
  }

  .split .agent.starting:disabled {
    opacity: 1;
  }

  .split .agent ~ .main {
    width: 88px;
  }

  .again {
    fill: none;
    stroke: var(--remote-dot);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .sign {
    flex: none;
    width: 14px;
    display: grid;
    place-items: center;
  }

  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .spin {
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    animation: spin 1.4s linear infinite;
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

  .choices button.hinted {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .choices small {
    display: block;
    font-size: 12px;
    color: var(--text-dim);
  }

  .choice-icon {
    display: flex;
    padding-top: 3px;
    color: var(--text-dim);
  }

  .rule {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-strong);
  }

  h3 {
    margin: 8px 0 2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .ignored .author,
  .ignored .about {
    opacity: 0.55;
  }

  .ignored-verdict {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-dim);
  }

  .unignore {
    justify-self: end;
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

    .unignore {
      grid-column: 3;
      grid-row: 1 / span 2;
    }
  }

  .fetching {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
  }

  .fetching strong {
    margin-top: 8px;
    font-size: 15px;
    color: var(--text-strong);
  }

  .why {
    max-width: 420px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .ring {
    fill: none;
    stroke-width: 3;
    animation: spin 1.1s linear infinite;
  }

  .ring .track {
    stroke: var(--surface-4);
  }

  .ring .arc {
    stroke: var(--accent);
    stroke-linecap: round;
    stroke-dasharray: 22 66;
  }

  .no-forge {
    margin: 64px auto 0;
    width: 460px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
  }

  .no-forge-logo {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 8px;
    background: var(--surface-3);
  }

  .no-forge-logo.dim {
    color: var(--text-dim);
  }

  .no-forge h3 {
    margin: 6px 0 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--text-strong);
  }

  .no-forge p {
    margin: 0;
    color: var(--text-muted);
  }

  .no-forge-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
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
