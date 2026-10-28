<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { errorText, t } from './i18n/index.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { FIELDS, loadConnections, tickets, type Connection } from './tickets.svelte'

  type Health = { count: number } | { error: string }

  let editing = $state<{ id: string | null; name: string; site: string; email: string; token: string } | null>(null)
  let busy = $state(false)
  let error = $state('')
  let health = $state<Record<string, Health | undefined>>({})

  function check(list: Connection[]) {
    for (const c of list) {
      health[c.id] = undefined
      invoke<unknown[]>('ticket_projects', { connection: c.id })
        .then((found) => (health[c.id] = { count: found.length }))
        .catch((e) => (health[c.id] = { error: errorText(e) }))
    }
  }

  loadConnections().then(check)

  const fresh = () => ({ id: null, name: '', site: '', email: tickets.connections?.[0]?.email ?? '', token: '' })

  async function submit(event: Event) {
    event.preventDefault()
    if (!editing) return
    error = ''
    busy = true
    try {
      const { id, name, site, email, token } = editing
      tickets.connections = await invoke<Connection[]>('ticket_connect', { id, name, site, email, token })
      check(tickets.connections.filter((c) => c.id === id || !health[c.id]))
      editing = null
    } catch (e) {
      error = errorText(e)
    } finally {
      busy = false
    }
  }

  async function remove(connection: Connection) {
    tickets.connections = await invoke<Connection[]>('ticket_disconnect', { id: connection.id })
    if (editing?.id === connection.id) editing = null
  }

  const on = (field: string) => !settings.ticket_fields_off.includes(field)
  const allOn = $derived(FIELDS.every(([f]) => on(f)))

  function toggle(fields: string[], value: boolean) {
    const off = value ? settings.ticket_fields_off.filter((f) => !fields.includes(f)) : [...new Set([...settings.ticket_fields_off, ...fields])]
    saveSettings({ ...$state.snapshot(settings), ticket_fields_off: off }).catch((e) => (error = errorText(e)))
  }
</script>

<p class="intro">{t('tickets.intro')}</p>

<section>
  <h3>{t('tickets.connections')}</h3>
  {#if tickets.connections && !tickets.connections.length && !editing}
    <p class="intro">{t('tickets.noConnection')}</p>
  {/if}
  {#each tickets.connections ?? [] as connection (connection.id)}
    {@const state = health[connection.id]}
    <div class="connection">
      <span class="logo" aria-hidden="true">J</span>
      <span class="what">
        <strong>{connection.name}</strong>
        <span class="meta">
          <code>{connection.site}</code> · {connection.email} ·
          {#if !state}
            {t('common.checking')}
          {:else if 'error' in state}
            <span class="bad">● {state.error}</span>
          {:else}
            <span class="ok">● {t('tickets.connected')}</span> · {t('tickets.projectsVisible', { count: state.count })}
          {/if}
        </span>
      </span>
      <button class="btn btn-secondary" onclick={() => ((error = ''), (editing = { ...connection, token: '' }))}>{t('common.edit')}</button>
      <button class="btn btn-ghost" aria-label={t('tickets.removeConnection', { name: connection.name })} title={t('tickets.removeConnection', { name: connection.name })} onclick={() => remove(connection)}>✕</button>
    </div>
  {/each}

  {#if editing}
    <form class="form" onsubmit={submit}>
      <div class="head">
        <strong>{editing.id ? t('tickets.editConnection', { name: editing.name }) : t('tickets.newConnection')}</strong>
        <span class="providers" role="radiogroup" aria-label={t('tickets.provider')}>
          <span role="radio" aria-checked="true" class="on">Jira</span>
          <span role="radio" aria-checked="false" aria-disabled="true">{t('tickets.soon', { name: 'Monday' })}</span>
          <span role="radio" aria-checked="false" aria-disabled="true">{t('tickets.soon', { name: 'Asana' })}</span>
        </span>
      </div>
      <label>{t('tickets.name')}<input class="field" bind:value={editing.name} placeholder="Akki" /></label>
      <label>{t('tickets.site')}<input class="field mono" bind:value={editing.site} placeholder={t('tickets.sitePlaceholder')} spellcheck="false" /></label>
      <label>{t('tickets.email')}<input class="field" type="email" autocomplete="off" bind:value={editing.email} /></label>
      <label>
        {t('tickets.token')}
        <input class="field mono" type="password" autocomplete="off" spellcheck="false" bind:value={editing.token} placeholder={editing.id ? t('tickets.tokenKeep') : ''} />
      </label>
      <p class="help">
        {t('tickets.tokenHelp')}
        <button type="button" class="link" onclick={() => invoke('open_link', { url: 'https://id.atlassian.com/manage-profile/security/api-tokens' })}>{t('tickets.openTokens')}</button>
      </p>
      {#if error}<p class="bad full" role="alert">{error}</p>{/if}
      <div class="actions">
        <button type="button" class="btn btn-ghost" onclick={() => ((editing = null), (error = ''))}>{t('common.cancel')}</button>
        <button class="btn btn-primary" disabled={busy || !editing.site.trim() || !editing.email.trim() || (!editing.id && !editing.token.trim())}>
          {busy ? t('common.checking') : editing.id ? t('tickets.testAndSave') : t('tickets.testAndAdd')}
        </button>
      </div>
    </form>
  {:else}
    <div><button class="btn btn-secondary" onclick={() => ((error = ''), (editing = fresh()))}>+ {t('tickets.newConnection')}</button></div>
  {/if}
</section>

<section>
  <h3>{t('tickets.contextTitle')}</h3>
  <p class="intro">{t('tickets.contextIntro')}</p>
  <div class="fields">
    <label class="all"><input type="checkbox" checked={allOn} onchange={(e) => toggle(FIELDS.map(([f]) => f), e.currentTarget.checked)} />{t('tickets.all')}</label>
    {#each FIELDS as [field, label] (field)}
      <label><input type="checkbox" checked={on(field)} onchange={(e) => toggle([field], e.currentTarget.checked)} />{t(label)}</label>
    {/each}
  </div>
</section>

<style>
  section {
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

  .connection {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .logo {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 3px;
    background: var(--jira);
    color: #fff;
    font: 700 10px var(--sans);
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .meta code {
    font-size: 12px;
  }

  .ok {
    color: var(--ok);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  .form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    padding: 14px;
    border: 1px dashed var(--border-dashed);
    border-radius: 6px;
  }

  .form label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-weight: 600;
  }

  .form .field {
    height: 34px;
  }

  .mono {
    font: 13px var(--mono);
  }

  .head,
  .help,
  .full,
  .actions {
    grid-column: 1 / -1;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .providers {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    background: var(--bg-sidebar);
  }

  .providers span {
    padding: 4px 10px;
    border-radius: 3px;
    font-size: 12px;
    color: var(--text-faint);
  }

  .providers .on {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .help {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .link {
    color: var(--accent-link);
  }

  .link:hover {
    text-decoration: underline;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .actions .btn {
    height: 34px;
  }

  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 9px 16px;
    padding: 12px 14px;
    border-radius: 6px;
    background: var(--surface-1);
  }

  .fields label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .fields input {
    accent-color: var(--accent);
  }

  .fields .all {
    grid-column: 1 / -1;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }
</style>
