<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app'
  import { invoke } from '@tauri-apps/api/core'
  import { agents } from './agents.svelte'
  import { account } from './catalogs.svelte'
  import { projects } from './projects.svelte'
  import { settings } from './settings.svelte'
  import { checkNow, install, update } from './update.svelte'
  import { t } from './i18n/index.svelte'

  let version = $state('')
  let claude = $state('')
  getVersion().then((v) => (version = v)).catch(() => {})
  invoke<string>('claude_version', { path: settings.claude }).then((v) => (claude = v)).catch(() => {})
</script>

<div class="head">
  <span class="triangle" aria-hidden="true"></span>
  <span>
    <strong>Squadron</strong>
    <small>{t('settings.versionLine', { version: version || '…', system: navigator.platform.startsWith('Mac') ? 'macOS' : navigator.platform.startsWith('Win') ? 'Windows' : 'Linux' })}</small>
  </span>
</div>
<p>{t('settings.aboutText')}</p>
<dl>
  <dt>Claude Code</dt>
  <dd>{claude || t('settings.notFound')}</dd>
  <dt>{t('settings.account')}</dt>
  <dd>{account.value ? [account.value.email, account.value.organization, account.value.subscriptionType].filter(Boolean).join(' · ') : t('settings.accountUnknown')}</dd>
  <dt>{t('settings.projects')}</dt>
  <dd>{projects.list.length}</dd>
  <dt>{t('settings.agents')}</dt>
  <dd>{agents.list.length}</dd>
  <dt>{t('settings.updates')}</dt>
  <dd class="updates">
    {#if update.found}
      <span>{t('settings.updateAvailable', { version: update.found.version })}</span>
      <button class="btn btn-primary" disabled={update.progress !== null} onclick={install}>{t('settings.update')}</button>
    {:else}
      <span>{t('settings.checkedEvery', { count: 10 })}</span>
      <button class="btn btn-secondary" disabled={update.checking} onclick={checkNow}>{update.checking ? t('common.checking') : t('settings.itemUpdates')}</button>
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
