<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { settleSecret, SUDO } from './agents.svelte'
  import { t } from './i18n/index.svelte'
  import { home } from './projects.svelte'
  import type { Agent, Ask } from './stream'

  let { agent, ask }: { agent: Agent; ask: Ask } = $props()

  const sudo = $derived(ask.tool_name === SUDO)
  const title = $derived(sudo ? t('shell.sudoTitle') : t('shell.sshKeyTitle'))
  let secret = $state('')
  let error = $state('')
  let busy = $state(false)
  let needed = $state(true)
  let user = $state('')

  $effect(() => {
    if (!sudo) return
    invoke<{ needed: boolean; user: string }>('sudo_needs_password')
      .then((r) => ((needed = r.needed), (user = r.user)))
      .catch((e) => (error = String(e)))
  })

  function explain(e: unknown) {
    const text = String(e)
    if (text === 'Wrong password.') return t('shell.wrongPassword')
    if (text === 'Wrong passphrase.') return t('shell.wrongPassphrase')
    return text
  }

  async function go() {
    if (busy || (needed && !secret)) return
    busy = true
    error = ''
    try {
      if (sudo) {
        const id = await invoke<number>('sudo_run', { dir: ask.input.dir, command: ask.input.command, password: needed ? secret : null })
        settleSecret(agent, ask, id)
      } else {
        await invoke('ssh_add_key', { path: ask.input.path, passphrase: secret })
        settleSecret(agent, ask, true)
      }
    } catch (e) {
      error = explain(e)
    } finally {
      secret = ''
      busy = false
    }
  }
</script>

<section class="card" aria-label={title}>
  <div class="head">
    <span class="icon" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 24 24"><rect x="5" y="10.5" width="14" height="10" rx="2" /><path d="M8.5 10.5V7.5a3.5 3.5 0 0 1 7 0v3" /></svg>
    </span>
    <span class="what">
      <strong>{title}</strong>
      <span class="mono">{home(sudo ? ask.input.dir : ask.input.path)}</span>
    </span>
  </div>

  {#if sudo && ask.input.reason}
    <p class="hint">{t('shell.agentSays')} <em>{ask.input.reason}</em></p>
  {/if}

  {#if sudo}
    <div class="command">
      <small>{t('shell.sudoExactly')}</small>
      <code><b>sudo</b> {ask.input.command}</code>
    </div>
  {/if}

  <form class="secret" onsubmit={(e) => (e.preventDefault(), go())}>
    {#if needed}
      <label for="secret-{ask.id}">
        {#if sudo}{t('shell.passwordFor')} <b>{user}</b>{:else}{t('shell.passphraseOf')}{/if}
        <span>{sudo ? t('shell.toSudoOnly') : t('shell.toSshAgentOnly')}</span>
      </label>
    {/if}
    <div class="actions">
      {#if needed}
        <span class="input" class:invalid={error}>
          <!-- svelte-ignore a11y_autofocus -->
          <input id="secret-{ask.id}" type="password" bind:value={secret} autocomplete="off" spellcheck="false" autofocus aria-invalid={!!error} />
        </span>
      {/if}
      <button type="submit" class="btn run" disabled={busy || (needed && !secret)}>{sudo ? t('common.run') : t('shell.load')}</button>
      <button type="button" class="btn refuse" onclick={() => settleSecret(agent, ask, '')}>{t('agent.deny')}</button>
    </div>
    {#if error}
      <span class="error" role="alert">
        <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16.5v.01" /></svg>
        {error}
      </span>
    {/if}
  </form>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .what strong {
    font-weight: 600;
    color: var(--wait-text);
  }

  .what .mono {
    font: 12.5px var(--mono);
    color: var(--wait-text-2);
    overflow-wrap: anywhere;
  }

  .hint {
    margin: -4px 0 0 40px;
    font-size: 12.5px;
    color: var(--wait-muted);
  }

  .hint em {
    font-style: normal;
    color: var(--wait-text-2);
  }

  .command {
    padding: 10px 14px;
    border-radius: 4px;
    background: var(--bg-app);
    box-shadow: inset 0 0 0 1px #4a2e29;
  }

  .command small {
    display: block;
    margin-bottom: 4px;
    font-size: 11px;
    color: var(--text-dim);
  }

  .command code {
    display: block;
    font: 12.5px/1.6 var(--mono);
    color: var(--text-strong);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .command b {
    font-weight: 500;
    color: var(--accent-soft);
  }

  .secret {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 12.5px;
    color: var(--wait-text-2);
  }

  label b {
    font: 500 12.5px var(--mono);
    color: var(--wait-text);
  }

  label span {
    margin-left: auto;
    color: var(--wait-muted);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .actions .btn {
    height: 36px;
  }

  .input {
    flex: 1;
    display: flex;
    border-radius: 4px;
    background: var(--option-off);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .input:focus-within {
    box-shadow: inset 0 0 0 2px var(--accent-soft);
  }

  .input.invalid {
    box-shadow: inset 0 0 0 2px var(--err);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 12px;
    border: 0;
    outline: none;
    background: none;
    color: var(--wait-text);
    font: 13px var(--mono);
  }

  .run {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .refuse {
    background: var(--surface-3);
    color: #e3d7cf;
  }

  .error {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--err-text);
  }
</style>
