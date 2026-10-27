<script lang="ts">
  import { asked, reply } from './askpass.svelte'
  import { find } from './agents.svelte'
  import { t } from './i18n/index.svelte'
  import { projects } from './projects.svelte'

  const current = $derived(asked.list[0])
  const agent = $derived(current && find(current.agent))
  const hidden = $derived(!current || /pass|PIN|secret|token/i.test(current.prompt))
  let dialog = $state<HTMLDialogElement>()
  let secret = $state('')
  let remember = $state(false)

  $effect(() => {
    if (current && dialog && !dialog.open) dialog.showModal()
    if (!current && dialog?.open) dialog.close()
  })

  $effect(() => {
    void current?.id
    secret = ''
    remember = false
  })

  function answer(value: string | null) {
    if (current) reply(current, value, remember)
  }
</script>

{#if current}
  <dialog bind:this={dialog} aria-labelledby="password-title" oncancel={(e) => (e.preventDefault(), answer(null))}>
    <form method="dialog" onsubmit={(e) => (e.preventDefault(), answer(secret))}>
      <div class="head">
        <span class="icon" aria-hidden="true">
          <svg width="16" height="16" viewBox="0 0 24 24"><rect x="5" y="10.5" width="14" height="10" rx="2" /><path d="M8.5 10.5V7.5a3.5 3.5 0 0 1 7 0v3" /></svg>
        </span>
        <div>
          <h1 id="password-title">{hidden ? t('shell.passwordAsked') : t('shell.answerAsked')}</h1>
          <p class="who">{t('shell.passwordFrom', { agent: agent?.name ?? '?', project: projects.list.find((p) => p.path === agent?.project)?.name ?? '' })}</p>
        </div>
      </div>
      {#if current.command}
        <div class="command"><small>{t('shell.runningNow')}</small>{current.command}</div>
      {/if}
      <label class="prompt" for="password-input">{current.prompt.trim()}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input id="password-input" type={hidden ? 'password' : 'text'} bind:value={secret} autocomplete="off" spellcheck="false" autofocus />
      {#if hidden}
        <label class="remember"><input type="checkbox" bind:checked={remember} />{t('shell.rememberFive')}</label>
      {/if}
      <div class="foot">
        <span>{t('shell.neverSent')}</span>
        <div class="actions">
          <button type="button" class="btn btn-secondary" onclick={() => answer(null)}>{t('shell.refuse')}</button>
          <button type="submit" class="btn btn-primary">{t('shell.validate')}</button>
        </div>
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
    border-radius: 10px;
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
    gap: 12px;
    padding: 18px 20px 4px;
  }

  .icon {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-strong);
  }

  .who {
    margin: 2px 0 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .command {
    margin: 12px 20px;
    padding: 9px 12px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--surface-1);
    font: 12.5px var(--mono);
    color: var(--text-strong);
    overflow-wrap: anywhere;
  }

  .command small {
    display: block;
    margin-bottom: 3px;
    font: 11px var(--sans);
    color: var(--text-faint);
  }

  .prompt {
    display: block;
    margin: 12px 20px 6px;
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  #password-input {
    width: calc(100% - 40px);
    margin: 0 20px;
    padding: 9px 12px;
    border: 1px solid var(--accent);
    border-radius: 7px;
    outline: none;
    background: var(--surface-1);
    color: var(--text-strong);
    font: 13px var(--mono);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .remember {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 16px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    background: var(--surface-1);
    font-size: 11.5px;
    color: var(--text-faint);
  }

  .actions {
    display: flex;
    gap: 8px;
  }
</style>
