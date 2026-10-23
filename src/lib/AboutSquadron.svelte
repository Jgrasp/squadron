<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { agents } from './agents.svelte'
  import { account } from './catalogs.svelte'
  import { projects } from './projects.svelte'
  import { settings } from './settings.svelte'
  import { checkNow, install, update } from './update.svelte'

  let version = $state('')
  let claude = $state('')
  getVersion().then((v) => (version = v)).catch(() => {})
  invoke<string>('claude_version', { path: settings.claude }).then((v) => (claude = v)).catch(() => {})
</script>

<div class="head">
  <span class="triangle" aria-hidden="true"></span>
  <span>
    <strong>Squadron</strong>
    <small>Version {version || '…'} · {navigator.platform.toLowerCase().includes('linux') ? 'Linux' : navigator.platform}</small>
  </span>
</div>
<p>Squadron runs several Claude Code agents per project, shows their conversations side by side and tells you when one of them needs you.</p>
<dl>
  <dt>Claude Code</dt>
  <dd>{claude || 'Not found'}</dd>
  <dt>Account</dt>
  <dd>{account.value ? [account.value.email, account.value.organization, account.value.subscriptionType].filter(Boolean).join(' · ') : 'Known once an agent has started'}</dd>
  <dt>Projects</dt>
  <dd>{projects.list.length}</dd>
  <dt>Agents</dt>
  <dd>{agents.list.length}</dd>
  <dt>Updates</dt>
  <dd class="updates">
    {#if update.found}
      <span>Squadron {update.found.version} is available</span>
      <button class="btn btn-primary" disabled={update.progress !== null} onclick={install}>Update</button>
    {:else}
      <span>Checked every 10 minutes</span>
      <button class="btn btn-secondary" disabled={update.checking} onclick={checkNow}>{update.checking ? 'Checking…' : 'Check for updates'}</button>
    {/if}
  </dd>
</dl>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }

  .head span:last-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .head strong {
    font-size: 16px;
  }

  small {
    color: var(--text-dim);
  }

  .triangle {
    width: 28px;
    height: 28px;
    background: var(--accent);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  p {
    margin: 0 0 16px;
    color: var(--text-muted);
    line-height: 1.6;
  }

  dl {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 8px 12px;
    margin: 0;
  }

  dt {
    color: var(--text-dim);
  }

  .updates {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .updates .btn {
    height: 32px;
    padding: 0 12px;
  }

  dd {
    margin: 0;
  }
</style>
