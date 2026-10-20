<script lang="ts">
  import { COLUMNS, moveCard, place, type Actions, type Card, type Column, type Mirror } from './board'
  import Dialog from './Dialog.svelte'
  import NewAgent from './NewAgent.svelte'
  import { home, projects, type Project } from './projects.svelte'
  import StatusIcon from './StatusIcon.svelte'

  let {
    project,
    agents,
    actions,
    popout = false,
    onclose,
    onpopout,
  }: {
    project: Project
    agents: Mirror[]
    actions: Actions
    popout?: boolean
    onclose?: () => void
    onpopout?: () => void
  } = $props()

  const LABELS = { working: 'Working', waiting: 'Waiting for you', done: 'Done', interrupted: 'Interrupted', error: 'Error' }

  let editing = $state<{ card: Card; fresh: boolean } | null>(null)
  let starting = $state<Card | null>(null)
  let drag = $state<{ id: string; x: number; y: number; over: Column | null; before: string | null } | null>(null)
  let focusId = ''
  let picking = $state(false)

  const agentOf = (card: Card) => agents.find((a) => a.id === card.agent)
  const look = (a: Mirror) => (a.attention && a.status !== 'working' && a.status !== 'error' ? 'waiting' : a.status)
  const label = (a: Mirror) => (a.status === 'waiting' && a.attention.split(' · ')[0]) || LABELS[look(a)]
  const virtual = $derived(
    agents
      .filter((a) => a.project === project.path && !project.cards.some((c) => c.agent === a.id))
      .map((a): Card => ({ id: a.id, key: '', title: a.name, prompt: '', column: 'progress', agent: a.id })),
  )
  const cards = $derived([...project.cards, ...virtual])
  const dragged = $derived(cards.find((c) => c.id === drag?.id))

  function save(next: Card[], keep = '') {
    const kept = next.filter((c) => c.id === keep || !virtual.some((v) => v.id === c.id))
    actions.cards(project.path, $state.snapshot(kept))
  }

  function create() {
    editing = { card: { id: crypto.randomUUID(), key: '', title: '', prompt: '', column: 'todo', agent: null }, fresh: true }
  }

  function edit(card: Card) {
    editing = { card: $state.snapshot(card), fresh: false }
  }

  function submit(event: Event) {
    event.preventDefault()
    if (!editing?.card.title.trim()) return
    const card = { ...editing.card, key: editing.card.key.trim(), title: editing.card.title.trim(), prompt: editing.card.prompt.trim() }
    save(editing.fresh ? [...project.cards, card] : cards.map((c) => (c.id === card.id ? card : c)), card.id)
    editing = null
  }

  function remove() {
    save(project.cards.filter((c) => c.id !== editing?.card.id))
    editing = null
  }

  function grab(event: PointerEvent, card: Card) {
    if (event.button !== 0 || (event.target as HTMLElement).closest('button')) return
    const el = event.currentTarget as HTMLElement
    const [x0, y0] = [event.clientX, event.clientY]
    el.setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => {
      if (!drag && Math.hypot(e.clientX - x0, e.clientY - y0) < 5) return
      const under = document.elementFromPoint(e.clientX, e.clientY)
      const target = under?.closest<HTMLElement>('[data-card]')
      const after = target && e.clientY > target.getBoundingClientRect().top + target.offsetHeight / 2
      drag = {
        id: card.id,
        x: e.clientX,
        y: e.clientY,
        over: (under?.closest<HTMLElement>('[data-column]')?.dataset.column as Column) ?? null,
        before: after ? ((target.nextElementSibling as HTMLElement | null)?.dataset.card ?? null) : (target?.dataset.card ?? null),
      }
    }
    const up = (e: PointerEvent) => {
      el.removeEventListener('pointermove', move)
      el.removeEventListener('pointerup', up)
      el.removeEventListener('pointercancel', up)
      if (!drag && e.type === 'pointerup') edit(card)
      else if (drag?.over) save(moveCard(cards, card.id, drag.over, drag.before), card.id)
      drag = null
    }
    el.addEventListener('pointermove', move)
    el.addEventListener('pointerup', up)
    el.addEventListener('pointercancel', up)
  }

  function keys(event: KeyboardEvent, card: Card, column: Column) {
    if (event.target !== event.currentTarget) return
    if (event.key === 'Enter') edit(card)
    else if (event.altKey && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
      event.preventDefault()
      const next = COLUMNS[COLUMNS.findIndex(([c]) => c === column) + (event.key === 'ArrowLeft' ? -1 : 1)]
      if (!next) return
      focusId = card.id
      save(moveCard(cards, card.id, next[0]), card.id)
    }
  }

  function refocus(el: HTMLElement, id: string) {
    if (id !== focusId) return
    el.focus()
    focusId = ''
  }
