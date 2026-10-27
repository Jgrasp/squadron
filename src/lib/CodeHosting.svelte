<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { lookUpAll } from './pulls.svelte'
  import { projects } from './projects.svelte'
  import { names } from './pipelines.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'

  type Accounts = { bitbucket: string | null; bitbucket_user: boolean; github: string | null }

  let accounts = $state<Accounts | null>(null)
  let email = $state('')
  let token = $state('')
  let error = $state('')
  let busy = $state(false)
  let copied = $state('')
  const SCOPES = ['read:repository:bitbucket', 'read:pullrequest:bitbucket', 'write:pullrequest:bitbucket', 'read:user:bitbucket', 'read:pipeline:bitbucket']

  function copy(scope: string) {
    navigator.clipboard.writeText(scope)
    copied = scope
    setTimeout(() => copied === scope && (copied = ''), 1500)
  }

  const LINGERS = [1, 2, 5, 10, 15, 30, 60]
  let hide = $state('')
  const seen = $derived([...new Set(projects.list.flatMap(names))].filter((n) => !settings.runs_hidden.includes(n)).sort())
  const change = (changes: Partial<typeof settings>) => saveSettings({ ...$state.snapshot(settings), ...changes }).catch((e) => (error = errorText(e)))

  function hideName(event: Event) {
    event.preventDefault()
    const name = hide.trim()
    if (name && !settings.runs_hidden.includes(name)) change({ runs_hidden: [...settings.runs_hidden, name] })
    hide = ''
  }

  const refresh = () => invoke<Accounts>('forge_accounts').then((found) => (accounts = found))
  refresh()

  async function signIn(event: Event) {
    event.preventDefault()
    error = ''
    busy = true
    try {
      await invoke('bitbucket_sign_in', { email, token })
      token = ''
      await refresh()
      lookUpAll(true)
    } catch (e) {
      error = errorText(e)
    } finally {
      busy = false
    }
  }

  async function signOut() {
    await invoke('bitbucket_sign_out')
    await refresh()
    lookUpAll(true)
  }
</script>

<p class="intro">{t('settings.hostingIntro')}</p>

<section>
  <h3>Bitbucket</h3>
  {#if !accounts}
    <p class="intro">{t('common.checking')}</p>
  {:else if accounts.bitbucket}
    <div class="signed">
      <span><Trans k="settings.bitbucketSignedIn">{#snippet email()}<strong>{accounts?.bitbucket}</strong>{/snippet}</Trans></span>
      <button class="btn btn-secondary" onclick={signOut}>{t('settings.signOut')}</button>
    </div>
    {#if !accounts.bitbucket_user}
      <p class="intro">
        <Trans k="settings.tokenLacksScope">{#snippet scope()}<code>read:user:bitbucket</code>{/snippet}</Trans>
      </p>
    {/if}
  {:else}
    <ol class="intro steps">
      <li>
        <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: 'https://id.atlassian.com/manage-profile/security/api-tokens' })}>
          {t('settings.openTokens')}
        </button>
      </li>
      <li>
        <Trans k="settings.stepCreate">{#snippet create()}<strong>Create API token with scopes</strong>{/snippet}{#snippet bitbucket()}<strong>Bitbucket</strong>{/snippet}</Trans>
      </li>
      <li>
        {t('settings.stepScopes')}
        <span class="scopes">
          {#each SCOPES as scope (scope)}
            <button class="scope" onclick={() => copy(scope)}>{copied === scope ? t('common.copied') : scope}</button>
          {/each}
        </span>
      </li>
      <li>{t('settings.stepPaste')}</li>
    </ol>
    <form onsubmit={signIn}>
      <label for="bitbucket-email">{t('settings.atlassianEmail')}</label>
      <input id="bitbucket-email" class="field" type="email" autocomplete="off" bind:value={email} />
      <label for="bitbucket-token">{t('settings.apiToken')}</label>
      <input id="bitbucket-token" class="field mono" type="password" autocomplete="off" spellcheck="false" bind:value={token} />
      <div class="row">
        <button class="btn btn-primary" disabled={busy || !email.trim() || !token.trim()}>{busy ? t('common.checking') : t('settings.signIn')}</button>
      </div>
    </form>
  {/if}
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
</section>

<section>
  <h3>GitHub</h3>
  {#if !accounts}
    <p class="intro">{t('common.checking')}</p>
  {:else if accounts.github}
    <p class="intro"><Trans k="settings.githubSignedIn">{#snippet login()}<strong>{accounts?.github}</strong>{/snippet}</Trans></p>
  {:else}
    <p class="intro"><Trans k="settings.githubHow">{#snippet command()}<code>gh auth login</code>{/snippet}</Trans></p>
  {/if}
</section>

<section>
  <h3>{t('runs.title')}</h3>
  <p class="intro">{t('runs.settingsIntro')}</p>
  <label class="check">
    <input type="checkbox" checked={settings.runs} onchange={(e) => change({ runs: e.currentTarget.checked })} />
    <span>
      <strong>{t('runs.show')}</strong>
      <small>{t('runs.showHint')}</small>
    </span>
  </label>
  <label for="runs-linger">{t('runs.linger')}</label>
  <select id="runs-linger" class="field" disabled={!settings.runs} value={settings.runs_linger} onchange={(e) => change({ runs_linger: Number(e.currentTarget.value) })}>
    {#each LINGERS as minutes (minutes)}<option value={minutes}>{t('runs.lingerMinutes', { count: minutes })}</option>{/each}
  </select>
  <span class="label">{t('runs.hiddenEverywhere')}</span>
  {#if settings.runs_hidden.length}
    <span class="names">
      {#each settings.runs_hidden as name (name)}
        <button class="name" aria-label={t('runs.unhide', { name })} title={t('runs.unhide', { name })} onclick={() => change({ runs_hidden: settings.runs_hidden.filter((n) => n !== name) })}>
          {name}<span aria-hidden="true">×</span>
        </button>
      {/each}
    </span>
  {:else}
    <p class="intro">{t('runs.hiddenNone')}</p>
  {/if}
  <form class="row" onsubmit={hideName}>
    <input class="field" list="runs-seen" placeholder={t('runs.hideName')} aria-label={t('runs.hideName')} bind:value={hide} />
    <datalist id="runs-seen">{#each seen as name (name)}<option value={name}></option>{/each}</datalist>
    <button class="btn btn-secondary" disabled={!hide.trim()}>{t('common.add')}</button>
  </form>
</section>

<style>
  .check {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .check input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .check span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check strong {
    font-weight: 600;
  }

  .check small {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-dim);
  }

  .label {
    font-weight: 600;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .name:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  form.row {
    flex-direction: row;
  }

  .row .field {
    flex: 1;
    height: 34px;
  }

  .row .btn {
    height: 34px;
    padding: 0 14px;
  }

  section,
  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h3 {
    margin: 10px 0 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .intro strong {
    color: var(--text-2);
  }

  code {
    font-size: 12.5px;
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-left: 18px;
  }

  .scopes {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }

  .scope {
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font: 12.5px var(--mono);
  }

  .scope:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  label {
    font-weight: 600;
  }

  .mono {
    font: 13px var(--mono);
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .signed {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .signed span {
    flex: 1;
    color: var(--text-2);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
