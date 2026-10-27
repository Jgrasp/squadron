<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { questions } from './asking.svelte'

  const current = $derived(questions[0])
  let dialog = $state<HTMLDialogElement>()

  $effect(() => {
    if (current && dialog && !dialog.open) dialog.showModal()
  })

  function answer(yes: boolean) {
    const done = questions.shift()
    if (!questions.length) dialog?.close()
    done?.answer(yes)
  }
</script>

{#if current}
  <dialog
    bind:this={dialog}
    class={current.kind ?? 'info'}
    aria-labelledby="confirm-title"
    aria-describedby="confirm-text"
    oncancel={(e) => (e.preventDefault(), answer(false))}
  >
    <div class="head">
      <span class="icon" aria-hidden="true">
        {#if current.kind === 'warning' || current.kind === 'error'}
          <svg width="18" height="18" viewBox="0 0 24 24"><path d="M12 4 2.5 20h19zM12 10v4.5M12 17.2v.3" /></svg>
        {:else}
          <svg width="18" height="18" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M12 11v5.5M12 7.8v.3" /></svg>
        {/if}
      </span>
      <h1 id="confirm-title">{current.title ?? 'Squadron'}</h1>
    </div>
    <p id="confirm-text">{current.text}</p>
    <div class="actions">
      {#if current.choice}
        <button class="btn btn-secondary" onclick={() => answer(false)}>{current.cancelLabel ?? t('common.cancel')}</button>
      {/if}
      <!-- svelte-ignore a11y_autofocus -->
      <button class="btn btn-primary" autofocus onclick={() => answer(true)}>
        {current.okLabel ?? t('common.ok')}
      </button>
    </div>
  </dialog>
{/if}

<style>
  dialog {
    width: 440px;
    max-width: calc(100vw - 48px);
    padding: 20px 22px 18px;
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

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .warning .icon {
    background: var(--wait-btn);
    color: var(--accent-soft);
  }

  .error .icon {
    background: var(--err-bg);
    color: var(--err);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 700;
    color: var(--text-strong);
  }

  p {
    margin: 12px 0 20px 44px;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--text-2);
    white-space: pre-line;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
