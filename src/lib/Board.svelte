<script lang="ts">
  import { COLUMNS, follow, moveCard, stage, type Actions, type Card, type Column, type Mirror } from './board'
  import Dialog from './Dialog.svelte'
  import { t, type Key } from './i18n/index.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import { home, projects, type Project } from './projects.svelte'
  import { hint, settings } from './settings.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import { GOAL_RELAUNCHES, type Goal, type Options } from './stream'

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

  const LABELS: Record<string, Key> = {
    working: 'board.statusWorking',
    waiting: 'board.statusWaiting',
    done: 'board.statusDone',
    interrupted: 'board.statusInterrupted',
    error: 'board.statusError',
  }

  let editing = $state<{ card: Card; fresh: boolean; options: Options; worktree: string | null } | null>(null)
  let drag = $state<{ id: string; x: number; y: number; over: Column | null; before: string | null } | null>(null)
  let focusId = ''
  let picking = $state(false)

  const GOALS: Record<Goal['state'], Key> = {
    planned: 'board.goalPlanned',
    sent: 'board.goalSent',
    active: 'board.goalActive',
    met: 'board.goalMet',
    stopping: 'board.goalStopping',
    partial: 'board.goalPartial',
    cleared: 'board.goalCleared',
  }
  const goalLine = (g: Goal) =>
    g.state === 'active' && g.relaunches ? t('board.goalRelaunch', { n: g.relaunches, total: GOAL_RELAUNCHES }) : t(GOALS[g.state], { count: GOAL_RELAUNCHES })

  const agentOf = (card: Card) => agents.find((a) => a.id === card.agent)
  const look = (a: Mirror) => (a.attention && a.status !== 'working' && a.status !== 'error' ? 'waiting' : a.status)
  const label = (a: Mirror) => (a.status === 'waiting' && a.attention.split(' · ')[0]) || t(LABELS[look(a)])
  const virtual = $derived(
    agents
      .filter((a) => a.project === project.path && !project.cards.some((c) => c.agent === a.id))
      .map((a): Card => ({ id: a.id, key: '', title: a.name, prompt: '', column: stage(a), auto: stage(a), agent: a.id })),
  )
  const cards = $derived([...project.cards, ...virtual])
  const dragged = $derived(cards.find((c) => c.id === drag?.id))

  function save(next: Card[], keep = '') {
    const kept = next.filter((c) => c.id === keep || !virtual.some((v) => v.id === c.id))
    actions.cards(project.path, $state.snapshot(kept))
  }

  const defaults = () => ({ ...$state.snapshot(settings.defaults) })

  function create(column: Column = 'todo') {
    editing = { card: { id: crypto.randomUUID(), key: '', title: '', prompt: '', goal: '', column, agent: null }, fresh: true, options: defaults(), worktree: null }
  }

  function edit(card: Card) {
    editing = { card: $state.snapshot(card), fresh: false, options: defaults(), worktree: null }
  }

  function submit(event: Event, start = false) {
    event.preventDefault()
    if (!editing?.card.title.trim()) return
    const card = { ...editing.card, title: editing.card.title.trim(), prompt: editing.card.prompt.trim(), goal: editing.card.goal?.trim() ?? '' }
    save(editing.fresh ? [...project.cards, card] : cards.map((c) => (c.id === card.id ? card : c)), card.id)
    if (start) actions.start(project.path, card.id, card.title, $state.snapshot(editing.options), card.prompt || card.title, card.goal ?? '', editing.worktree)
    editing = null
  }

  function remove() {
    save(project.cards.filter((c) => c.id !== editing?.card.id))
    editing = null
  }

  function grab(event: PointerEvent, card: Card) {
    if (event.button !== 0 || (event.target as HTMLElement).closest('button')) return
    const [x0, y0] = [event.clientX, event.clientY]
    let moved = false
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    const move = (e: PointerEvent) => {
      if (!e.buttons) return end(e, true)
      if (!moved && Math.hypot(e.clientX - x0, e.clientY - y0) < 5) return
      moved = true
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
    const listeners: [string, (e: Event) => void][] = [
      ['pointermove', move as (e: Event) => void],
      ['pointerup', (e) => end(e, true)],
      ['pointercancel', (e) => end(e, false)],
      ['blur', (e) => end(e, false)],
      ['keydown', (e) => (e as KeyboardEvent).key === 'Escape' && end(e, false)],
    ]
    const end = (e: Event, drop: boolean) => {
      for (const [name, listener] of listeners) window.removeEventListener(name, listener)
      if (!moved && e.type === 'pointerup') edit(card)
      else if (drop && drag?.over) save(moveCard(cards, card.id, drag.over, drag.before), card.id)
      drag = null
    }
    for (const [name, listener] of listeners) window.addEventListener(name, listener)
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
          <button class="back" aria-label={t('board.backToConversation')} title={t('board.backToConversationHint', { shortcut: hint('kanban') })} onclick={onclose}>
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
              <div class="menu" role="menu" aria-label={t('board.projects')}>
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
      <span class="sub">{t('board.intro')}</span>
    </div>
    <div class="actions">
      <button class="btn btn-primary" onclick={() => create()}>{t('board.addTaskButton')}</button>
      {#if onpopout}
        <button class="btn btn-ghost" onclick={onpopout} title={t('board.popOutHint')}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6M20 4l-9 9M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" /></svg>
          {t('board.popOut')}
        </button>
      {/if}
    </div>
  </div>

  <div class="columns">
    {#each COLUMNS as [column, title] (column)}
      {@const list = cards.filter((c) => follow(c, agentOf(c)).column === column)}
      <section data-column={column} class:over={drag?.over === column} aria-labelledby="column-{column}">
        <h2 id="column-{column}"><span class="dot {column}" aria-hidden="true"></span>{t(title)} <span class="count">{list.length}</span></h2>
        <ul>
          {#each list as card (card.id)}
            {@const agent = agentOf(card)}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
            <li
              data-card={card.id}
              style:--project={project.color}
              class="card {agent ? look(agent) : 'task'} {agent && look(agent) === 'working' ? `effect-${settings.tab_effect}` : ''}"
              class:finished={column === 'done'}
              class:dragging={drag?.id === card.id}
              class:before={drag && drag.over === column && drag.before === card.id && drag.id !== card.id}
              tabindex="0"
              aria-keyshortcuts="Enter Alt+ArrowLeft Alt+ArrowRight"
              onpointerdown={(e) => grab(e, card)}
              ondragstart={(e) => e.preventDefault()}
              onkeydown={(e) => keys(e, card, column)}
              use:refocus={card.id}
            >
              <strong>{card.title}</strong>
              {#if !agent && card.prompt}<span class="note">{card.prompt}</span>{/if}
              {#if !agent && card.goal}<span class="goal" title={card.goal}>{t('board.cardDoneWhen', { goal: card.goal })}</span>{/if}
              {#if agent}
                <span class="state"><StatusIcon status={look(agent)} />{label(agent)}</span>
                {#if agent.detail}<span class="detail">{agent.detail}</span>{/if}
                {#if agent.goal}
                  <span class="goal {agent.goal.state}" title={agent.goal.condition}>{t('board.goalLine', { state: goalLine(agent.goal) })}</span>
                  {#if agent.goal.reason && agent.goal.state !== 'met'}<span class="note">{agent.goal.reason}</span>{/if}
                {/if}
                {#if agent.total}
                  <span class="progress">
                    <span class="bar" aria-hidden="true"><span style:width="{(agent.done / agent.total) * 100}%"></span></span>
                    <span class="mono">{agent.done}/{agent.total}</span>
                  </span>
                {/if}
                <span class="buttons">
                  <button class="link" class:answer={!!agent.attention} onclick={() => actions.show(agent.id)}>
                    {agent.attention ? t('board.answer') : t('board.openAgent')} ›
                  </button>
                  {#if column === 'review' && (agent.add || agent.del)}
                    <button class="link diff" onclick={() => actions.diff(agent.id)}>
                      {t('board.seeDiff')} <span class="add">+{agent.add}</span>{#if agent.del}<span class="del">−{agent.del}</span>{/if}
                    </button>
                  {/if}
                </span>
              {:else if column !== 'done'}
                <button class="link" disabled={!project.exists} onclick={() => edit(card)}>
                  <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4l12 8-12 8z" /></svg>{t('board.startAnAgent')}
                </button>
              {/if}
            </li>
          {/each}
          {#if column !== 'done'}
            <li class="add-row"><button class="new-task" onclick={() => create(column)}>{t('board.addTask')}</button></li>
          {/if}
        </ul>
      </section>
    {/each}
  </div>
</main>

{#if drag && dragged}
  <div class="ghost" style:left="{drag.x}px" style:top="{drag.y}px" aria-hidden="true">{dragged.title}</div>
{/if}

{#if editing}
  {@const linked = cards.some((c) => c.id === editing?.card.id && c.agent)}
  <Dialog title={editing.fresh ? t('board.newTask') : t('board.task')} width={640} onclose={() => (editing = null)}>
    <form onsubmit={(e) => submit(e)}>
      <div class="fields">
        <div class="group">
          <label for="card-title">{t('common.title')}</label>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="card-title" class="field" bind:value={editing.card.title} autofocus />
        </div>
        <div class="group">
          <label for="card-prompt">{t('common.description')}</label>
          <textarea
            id="card-prompt"
            class="field"
            rows="4"
            bind:value={editing.card.prompt}
            placeholder={t('board.taskPromptPlaceholder')}
            onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
          ></textarea>
        </div>
        {#if !linked}
          <div class="group">
            <label for="card-goal">{t('common.doneWhen')}</label>
            <textarea
              id="card-goal"
              class="field"
              rows="2"
              bind:value={editing.card.goal}
              placeholder={t('board.taskGoalPlaceholder')}
            ></textarea>
          </div>
          <details class="agent">
            <summary><span class="chevron" aria-hidden="true">›</span>{t('board.agentSettings')}</summary>
            <div class="options"><OptionsFields bind:options={editing.options} /></div>
          </details>
        {/if}
      </div>
      <footer>
        {#if !editing.fresh && project.cards.some((c) => c.id === editing?.card.id)}
          <button type="button" class="btn btn-danger" onclick={remove}>{t('common.delete')}</button>
        {/if}
        <span class="spacer"></span>
        <button type="button" class="btn btn-secondary" onclick={() => (editing = null)}>{t('common.cancel')}</button>
        {#if linked}
          <button class="btn btn-primary" disabled={!editing.card.title.trim()}>{t('common.save')}</button>
        {:else}
          <button type="button" class="btn btn-secondary" disabled={!editing.card.title.trim()} onclick={(e) => submit(e)}
            >{editing.fresh ? t('board.createTask') : t('common.save')}</button
          >
          <button
            type="button"
            class="btn btn-primary"
            disabled={!editing.card.title.trim() || !project.exists}
            onclick={(e) => submit(e, true)}>{editing.fresh ? t('board.createAndStart') : t('board.saveAndStart')}</button
          >
        {/if}
      </footer>
    </form>
  </Dialog>
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
    border-radius: 8px;
    background: var(--bg-sidebar);
  }

  section.over {
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-dashed);
  }

  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 14px 16px 10px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  h2 .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--text-faint);
  }

  h2 .dot.progress {
    background: var(--sub-running);
  }

  h2 .dot.review {
    background: var(--accent);
  }

  h2 .dot.done {
    background: var(--done);
  }

  .count {
    font: 500 11.5px var(--mono);
    letter-spacing: 0;
    color: var(--text-faint);
  }

  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0 10px 12px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
    cursor: grab;
    user-select: none;
    touch-action: none;
  }

  .card:hover {
    background: var(--surface-2);
  }

  .card.waiting {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .card.error {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
  }

  .card.finished strong {
    color: var(--text-3);
  }

  .card.dragging {
    opacity: 0.4;
  }

  .card.before {
    box-shadow: 0 -5px 0 -3px var(--accent);
  }

  strong {
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }

  .note {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-dim);
    overflow-wrap: anywhere;
  }

  .goal {
    font-size: 12px;
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }

  .goal.met {
    color: var(--done-text);
  }

  .goal.partial {
    color: var(--err-text);
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    margin-left: -8px;
    padding: 0 8px;
    border-radius: 4px;
    color: var(--accent-link);
    font-size: 12.5px;
    font-weight: 600;
  }

  .link svg {
    fill: currentColor;
  }

  .link:not(:disabled):hover {
    background: var(--surface-3);
    color: var(--accent-soft);
  }

  .link:disabled {
    opacity: 0.4;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
  }

  .diff .add {
    font-family: var(--mono);
    color: var(--ok);
  }

  .diff .del {
    font-family: var(--mono);
    color: var(--err);
  }

  .link.answer {
    margin-left: 0;
    padding: 0 10px;
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .link.answer:hover {
    background: var(--accent-soft);
    color: var(--accent-ink);
  }

  .add-row {
    list-style: none;
  }

  .new-task {
    width: 100%;
    height: 34px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--border-dashed);
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .new-task:hover {
    background: var(--surface-1);
    color: var(--text);
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding-top: 10px;
  }

  .agent summary {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
    font-weight: 600;
    cursor: pointer;
    list-style: none;
  }

  .agent summary::-webkit-details-marker {
    display: none;
  }

  .agent .chevron {
    display: inline-block;
    transition: transform 0.15s;
    color: var(--text-dim);
  }

  .agent[open] .chevron {
    transform: rotate(90deg);
  }

  .agent :global(.pair) {
    margin-top: 10px;
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

  .group {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
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