</script>

<svelte:window
  onclick={(e) => !(e.target as HTMLElement).closest('[data-pick]') && (picking = false)}
  onkeydown={(e) => e.key === 'Escape' && (picking = false)}
/>

<main class:popout>
  <div class="top">
    <div>
      <div class="heading">
        {#if onclose}
          <button class="back" aria-label="Back to the conversation" title="Back to the conversation (Ctrl K)" onclick={onclose}>
            <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path d="M19 12H5M11 18l-6-6 6-6" /></svg>
          </button>
        {/if}
        <h1>
          Kanban ·
          {#if popout}
            <span class="sr-only">{project.name}</span>
          {:else}
            <span class="square" aria-hidden="true" style:background={project.color}></span>{project.name}
          {/if}
        </h1>
        {#if popout}
          <span class="pick" data-pick>
            <button aria-haspopup="menu" aria-expanded={picking} onclick={() => (picking = !picking)}>
              <span class="square" aria-hidden="true" style:background={project.color}></span>{project.name}
              <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
            </button>
            {#if picking}
              <div class="menu" role="menu" aria-label="Projects">
                {#each projects.list as p (p.path)}
                  <button
                    role="menuitemradio"
                    aria-checked={p.path === project.path}
                    onclick={() => ((projects.current = p.path), (picking = false))}
                  >
                    <strong><span class="square" aria-hidden="true" style:background={p.color}></span>{p.name}</strong>
                    <span>{home(p.path)}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </span>
        {/if}
      </div>
      <span class="sub">Cards in progress follow the state of their agent</span>
    </div>
    <div class="actions">
      <button class="btn btn-primary" onclick={create}>+ Card</button>
      {#if onpopout}
        <button class="btn btn-ghost" onclick={onpopout} title="Open the board in its own window">
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></svg>
          Pop out
        </button>
      {/if}
    </div>
  </div>

  <div class="columns">
    {#each COLUMNS as [column, title] (column)}
      {@const list = cards.filter((c) => place(c, agentOf(c)) === column)}
      <section data-column={column} class:over={drag?.over === column} aria-labelledby="column-{column}">
        <h2 id="column-{column}">{title} <span>{list.length}</span></h2>
        <ul>
          {#each list as card (card.id)}
            {@const agent = agentOf(card)}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
            <li
              data-card={card.id}
              class="card {agent ? look(agent) : ''}"
              class:dragging={drag?.id === card.id}
              class:before={drag && drag.over === column && drag.before === card.id && drag.id !== card.id}
              tabindex="0"
              aria-keyshortcuts="Enter Alt+ArrowLeft Alt+ArrowRight"
              onpointerdown={(e) => grab(e, card)}
              onkeydown={(e) => keys(e, card, column)}
              use:refocus={card.id}
            >
              {#if card.key}<span class="key">{card.key}</span>{/if}
              <strong>{card.title}</strong>
              {#if agent}
                <span class="state"><StatusIcon status={look(agent)} />{label(agent)}</span>
                {#if agent.detail}<span class="detail">{agent.detail}</span>{/if}
                {#if agent.total}
                  <span class="progress">
                    <span class="bar" aria-hidden="true"><span style:width="{(agent.done / agent.total) * 100}%"></span></span>
                    <span class="mono">{agent.done}/{agent.total}</span>
                  </span>
                {/if}
                <button class="link" class:answer={!!agent.attention} onclick={() => actions.show(agent.id)}>
                  {agent.attention ? 'Answer' : 'Open agent'}
                </button>
              {:else if column !== 'done'}
                <button class="link" disabled={!project.exists} onclick={() => (starting = card)}>Start an agent</button>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
</main>

{#if drag && dragged}
  <div class="ghost" style:left="{drag.x}px" style:top="{drag.y}px" aria-hidden="true">{dragged.title}</div>
{/if}

{#if editing}
  <Dialog title={editing.fresh ? 'New card' : 'Edit card'} width={560} onclose={() => (editing = null)}>
    <form onsubmit={submit}>
      <div class="fields">
        <div class="row">
          <div class="group key-field">
            <label for="card-key">Key</label>
            <input id="card-key" class="field mono" bind:value={editing.card.key} placeholder="T16" />
          </div>
          <div class="group">
            <label for="card-title">Title</label>
            <!-- svelte-ignore a11y_autofocus -->
            <input id="card-title" class="field" bind:value={editing.card.title} autofocus />
          </div>
        </div>
        <div class="group">
          <label for="card-prompt">Instruction</label>
          <textarea
            id="card-prompt"
            class="field"
            rows="5"
            bind:value={editing.card.prompt}
            placeholder="First message of the agent started from this card"
            onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
          ></textarea>
        </div>
      </div>
      <footer>
        {#if !editing.fresh && project.cards.some((c) => c.id === editing?.card.id)}
          <button type="button" class="btn btn-danger" onclick={remove}>Delete</button>
        {/if}
        <span class="spacer"></span>
        <button type="button" class="btn btn-secondary" onclick={() => (editing = null)}>Cancel</button>
        <button class="btn btn-primary" disabled={!editing.card.title.trim()}>{editing.fresh ? 'Add card' : 'Save'}</button>
      </footer>
    </form>
  </Dialog>
{/if}

{#if starting}
  {@const card = starting}
  <NewAgent
    {project}
    draft={{ name: [card.key, card.title].filter(Boolean).join(' · '), text: card.prompt || card.title }}
    start={(name, options, text) => actions.start(project.path, card.id, name, options, text)}
    onclose={() => (starting = null)}
  />
{/if}

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
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 24px 32px 16px;
  }

  .heading {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
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
    flex: none;
    width: 12px;
    height: 12px;
    margin: 0 2px;
    border-radius: 3px;
  }

  .menu .square {
    width: 9px;
    height: 9px;
    margin: 0 8px 0 0;
    border-radius: 2px;
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

  .pick {
    position: relative;
  }

  .pick > button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    margin-left: -8px;
    padding: 0 6px 0 8px;
    border-radius: 4px;
    font-size: 22px;
    font-weight: 700;
  }

  .pick > button:hover,
  .pick > button[aria-expanded='true'] {
    background: var(--surface-3);
  }

  .pick > button svg {
    color: var(--text-dim);
  }

  .menu {
    position: absolute;
    top: 40px;
    left: -8px;
    z-index: 5;
    width: 270px;
    max-height: 60vh;
    overflow-y: auto;
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

  .menu button {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 9px 10px;
    border-radius: 4px;
    text-align: left;
    font-size: 13.5px;
  }

  .menu button:hover,
  .menu button[aria-checked='true'] {
    background: var(--menu-hover);
  }

  .menu strong {
    display: flex;
    align-items: center;
    font-weight: 600;
  }

  .menu span {
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  .heading svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .sub {
    color: var(--text-dim);
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions .btn {
    height: 34px;
  }

  .actions svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(220px, 1fr));
    gap: 12px;
    padding: 0 32px 24px;
    overflow-x: auto;
  }

  section {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    background: var(--bg-sidebar);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  section.over {
    box-shadow: inset 0 0 0 1px var(--border-focus);
    background: var(--surface-1);
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 12px 14px 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  h2 span {
    font: 500 11.5px var(--mono);
    letter-spacing: 0;
    color: var(--text-faint);
  }

  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 2px 10px 10px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 12px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    cursor: grab;
    user-select: none;
    touch-action: none;
  }

  .card:hover {
    background: var(--surface-3);
  }

  .card.waiting {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .card.error {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
  }

  .card.dragging {
    opacity: 0.4;
  }

  .card.before {
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 -5px 0 -3px var(--accent);
  }

  .key {
    font: 500 11.5px var(--mono);
    color: var(--accent-soft);
  }

  strong {
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .state {
    display: flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    font-size: 12.5px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .waiting .state {
    color: var(--accent-soft);
    font-weight: 600;
  }

  .error .state {
    color: var(--err-text);
    font-weight: 600;
  }

  .detail {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font: 12px var(--mono);
    color: var(--text-3);
    overflow-wrap: anywhere;
  }

  .waiting .detail {
    color: var(--wait-text-2);
  }

  .progress {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .bar {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--surface-4);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--ok);
  }

  .mono {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .link {
    height: 28px;
    margin-top: 2px;
    padding: 0 10px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .link:not(:disabled):hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .link:disabled {
    opacity: 0.4;
  }

  .link.answer {
    background: var(--accent);
    box-shadow: none;
    color: var(--accent-ink);
    font-weight: 700;
  }

  .ghost {
    position: fixed;
    z-index: 10;
    max-width: 260px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-4);
    box-shadow:
      0 0 0 1px var(--border-focus),
      0 12px 30px rgba(0, 0, 0, 0.5);
    font-weight: 600;
    pointer-events: none;
    transform: translate(-50%, -50%) rotate(-2deg);
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 10px 22px 20px;
  }

  .row {
    display: flex;
    gap: 12px;
  }

  .group {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .key-field {
    flex: 0 0 120px;
  }

  label {
    font-weight: 600;
  }

  textarea {
    height: auto;
    padding: 12px 14px;
    resize: none;
    line-height: 1.5;
  }

  .field.mono {
    font: 13px var(--mono);
    color: var(--text);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }

  .spacer {
    flex: 1;
  }

  .btn-danger {
    background: #3a2420;
    color: #f8d5cc;
  }
</style>
