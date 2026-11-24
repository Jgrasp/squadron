<script lang="ts">
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { ACTIONS, binding, combo, label, usable, type Action } from './shortcuts'

  const SECTIONS: [Key, Action[]][] = [
    ['settings.agents', ['newAgent', 'jump', 'interrupt', 'find']],
    ['settings.tabPanels', ['kanban', 'editor', 'terminal', 'diff', 'quickOpen', 'sidebar']],
    ['settings.shortcutsNavigation', ['search', 'projects', 'back', 'forward']],
    ['settings.groupSquadron', ['quit']],
  ]
  const HINTS: Partial<Record<Action, Key>> = {
    newAgent: 'settings.actionNewAgentHint',
    jump: 'settings.actionJumpHint',
    interrupt: 'settings.actionInterruptHint',
    search: 'settings.actionSearchHint',
  }
  const FIXED: [Key, [Key, Key | null, string[]][]][] = [
    [
      'settings.fixedComposer',
      [
        ['settings.fixedSend', null, ['Enter']],
        ['settings.fixedNewLine', null, ['Shift+Enter']],
        ['settings.fixedTakeBack', 'settings.fixedTakeBackHint', ['ArrowUp']],
        ['settings.fixedHistory', 'settings.fixedHistoryHint', ['ArrowUp']],
        ['settings.fixedSuggestion', 'settings.fixedSuggestionHint', ['Tab']],
        ['settings.fixedGuess', 'settings.fixedGuessHint', ['ArrowRight']],
        ['settings.fixedLink', null, ['Ctrl+Click']],
      ],
    ],
    [
      'settings.fixedEditor',
      [
        ['settings.fixedSave', null, ['Ctrl+S']],
        ['settings.fixedSpot', 'settings.fixedSpotHint', ['N', 'Shift+N']],
        ['settings.fixedComment', 'settings.fixedCommentHint', ['Ctrl+Enter']],
      ],
    ],
    [
      'settings.fixedKanban',
      [
        ['settings.fixedTask', null, ['Ctrl+Enter']],
        ['settings.fixedEditCard', null, ['Enter']],
        ['settings.fixedMoveCard', null, ['Alt+ArrowLeft', 'ArrowRight']],
        ['settings.fixedSelectCards', 'settings.fixedRangeHint', ['Ctrl+Click']],
      ],
    ],
    [
      'settings.fixedAgents',
      [
        ['settings.fixedStart', 'settings.fixedStartHint', ['Ctrl+Enter']],
        ['settings.fixedRename', null, ['F2']],
        ['settings.fixedSelectAgents', 'settings.fixedRangeHint', ['Ctrl+Click']],
        ['settings.fixedSubagent', 'settings.fixedSubagentHint', ['J', 'K']],
        ['settings.fixedSubagentFull', 'settings.fixedSubagentHint', ['F']],
      ],
    ],
  ]

  let recording = $state<Action | null>(null)
  let conflict = $state('')
  let error = $state('')
  const changed = $derived(Object.keys(settings.keys).length)

  async function save(keys: Record<string, string>) {
    error = ''
    try {
      await saveSettings({ ...$state.snapshot(settings), keys })
    } catch (e) {
      error = errorText(e)
    }
  }

  function start(id: Action | null) {
    recording = id
    conflict = ''
  }

  function reset(id: Action) {
    const { [id]: _, ...rest } = settings.keys
    save(rest)
  }

  function record(event: KeyboardEvent) {
    if (!recording) return
    event.preventDefault()
    event.stopPropagation()
    const value = combo(event)
    if (!value) return
    const other = ACTIONS.find(([id]) => id !== recording && binding(settings.keys, id) === value)
    if (other) conflict = t('settings.conflictUsed', { keys: label(value), action: t(other[1]) })
    else if (!usable(value)) conflict = t('settings.conflictTyping', { keys: label(value) })
    else {
      const { [recording]: _, ...rest } = settings.keys
      save(value === ACTIONS.find(([id]) => id === recording)![2] ? rest : { ...rest, [recording]: value })
      start(null)
    }
  }

  const name = (id: Action) => ACTIONS.find(([action]) => action === id)![1]
</script>

<svelte:window onkeydowncapture={record} />

<p class="intro">{t('settings.shortcutsIntro')}</p>

{#each SECTIONS as [title, actions] (title)}
  <h3>{t(title)}</h3>
  <div class="options">
    {#each actions as id (id)}
      <div class="option">
        <span class="label">
          <strong>{t(name(id))}</strong>
          {#if recording === id && conflict}
            <small class="bad" role="alert">{conflict}</small>
          {:else if HINTS[id]}
            <small>{t(HINTS[id])}</small>
          {/if}
        </span>
        {#if recording === id}
          <span class="listening" role="status">{t('settings.pressKeys')}</span>
          <button class="btn btn-ghost" onclick={() => start(null)}>{t('common.cancel')}</button>
        {:else}
          <button class="combo" class:custom={!!settings.keys[id]} title={t('settings.change')} onclick={() => start(id)}>
            {label(binding(settings.keys, id))}
          </button>
          {#if settings.keys[id]}
            <button class="btn btn-ghost" onclick={() => reset(id)}>{t('common.default')}</button>
          {/if}
        {/if}
      </div>
    {/each}
  </div>
{/each}

{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if changed}
  <div class="tail">
    <span>{t('settings.shortcutsChanged', { count: changed })}</span>
    <button class="btn btn-ghost" onclick={() => (start(null), save({}))}>{t('settings.shortcutsReset')}</button>
  </div>
{/if}

<div class="fixed-head">
  <strong>{t('settings.fixedTitle')}</strong>
  <small>{t('settings.fixedIntro')}</small>
</div>

{#each FIXED as [title, rows] (title)}
  <h3>{t(title)}</h3>
  <div class="options">
    {#each rows as [what, hint, keys] (what)}
      <div class="option fixed">
        <span class="label">
          <strong>{t(what)}</strong>
          {#if hint}<small>{t(hint)}</small>{/if}
        </span>
        <kbd class="combo">{keys.map(label).join(' / ')}</kbd>
      </div>
    {/each}
  </div>
{/each}

<style>
  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  h3 {
    margin: 6px 0 -8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .option + .option {
    border-top: 1px solid var(--border);
  }

  .option.fixed {
    min-height: 44px;
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .label strong {
    font-size: 13px;
    font-weight: 600;
  }

  .fixed .label strong {
    font-weight: 500;
    color: var(--text-2);
  }

  .label small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .option .btn-ghost {
    height: 30px;
  }

  .combo {
    min-width: 96px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12.5px var(--mono);
  }

  button.combo:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .combo.custom {
    box-shadow: inset 0 0 0 1px var(--accent);
    color: var(--accent-soft);
  }

  kbd.combo {
    box-shadow: none;
    background: var(--surface-2);
    color: var(--text-dim);
  }

  .listening {
    min-width: 96px;
    height: 30px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text-2);
    font-size: 12.5px;
  }

  .tail {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .tail .btn-ghost {
    height: 32px;
  }

  .fixed-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 18px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }

  .fixed-head strong {
    font-size: 13.5px;
    font-weight: 600;
  }

  .fixed-head small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .bad,
  .label small.bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
