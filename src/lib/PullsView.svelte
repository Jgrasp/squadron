<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { agents, clock, show } from './agents.svelte'
  import { ui } from './actions.svelte'
  import type { Project } from './projects.svelte'
  import { hostName, lists, lookUpLists, pulls, stateLabel, type Pull } from './pulls.svelte'
  import { workdir } from './stream'

  let { project, onclose }: { project: Project; onclose: () => void } = $props()

  let refreshing = $state(false)
  const found = $derived(lists[project.path])
  const waiting = $derived(found?.review.filter((p) => !p.reviewed) ?? [])
  const reviewed = $derived(found?.review.filter((p) => p.reviewed) ?? [])

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

  const verdict = (pull: Pull) =>
    pull.reviewed === 'approved' ? 'You approved' : pull.reviewed === 'changes_requested' ? 'You asked for changes' : 'Waiting for you'
</script>

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
          <article class:done={!!pull.reviewed}>
            <span class="verdict {pull.reviewed ?? 'waiting'}">{verdict(pull)}</span>
            {@render about(pull)}
            <div class="actions">
              <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: pull.url })}>Open on {hostName(pull)}</button>
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
            <span class="verdict {pull.changes_requested ? 'changes_requested' : pull.approvals ? 'approved' : pull.draft ? 'draft' : 'open'}">
              {pull.changes_requested ? 'Changes requested' : pull.approvals ? `${pull.approvals} approval${pull.approvals > 1 ? 's' : ''}` : stateLabel(pull)}
            </span>
            {@render about(pull)}
            <div class="actions">
              {#if agent}<button class="btn btn-secondary" title="The agent working on {pull.branch}" onclick={() => show(agent)}>{agent.name}</button>{/if}
              <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: pull.url })}>Open on {hostName(pull)}</button>
            </div>
          </article>
        {:else}
          <p class="empty">You have no open pull request in {project.name}.</p>
        {/each}
      </section>
    {/if}
  </div>
</main>

{#snippet about(pull: Pull)}
  <div class="about">
    <strong>{pull.title}</strong>
    <span class="meta">
      <span class="mono">#{pull.number}</span> · {pull.author} · <span class="mono">{pull.branch} → {pull.base}</span> · updated {since(pull.updated)}
    </span>
  </div>
  <span class="facts">
    {#if pull.checks}<span class="checks {pull.checks}" title="Builds {pull.checks}"></span>{/if}
    {#if pull.comments}
      <span class="talk" title="{pull.comments} comment{pull.comments > 1 ? 's' : ''}"
        ><svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>{pull.comments}</span
      >
    {/if}
  </span>
{/snippet}

<style>
  main {
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
    grid-template-columns: 170px minmax(0, 1fr) auto auto;
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

  .verdict.waiting {
    background: var(--accent);
    color: var(--accent-ink);
  }

  .verdict.approved,
  .verdict.open {
    background: var(--ok-bg);
    color: var(--ok-text);
  }

  .verdict.changes_requested {
    background: var(--err-bg);
    color: var(--err-text);
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
