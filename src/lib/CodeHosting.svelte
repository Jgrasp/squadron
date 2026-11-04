<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { lookUpAll, REVIEW_PROMPT } from './pulls.svelte'
  import { projects } from './projects.svelte'
  import { names } from './pipelines.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import Trans from './Trans.svelte'
  import HostLogo from './HostLogo.svelte'

  type Host = 'bitbucket' | 'gitlab' | 'azure' | 'github'
  type Accounts = { bitbucket: string | null; bitbucket_user: boolean; github: string | null; gitlab: string | null; azure: string | null }

  let accounts = $state<Accounts | null>(null)
  let email = $state('')
  let token = $state('')
  let error = $state('')
  let site = $state('')
  let organization = $state('')
  let tokens = $state({ gitlab: '', azure: '' })
  let failed = $state({ bitbucket: '', gitlab: '', azure: '' })
  let open = $state<Host | null>(null)
  const ROWS: [Host, string, Key][][] = [
    [
      ['bitbucket', 'Bitbucket', 'settings.bitbucketHint'],
      ['gitlab', 'GitLab', 'settings.gitlabHint'],
    ],
    [
      ['azure', 'Azure DevOps', 'settings.azureHint'],
      ['github', 'GitHub', 'settings.githubHint'],
    ],
  ]
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
    failed.bitbucket = ''
    busy = true
    try {
      await invoke('bitbucket_sign_in', { email, token })
      token = ''
      open = null
      await refresh()
      lookUpAll(true)
    } catch (e) {
      failed.bitbucket = errorText(e)
    } finally {
      busy = false
    }
  }

  async function connect(event: Event, host: 'gitlab' | 'azure', args: Record<string, string>) {
    event.preventDefault()
    failed[host] = ''
    busy = true
    try {
      await invoke(`${host}_sign_in`, { ...args, token: tokens[host] })
      tokens[host] = ''
      open = null
      await refresh()
      lookUpAll(true)
    } catch (e) {
      failed[host] = errorText(e)
    } finally {
      busy = false
    }
  }

  async function signOut(account = 'bitbucket') {
    await invoke('forge_sign_out', { account })
    await refresh()
    lookUpAll(true)
  }
</script>

<p class="intro">{t('settings.hostingIntro')}</p>

