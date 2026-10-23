<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { agents, clock, find, show } from './agents.svelte'
  import { ui } from './actions.svelte'
  import type { Project } from './projects.svelte'
  import { hostName, lists, lookUpLists, moved, progress, pulls, reopen, reviewWithAgent, sinceOf, standing, waitsFor, type Pull } from './pulls.svelte'
  import Avatar from './Avatar.svelte'
  import Reviewers from './Reviewers.svelte'
  import DiffPanel from './DiffPanel.svelte'
  import { workdir, type Comment } from './stream'

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

  async function askAgent(pull: Pull) {
    const agent = find(await reviewWithAgent(project.path, pull))
    if (agent) show(agent)
  }
  const waiting = $derived(found?.review.filter((p) => waitsFor(project.path, p)) ?? [])
  const reviewed = $derived(found?.review.filter((p) => !waitsFor(project.path, p)) ?? [])

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
    if (min < 60) return min < 1 ? 'just now' : `${min} min ago`
    if (min < 48 * 60) return `${Math.floor(min / 60)} h ago`
    return `${Math.floor(min / 1440)} days ago`
  }

  function verdict(pull: Pull): [string, string] {
    if (!pull.reviewed) return ['act-review', 'Waiting for you']
    if (moved(project.path, pull)) return ['act-recheck', 'New changes']
    if (pull.reviewed === 'approved') return ['done-ok', 'You approved']
    if (pull.reviewed === 'changes_requested') return ['wait-bad', 'You asked for changes']
    return ['quiet', 'You commented']
  }
</script>

<div class="page">
<main>
  <div class="top">
    <div class="heading">
      <button class="back" aria-label="Back to the conversation" title="Back to the conversation" onclick={onclose}>
        <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M19 12H5M11 18l-6-6 6-6" /></svg>
      </button>
      <h1>Pull requests · <span class="square" aria-hidden="true" style:background={project.color}></span>{project.name}</h1>
    </div>
    <button class="btn btn-secondary" disabled={refreshing} onclick={refresh}>{refreshing ? 'Refreshing…' : 'Refresh'}</button>
  </div>

  <div class="scroll">
    {#if !found}
      <p class="empty">Looking for pull requests…</p>
    {:else if found.error}
      <div class="problem">
        <p>{found.error}</p>
        {#if /token|scope/i.test(found.error)}
          <button class="btn btn-secondary" onclick={() => (ui.settings = 'hosting')}>Open Settings › Code hosting</button>
        {/if}
      </div>
    {:else}
      <section>
        <h2>To review <span class="n">{found.review.length}</span></h2>
        {#each [...waiting, ...reviewed] as pull (pull.number)}
          <article class:done={!waitsFor(project.path, pull)}>
            <span class="verdict {verdict(pull)[0]}">{verdict(pull)[1]}</span>
            {@render about(pull)}
            <div class="actions">
              <button class="btn btn-secondary open" onclick={() => invoke('open_link', { url: pull.url })}>Open on {hostName(pull)}</button>
              <button
                class="btn {waitsFor(project.path, pull) ? 'btn-primary' : 'btn-secondary'}"
                aria-pressed={opened?.number === pull.number}
                onclick={() => ((opened = opened?.number === pull.number ? null : pull), (file = ''))}
              >
                Review
              </button>
            </div>
          </article>
        {:else}
          <p class="empty">Nobody waits for your review in {project.name}.</p>
        {/each}
      </section>

      <section>
        <h2>Yours <span class="n">{found.mine.length}</span></h2>
        {#each found.mine as pull (pull.number)}
          {@const agent = agentOn(pull)}
          <article>
            <span class="verdict {standing(pull)[0]}">{standing(pull)[1]}</span>
            {@render about(pull)}
            <div class="actions">
              {#if agent}<button class="btn btn-secondary" title="The agent working on {pull.branch}" onclick={() => show(agent)}>{agent.name}</button>{/if}
              <button class="btn btn-secondary open" onclick={() => invoke('open_link', { url: pull.url })}>Open on {hostName(pull)}</button>
            </div>
          </article>
        {:else}
          <p class="empty">You have no open pull request in {project.name}.</p>
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

{#snippet about(pull: Pull)}
  <span class="author" title="By {pull.author}"><Avatar name={pull.author} src={pull.author_avatar} size={30} /></span>
  <div class="about">
    <strong>{pull.title}</strong>
    <span class="meta">
      <span class="mono">#{pull.number}</span> · {pull.author} · <span class="mono">{pull.branch} → {pull.base}</span> · updated {since(pull.updated)}
    </span>
  </div>
  <span class="facts">
    {#if pull.conflicts}<span class="tag bad" title="The branch conflicts with {pull.base}">Conflicts</span>{/if}
    {#if pull.checks}
      <span class="tag {pull.checks}" title="Builds and checks of the last commit"
        ><span class="checks {pull.checks}" aria-hidden="true"></span>{pull.checks === 'passing' ? 'Builds pass' : pull.checks === 'failing' ? 'Builds fail' : 'Building'}</span
      >
    {/if}
    {#if pull.reviewers.length || pull.approvals}
      <span class="review" title="{progress(pull)}">
        <Reviewers reviewers={pull.reviewers} />
        <span class="count">{progress(pull)}</span>
      </span>
    {/if}
    {#if pull.comments}
      <span class="talk" title="{pull.comments} comment{pull.comments > 1 ? 's' : ''}"
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
