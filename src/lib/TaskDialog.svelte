<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { COLUMNS } from './board'
  import { find, show } from './agents.svelte'
  import { projects } from './projects.svelte'
  import { progress, pullOf, stateLabel } from './pulls.svelte'
  import { getTicket, type Ticket } from './tickets.svelte'
  import { codeOf, nameOf } from './taskref'
  import { t } from './i18n/index.svelte'
  import Dialog from './Dialog.svelte'
  import Markdown from './Markdown.svelte'
  import Attachment from './Attachment.svelte'
  import StatusIcon from './StatusIcon.svelte'

  let { id, onclose }: { id: string; onclose: () => void } = $props()

  const project = $derived(projects.list.find((p) => p.cards.some((c) => c.id === id)))
  const card = $derived(project?.cards.find((c) => c.id === id))
  const agent = $derived(card?.agent ? find(card.agent) : undefined)
  const pull = $derived(agent ? pullOf(agent) : null)
  const branch = $derived(pull ? `${pull.branch} → ${pull.base}` : (agent?.worktree?.branch ?? ''))
  let ticket = $state<Ticket | null>(null)

  $effect(() => {
    const link = card?.ticket
    if (link) getTicket(link.connection, link.key).then((found) => (ticket = found)).catch(() => {})
  })

  function board() {
    if (!project) return
    projects.overview = false
    projects.pulls = false
    projects.current = project.path
    projects.board = true
    onclose()
  }
</script>

{#if card}
<Dialog title={nameOf(card.title)} width={920} {onclose}>
  {#snippet badge()}
    {#if codeOf(card.title)}<span class="code">{codeOf(card.title)}</span>{/if}
  {/snippet}
  <div class="body">
    <div class="main">
      {#if card.goal}
        <section>
          <h2>{t('board.taskGoal')}</h2>
          <div class="goal">{card.goal}</div>
        </section>
      {/if}
      {#if card.prompt}
        <section>
          <h2>{t('board.taskDescription')}</h2>
          <Markdown text={card.prompt} />
        </section>
      {/if}
      {#if card.files?.length}
        <section>
          <h2>{t('board.taskFiles')}</h2>
          <div class="files">{#each card.files as file (file)}<Attachment path={file} look="card" />{/each}</div>
        </section>
      {/if}
    </div>
    <aside>
      <div>
        <h3>{t('board.taskColumn')}</h3>
        <span class="pill">{t(COLUMNS.find(([c]) => c === card.column)![1])}</span>
      </div>
      {#if card.ticket}
        <div>
          <h3>{t('board.taskTicket')}</h3>
          <button class="link" onclick={() => ticket?.url && invoke('open_link', { url: ticket.url })}>{card.ticket.key}</button>
          {#if ticket}<span class="muted"> · {ticket.status}</span><div class="muted">{ticket.title}</div>{/if}
        </div>
      {/if}
      <div>
        <h3>{t('board.taskAgent')}</h3>
        {#if agent}
          <div class="agent"><StatusIcon status={agent.status} /><span class="strong">{agent.name}</span></div>
          {#if agent.status === 'working' && agent.action}<div class="muted">{agent.action}</div>{/if}
        {:else}
          <span class="muted">{t('board.taskNoAgent')}</span>
        {/if}
      </div>
      {#if pull}
        <div>
          <h3>{t('board.taskPull')}</h3>
          <span class="strong">#{pull.number}</span> <span class="pill">{stateLabel(pull)}</span>
          {#if progress(pull)}<div class="muted">{progress(pull)}</div>{/if}
        </div>
      {/if}
      {#if branch}
        <div>
          <h3>{t('board.taskBranch')}</h3>
          <code>{branch}</code>
        </div>
      {/if}
    </aside>
  </div>
  <div class="foot">
    <button class="link" onclick={board}>{t('board.taskShowBoard')}</button>
    {#if agent}<button class="link" onclick={() => (show(agent), onclose())}>{t('board.taskShowAgent')}</button>{/if}
    {#if pull}<button class="link" onclick={() => invoke('open_link', { url: pull.url })}>{t('board.taskOpenPull')}</button>{/if}
  </div>
</Dialog>
{/if}

<style>
  .code {
    padding: 3px 9px;
    border-radius: 5px;
    background: var(--surface-3);
    font: 13px var(--mono);
    color: var(--text-strong);
  }

  .body {
    display: grid;
    grid-template-columns: 1fr 280px;
    min-height: 360px;
    max-height: calc(100vh - 220px);
    margin-top: 14px;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 20px 22px;
    overflow: auto;
  }

  h2 {
    margin: 0 0 6px;
    font-size: 11.5px;
    font-weight: 500;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  .goal {
    padding: 10px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  aside {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 20px;
    overflow: auto;
    border-left: 1px solid var(--border);
    background: var(--surface-1);
    font-size: 13px;
  }

  h3 {
    margin: 0 0 4px;
    font-size: 11.5px;
    font-weight: 400;
    color: var(--text-faint);
  }

  .pill {
    padding: 1px 8px;
    border-radius: 10px;
    background: var(--surface-3);
    font-size: 12px;
  }

  .agent {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .strong {
    color: var(--text-strong);
  }

  .muted {
    color: var(--text-muted);
  }

  code {
    font: 12.5px var(--mono);
    overflow-wrap: anywhere;
  }

  .link {
    padding: 0;
    background: none;
    color: var(--accent);
    font-size: 13px;
  }

  .foot {
    display: flex;
    gap: 20px;
    padding: 12px 22px;
  }
</style>