<div class="cards">
  {#each ROWS as row, r (r)}
    {#each row as [host, name, hint] (host)}
      {@const who = accounts?.[host]}
      <div class="card" class:selected={open === host}>
        <span class="top">
          <HostLogo {host} />
          <span><strong>{name}</strong><small>{t(hint)}</small></span>
        </span>
        <span class="foot">
          {#if !accounts}
            <span class="status">{t('common.checking')}</span>
          {:else if who}
            <span class="status on"><span class="dot"></span>{who}</span>
            {#if host !== 'github'}<button class="btn btn-ghost" onclick={() => signOut(host)}>{t('settings.signOut')}</button>{/if}
          {:else}
            <span class="status">{t('settings.notConnected')}</span>
            <button class="btn btn-ghost" aria-expanded={open === host} onclick={() => (open = open === host ? null : host)}>
              {t(host === 'github' ? 'settings.howTo' : 'settings.connect')}
            </button>
          {/if}
        </span>
      </div>
    {/each}
    {#if open && accounts && !accounts[open] && row.some(([host]) => host === open)}
      <div class="drawer">
        <div class="drawer-head">
          <HostLogo host={open} size={20} />
          <strong>{t(open === 'github' ? 'settings.githubTitle' : 'settings.connectHost', { name: row.find(([host]) => host === open)?.[1] ?? '' })}</strong>
          <button class="btn btn-ghost" onclick={() => (open = null)}>{open === 'github' ? t('common.close') : t('common.cancel')}</button>
        </div>
        {#if open === 'bitbucket'}
          <form onsubmit={signIn}>
            <ol class="steps">
              <li>
                <div>
                  <button type="button" class="btn btn-secondary" onclick={() => invoke('open_link', { url: 'https://id.atlassian.com/manage-profile/security/api-tokens' })}>
                    {t('settings.openTokens')}
                  </button>
                </div>
              </li>
              <li>
                <div>
                  <span><Trans k="settings.stepCreate">{#snippet create()}<strong>Create API token with scopes</strong>{/snippet}{#snippet bitbucket()}<strong>Bitbucket</strong>{/snippet}</Trans></span>
                </div>
              </li>
              <li>
                <div>
                  <span>{t('settings.stepScopes')}</span>
                  <span class="scopes">
                    {#each SCOPES as scope (scope)}
                      <button type="button" class="scope" onclick={() => copy(scope)}>{copied === scope ? t('common.copied') : scope}</button>
                    {/each}
                  </span>
                </div>
              </li>
              <li>
                <div>
                  <span>{t('settings.stepPaste')}</span>
                  <input class="field" type="email" autocomplete="off" placeholder={t('settings.atlassianEmail')} aria-label={t('settings.atlassianEmail')} bind:value={email} />
                  <span class="row">
                    <input class="field mono" type="password" autocomplete="off" spellcheck="false" placeholder={t('settings.apiToken')} aria-label={t('settings.apiToken')} bind:value={token} />
                    <button class="btn btn-primary" disabled={busy || !email.trim() || !token.trim()}>{busy ? t('common.checking') : t('settings.signIn')}</button>
                  </span>
                </div>
              </li>
            </ol>
          </form>
        {:else if open === 'gitlab'}
          <form onsubmit={(e) => connect(e, 'gitlab', { site })}>
            <ol class="steps">
              <li>
                <div>
                  <span>{t('settings.gitlabSite')}</span>
                  <input class="field" autocomplete="off" spellcheck="false" placeholder="https://gitlab.com" aria-label={t('settings.gitlabSite')} bind:value={site} />
                </div>
              </li>
              <li>
                <div>
                  <span><Trans k="settings.gitlabStep">{#snippet scope()}<code>api</code>{/snippet}</Trans></span>
                  <button
                    type="button"
                    class="btn btn-secondary"
                    onclick={() => invoke('open_link', { url: `${(site.trim() || 'https://gitlab.com').replace(/\/+$/, '')}/-/user_settings/personal_access_tokens?name=Squadron&scopes=api` })}
                  >
                    {t('settings.openTokens')}
                  </button>
                </div>
              </li>
              <li>
                <div>
                  <span>{t('settings.stepPasteToken')}</span>
                  <span class="row">
                    <input class="field mono" type="password" autocomplete="off" spellcheck="false" placeholder={t('settings.apiToken')} aria-label={t('settings.apiToken')} bind:value={tokens.gitlab} />
                    <button class="btn btn-primary" disabled={busy || !tokens.gitlab.trim()}>{busy ? t('common.checking') : t('settings.signIn')}</button>
                  </span>
                </div>
              </li>
            </ol>
          </form>
        {:else if open === 'azure'}
          <form onsubmit={(e) => connect(e, 'azure', { organization })}>
            <ol class="steps">
              <li>
                <div>
                  <span>{t('settings.azureOrganization')}</span>
                  <input class="field" autocomplete="off" spellcheck="false" placeholder="https://dev.azure.com/…" aria-label={t('settings.azureOrganization')} bind:value={organization} />
                </div>
              </li>
              <li>
                <div>
                  <span><Trans k="settings.azureStep">{#snippet code()}<strong>Code (Read &amp; write)</strong>{/snippet}{#snippet build()}<strong>Build (Read)</strong>{/snippet}</Trans></span>
                  <button
                    type="button"
                    class="btn btn-secondary"
                    disabled={!organization.trim()}
                    onclick={() => invoke('open_link', { url: `https://dev.azure.com/${organization.trim().replace(/\/+$/, '').split('/').at(-1)}/_usersSettings/tokens` })}
                  >
                    {t('settings.openTokens')}
                  </button>
                </div>
              </li>
              <li>
                <div>
                  <span>{t('settings.stepPasteToken')}</span>
                  <span class="row">
                    <input class="field mono" type="password" autocomplete="off" spellcheck="false" placeholder={t('settings.apiToken')} aria-label={t('settings.apiToken')} bind:value={tokens.azure} />
                    <button class="btn btn-primary" disabled={busy || !organization.trim() || !tokens.azure.trim()}>{busy ? t('common.checking') : t('settings.signIn')}</button>
                  </span>
                </div>
              </li>
            </ol>
          </form>
        {:else}
          <p class="intro"><Trans k="settings.githubHow">{#snippet command()}<code>gh auth login</code>{/snippet}</Trans></p>
        {/if}
        {#if open !== 'github'}<p class="keyring">{t('settings.keyring')}</p>{/if}
        {#if open !== 'github' && failed[open]}<p class="bad" role="alert">{failed[open]}</p>{/if}
      </div>
    {/if}
  {/each}
</div>
{#if accounts?.bitbucket && !accounts.bitbucket_user}
  <p class="intro">
    <Trans k="settings.tokenLacksScope">{#snippet scope()}<code>read:user:bitbucket</code>{/snippet}</Trans>
  </p>
{/if}
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<section>
  <h3>{t('review.promptTitle')}</h3>
  <p class="intro">{t('review.promptIntro')}</p>
  <textarea
    class="field prompt"
    rows="5"
    aria-label={t('review.promptTitle')}
    value={settings.review_prompt || REVIEW_PROMPT}
    onchange={(e) => change({ review_prompt: e.currentTarget.value.trim() === REVIEW_PROMPT ? '' : e.currentTarget.value.trim() })}
  ></textarea>
  {#if settings.review_prompt}
    <div class="row"><button class="btn btn-secondary" onclick={() => change({ review_prompt: '' })}>{t('review.promptReset')}</button></div>
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
  <label class="check">
    <input type="checkbox" checked={settings.runs_notify} disabled={!settings.runs} onchange={(e) => change({ runs_notify: e.currentTarget.checked })} />
    <span>
      <strong>{t('runs.notify')}</strong>
      <small>{t('runs.notifyHint')}</small>
    </span>
  </label>
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

  code {
    font-size: 12.5px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 10px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .card.selected {
    box-shadow: inset 0 0 0 1.5px var(--border-focus);
  }

  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .top span {
    display: flex;
    flex-direction: column;
  }

  .top small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 30px;
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 12.5px;
    color: var(--text-faint);
  }

  .status.on {
    color: var(--text-2);
  }

  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--ok);
  }

  .drawer {
    grid-column: 1 / -1;
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin: 6px 0 10px;
    padding: 18px 20px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .drawer-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .drawer-head strong {
    flex: 1;
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
    counter-reset: step;
    font-size: 13px;
    color: var(--text-muted);
  }

  .steps li {
    counter-increment: step;
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: 10px;
  }

  .steps li::before {
    content: counter(step);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 600;
  }

  .steps li > div {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding-top: 2px;
  }

  .steps li > div > :is(.field, .row) {
    align-self: stretch;
  }

  .steps strong {
    color: var(--text-2);
  }

  .keyring {
    margin: 0;
    font-size: 12px;
    color: var(--text-faint);
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

  .prompt {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    line-height: 1.5;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
