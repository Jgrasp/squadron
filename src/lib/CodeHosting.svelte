<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { lookUpAll } from './pulls.svelte'

  type Accounts = { bitbucket: string | null; github: string | null }

  let accounts = $state<Accounts | null>(null)
  let email = $state('')
  let token = $state('')
  let error = $state('')
  let busy = $state(false)

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
  {:else}
    <p class="intro">
      Create an API token on <strong>id.atlassian.com › Security › API tokens</strong>, choose “Create API token with scopes”, then Bitbucket with
      <code>read:repository:bitbucket</code>, <code>read:pullrequest:bitbucket</code> and <code>write:pullrequest:bitbucket</code>.
    </p>
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
