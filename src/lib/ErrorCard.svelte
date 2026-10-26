<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { reload, retry } from './agents.svelte'
  import { time } from './format'
  import { t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'
  import type { Agent, Entry } from './stream'

  let { agent, entry }: { agent: Agent; entry: Extract<Entry, { kind: 'error' }> } = $props()

  let path = $state('')
  let copied = $state(false)

  async function copy(text: string) {
    await navigator.clipboard.writeText(text)
    copied = true
    setTimeout(() => (copied = false), 1500)
  }

  async function browse() {
    const chosen = await open({ directory: false })
    if (typeof chosen === 'string') path = chosen
  }

  async function useClaude() {
    await invoke('set_claude_path', { path })
    retry(agent.id)
  }
</script>

<section class="card" role="alert" aria-labelledby="error-{entry.at}">
  {#if entry.missing}
    <div class="title">
      <span class="icon" aria-hidden="true">
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M4 17l6-6-6-6M12 19h8" /></svg>
      </span>
      <h2 id="error-{entry.at}">{t('agent.claudeNotFound')}</h2>
    </div>
    <p><Trans k="agent.claudeNotInPath">{#snippet command()}<code>claude</code>{/snippet}</Trans></p>
    <label for="path-{entry.at}">{t('agent.executablePath')}</label>
    <div class="row">
      <input id="path-{entry.at}" class="field" bind:value={path} placeholder="/home/user/.local/bin/claude" spellcheck="false" />
      <button class="btn btn-danger" onclick={browse}>{t('common.browse')}</button>
    </div>
    <div class="row">
      <button class="btn btn-primary" onclick={useClaude}>{t('common.retry')}</button>
      <a href="https://docs.claude.com/en/docs/claude-code/setup" target="_blank" rel="noreferrer">{t('agent.installClaudeCode')}</a>
    </div>
  {:else if entry.login}
    <div class="title">
      <span class="icon" aria-hidden="true">
        <svg width="16" height="16" viewBox="0 0 24 24"><rect x="5" y="11" width="14" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
      </span>
      <h2 id="error-{entry.at}">{t('agent.notLoggedIn')}</h2>
    </div>
    <p>{t('agent.loginHint')}</p>
    <pre><span>$</span> claude auth login</pre>
    <div class="row">
      <button class="btn btn-primary" onclick={() => copy('claude auth login')}>{copied ? t('common.copied') : t('agent.copyCommand')}</button>
      <button class="btn btn-danger" onclick={() => retry(agent.id)}>{t('common.retry')}</button>
      <span class="raw">{entry.message}</span>
    </div>
  {:else}
    <div class="title">
      <span class="icon" aria-hidden="true">
        <svg width="16" height="16" viewBox="0 0 24 24"><path d="M12 3l9 16H3zM12 10v4M12 17h.01" /></svg>
      </span>
      <span class="head">
        <h2 id="error-{entry.at}">{t('agent.sessionStopped')}</h2>
        <small>{time(entry.at)}</small>
      </span>
    </div>
    <pre>{entry.message}</pre>
    <p>{t('agent.sessionStoppedHint')}</p>
    <div class="row">
      <button class="btn btn-primary" onclick={() => reload(agent.id)}>{t('common.reload')}</button>
      <button class="btn btn-danger" onclick={() => copy(entry.message)}>{copied ? t('common.copied') : t('agent.copyError')}</button>
    </div>
  {/if}
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: 6px;
    background: var(--err-bg);
    box-shadow: 0 0 0 1px var(--err-border);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-strong);
  }

  small {
    font-size: 12.5px;
    color: var(--err-muted);
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--err);
    color: var(--err-ink);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--err-muted);
  }

  code {
    padding: 1px 5px;
    border-radius: 3px;
    background: #3a2420;
    color: #f8d5cc;
    font-size: 12.5px;
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--err-pre);
    font-size: 12px;
    line-height: 1.6;
    color: var(--err-text);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  pre span {
    color: #c99a90;
  }

  label {
    font-size: 12.5px;
    font-weight: 600;
    color: #f8d5cc;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .field {
    height: 38px;
    background: var(--err-pre);
    box-shadow: inset 0 0 0 1px #6b3a33;
    font: 12.5px var(--mono);
  }

  .btn {
    height: 36px;
  }

  .btn-danger {
    background: #3a2420;
    color: #f8d5cc;
  }

  a {
    font-size: 12.5px;
  }

  .raw {
    flex: 1;
    text-align: right;
    font: 11.5px var(--mono);
    color: #c99a90;
  }
</style>
