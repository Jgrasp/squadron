<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { lookUpAll } from './pulls.svelte'

  type Accounts = { bitbucket: string | null; bitbucket_user: boolean; github: string | null }

  let accounts = $state<Accounts | null>(null)
  let email = $state('')
  let token = $state('')
  let error = $state('')
  let busy = $state(false)
  let copied = $state('')
  const SCOPES = ['read:repository:bitbucket', 'read:pullrequest:bitbucket', 'write:pullrequest:bitbucket', 'read:user:bitbucket']

  function copy(scope: string) {
    navigator.clipboard.writeText(scope)
    copied = scope
    setTimeout(() => copied === scope && (copied = ''), 1500)
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
      error = String(e)
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

<p class="intro">Squadron finds the pull request of each agent's branch, shows its state and lets you review it in the Diff panel.</p>

<section>
  <h3>Bitbucket</h3>
  {#if !accounts}
    <p class="intro">Checking…</p>
  {:else if accounts.bitbucket}
    <div class="signed">
      <span>Signed in as <strong>{accounts.bitbucket}</strong> · the token stays in the system keyring</span>
      <button class="btn btn-secondary" onclick={signOut}>Sign out</button>
    </div>
    {#if !accounts.bitbucket_user}
      <p class="intro">
        This token lacks <code>read:user:bitbucket</code>, so Squadron cannot list the pull requests you review or wrote. Sign out, then sign in with a token
        that has the four scopes.
      </p>
    {/if}
  {:else}
    <ol class="intro steps">
      <li>
        <button class="btn btn-secondary" onclick={() => invoke('open_link', { url: 'https://id.atlassian.com/manage-profile/security/api-tokens' })}>
          Open the API tokens page
        </button>
      </li>
      <li>Choose <strong>Create API token with scopes</strong>, name it Squadron, then pick <strong>Bitbucket</strong>.</li>
      <li>
        Tick these four scopes. Click one to copy it, then paste it in the scope search:
        <span class="scopes">
          {#each SCOPES as scope (scope)}
            <button class="scope" onclick={() => copy(scope)}>{copied === scope ? 'Copied' : scope}</button>
          {/each}
        </span>
      </li>
      <li>Paste the token below with the email of your Atlassian account.</li>
    </ol>
    <form onsubmit={signIn}>
      <label for="bitbucket-email">Atlassian email</label>
      <input id="bitbucket-email" class="field" type="email" autocomplete="off" bind:value={email} />
      <label for="bitbucket-token">API token</label>
      <input id="bitbucket-token" class="field mono" type="password" autocomplete="off" spellcheck="false" bind:value={token} />
      <div class="row">
        <button class="btn btn-primary" disabled={busy || !email.trim() || !token.trim()}>{busy ? 'Checking…' : 'Sign in'}</button>
      </div>
    </form>
  {/if}
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
</section>

<section>
  <h3>GitHub</h3>
  {#if !accounts}
    <p class="intro">Checking…</p>
  {:else if accounts.github}
    <p class="intro">Signed in through the GitHub CLI as <strong>{accounts.github}</strong>.</p>
  {:else}
    <p class="intro">Install the GitHub CLI and run <code>gh auth login</code> in a terminal. Squadron uses its sign in.</p>
  {/if}
</section>

<style>
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
