<script lang="ts">
  import Icon from './Icon.svelte'
  import PersonaChip from './PersonaChip.svelte'
  import { starts } from './asking.svelte'
  import { effortLabel, modelLabel } from './format'
  import { t } from './i18n/index.svelte'

  const current = $derived(starts[0])
  const elsewhere = $derived(current && current.from.name !== current.to.name)
  const modeName = $derived(current && t(current.mode === 'auto' ? 'settings.modeAuto' : 'settings.modeAcceptEdits'))
  const modeHint = $derived(current && t(current.mode === 'auto' ? 'settings.modeAutoHint' : 'settings.modeAcceptEditsHint'))
  let dialog = $state<HTMLDialogElement>()

  $effect(() => {
    if (current && dialog && !dialog.open) dialog.showModal()
  })

  function answer(yes: boolean) {
    const done = starts.shift()
    if (!starts.length) dialog?.close()
    done?.answer(yes)
  }
</script>

{#if current}
  <dialog bind:this={dialog} aria-labelledby="start-title" oncancel={(e) => (e.preventDefault(), answer(false))}>
    <form method="dialog" onsubmit={(e) => (e.preventDefault(), answer(true))}>
      <div class="top">
        <div class="route">
          <span class="tab" style:--project={current.from.color}><span class="sq"></span>{current.from.name} <span class="by">· {current.caller}</span></span>
          {#if elsewhere}
            <svg class="arrow" width="14" height="14" viewBox="0 0 24 24" aria-label={t('shell.pilotStartTo')}><path d="M5 12h14M13 6l6 6-6 6" /></svg>
            <span class="tab" style:--project={current.to.color}><span class="sq"></span>{current.to.name}</span>
          {:else}
            <span>{t('shell.pilotStartOwn')}</span>
          {/if}
        </div>
        <h1 id="start-title" class:unnamed={!current.name}>{current.name || t('shell.pilotStartUnnamed')}</h1>
        <div class="meta">
          <span class="mode tone-{current.mode}">{modeName}</span>
          {#if current.persona}<PersonaChip name={current.persona} />{/if}
          {#if current.card}<span class="chip"><Icon name="kanban" size={12} />{current.card}</span>{/if}
          <span class="chip">{modelLabel(current.model)} · {effortLabel(current.effort)}</span>
          {#if current.worktree}<span class="chip" title={current.worktree}><Icon name="branch" size={12} />{current.worktree.split('/').pop()}</span>{/if}
        </div>
      </div>
      <div class="warn">
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4 2.5 20h19zM12 10v4.5M12 17.2v.3" /></svg>
        <span><b>{modeName}.</b> {modeHint}.</span>
      </div>
      <div class="sec">
        <span class="cap">{t('shell.pilotStartMessage')}</span>
        <div class="prompt">{current.prompt}</div>
        {#if current.goal}
          <div class="goal"><Icon name="check" size={13} /><span>{t('shell.pilotStartGoal', { goal: current.goal })}</span></div>
        {/if}
      </div>
      <div class="foot">
        <span class="keys">{t('shell.pilotStartKeys')}</span>
        <button type="button" class="btn btn-secondary" onclick={() => answer(false)}>{t('shell.pilotStartRefuse')}</button>
        <!-- svelte-ignore a11y_autofocus -->
        <button type="submit" class="btn btn-primary" autofocus>{t('shell.pilotStartOk')}</button>
      </div>
    </form>
  </dialog>
{/if}

<style>
  dialog {
    width: 460px;
    max-width: calc(100vw - 48px);
    padding: 0;
    border: none;
    border-radius: 8px;
    background: var(--bg-main);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
  }

  dialog::backdrop {
    background: var(--overlay);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .top {
    padding: 18px 20px 0;
  }

  .route {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 55%, transparent);
    color: var(--text-strong);
    font-weight: 600;
  }

  .by {
    font-weight: 400;
    color: var(--text-muted);
  }

  .sq {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project);
  }

  .arrow {
    color: var(--text-faint);
  }

  h1 {
    margin: 14px 0 0;
    font-size: 18px;
    font-weight: 700;
    line-height: 1.3;
    color: var(--text-strong);
    overflow-wrap: anywhere;
  }

  h1.unnamed {
    font-weight: 600;
    color: var(--text-muted);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }

  .mode {
    --tone: var(--mode-auto);
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--tone) 13%, transparent);
    color: var(--tone);
    font-size: 12px;
    font-weight: 600;
  }

  .mode.tone-acceptEdits {
    --tone: var(--mode-accept);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    height: 22px;
    padding: 0 7px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-3);
    font: 11.5px var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .warn {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin: 14px 20px 0;
    padding: 9px 12px;
    border-radius: 6px;
    background: var(--wait-note);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    font-size: 12.5px;
    color: var(--wait-muted);
  }

  .warn b {
    font-weight: 600;
    color: var(--wait-text);
  }

  .warn svg {
    flex: none;
    margin-top: 2px;
    color: var(--accent-soft);
  }

  .sec {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 14px 20px 0;
  }

  .cap {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .prompt {
    max-height: 132px;
    overflow: auto;
    padding: 9px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-2);
    white-space: pre-line;
    overflow-wrap: anywhere;
  }

  .goal {
    display: flex;
    gap: 8px;
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  .goal :global(.icon) {
    flex: none;
    margin-top: 3px;
    color: var(--done);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    border-radius: 0 0 8px 8px;
    background: var(--surface-1);
  }

  .keys {
    margin-right: auto;
    font-size: 11.5px;
    color: var(--text-faint);
  }
</style>
