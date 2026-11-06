<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { errorText, locale, t } from './i18n/index.svelte'
  import { onMount } from 'svelte'
  import { installedOf, newer, text, type CatalogEntry, type Extension } from './extensions'
  import { approve, asks, catalog, commandLine, install, loadCatalog, loadExtensions, extensions, uninstall } from './extensions.svelte'
  import { home } from './projects.svelte'

  let error = $state('')

  const attempt = async (job: () => Promise<unknown>) => {
    error = ''
    try {
      await job()
      await loadExtensions()
    } catch (e) {
      error = errorText(e)
    }
  }

  async function add() {
    const path = await open({ directory: true, title: t('extensions.addTitle') })
    if (typeof path === 'string') attempt(() => invoke('extensions_link', { path }))
  }

  const enable = (m: Extension, on: boolean) => attempt(() => invoke('extensions_enable', { id: m.id, on }))
  const unlink = (m: Extension) => attempt(() => invoke('extensions_unlink', { path: m.dir }))

  let busy = $state('')

  onMount(loadCatalog)

  async function get(entry: CatalogEntry) {
    busy = entry.id
    await attempt(() => install(entry))
    busy = ''
  }

  let set = $state<Record<string, string[]>>({})
  let typed = $state<Record<string, string>>({})

  $effect(() => {
    for (const m of extensions.list) {
      const names = (m.manifest?.contributes?.secrets ?? []).map((s) => s.id)
      if (names.length) invoke<string[]>('extensions_secrets', { id: m.id, names }).then((found) => (set[m.id] = found), (e) => (error = errorText(e)))
    }
  })

  async function keep(m: Extension, name: string, value: string | null) {
    await attempt(() => invoke('extensions_secret_set', { id: m.id, name, value }))
    if (!error) {
      set[m.id] = value === null ? (set[m.id] ?? []).filter((n) => n !== name) : [...new Set([...(set[m.id] ?? []), name])]
      typed[`${m.id}.${name}`] = ''
    }
  }
</script>

<p class="intro">{t('extensions.intro')}</p>
<div class="row">
  <button class="btn btn-secondary" onclick={add}>{t('extensions.add')}</button>
  <button class="btn btn-secondary" onclick={() => attempt(() => invoke('open_extensions_folder'))}>{t('extensions.openFolder')}</button>
</div>
{#if !extensions.list.length}
  <p class="empty">{t('extensions.none')}</p>
{/if}
<ul>
  {#each extensions.list as m (m.dir)}
    <li class:off={!m.enabled || !m.manifest}>
      <label class="who">
        <input type="checkbox" checked={m.enabled && !!m.manifest} disabled={!m.manifest} onchange={(e) => enable(m, e.currentTarget.checked)} />
        <span>
          <strong>{m.manifest ? text(m.manifest.name, locale()) : m.id}{#if m.manifest}<span class="version">{m.manifest.version}</span>{/if}</strong>
          {#if m.manifest?.description}<small>{text(m.manifest.description, locale())}</small>{/if}
          <small class="mono">{home(m.dir)}{m.linked ? ` · ${t('extensions.linked')}` : ''}</small>
          {#each m.problems as [key, values], i (i)}
            <small class="bad">{t(key, values)}</small>
          {/each}
          {#if m.enabled && extensions.errors[m.id]}<small class="bad">{extensions.errors[m.id]}</small>{/if}
          {#if m.manifest?.contributes?.tools?.length}
            <small>{t('extensions.tools', { tools: m.manifest.contributes.tools.map((tool) => `${m.id}_${tool.name}`).join(', ') })}</small>
          {/if}
        </span>
      </label>
      {#if m.linked}
        <button class="remove" onclick={() => unlink(m)}>{t('common.remove')}</button>
      {:else}
        <button class="remove" onclick={() => attempt(() => uninstall(m))}>{t('extensions.uninstall')}</button>
      {/if}
    </li>
    {#if m.enabled && m.manifest?.contributes?.secrets?.length}
      <li class="secrets">
        <small>{t('extensions.secretHint')}</small>
        {#each m.manifest.contributes.secrets as s (s.id)}
          {@const key = `${m.id}.${s.id}`}
          {@const saved = set[m.id]?.includes(s.id)}
          <form onsubmit={(e) => (e.preventDefault(), keep(m, s.id, typed[key]))}>
            <span>
              <strong>{text(s.label, locale())}</strong>
              <small>{saved ? t('extensions.secretSet') : t('extensions.secretUnset')}{s.hint ? ` · ${text(s.hint, locale())}` : ''}</small>
            </span>
            <input class="field mono" type="password" autocomplete="off" spellcheck="false" aria-label={text(s.label, locale())} bind:value={typed[key]} />
            <button class="btn btn-primary" disabled={!typed[key]?.trim()}>{t('common.save')}</button>
            {#if saved}
              <button type="button" class="remove" onclick={() => keep(m, s.id, null)}>{t('common.remove')}</button>
            {/if}
          </form>
        {/each}
      </li>
    {/if}
    {#if m.manifest && m.pending.length}
      <li class="ask">
        <span class="who">
          <strong>{t('extensions.pending', { name: text(m.manifest.name, locale()) })}</strong>
          <small>{t('extensions.pendingHint')}</small>
          {#each m.commands as command (command)}
            <code class:new={m.pending.includes(command)}>{commandLine(command)}</code>
          {/each}
        </span>
        <button class="btn btn-primary" onclick={() => attempt(() => approve(m))}>{t('extensions.allow')}</button>
      </li>
    {/if}
  {/each}
</ul>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<h3>{t('extensions.catalog')}</h3>
{#if catalog.error}
  <p class="bad">{catalog.error}</p>
{:else if !catalog.list}
  <p class="empty">{t('extensions.catalogLoading')}</p>
{:else if !catalog.list.length}
  <p class="empty">{t('extensions.catalogEmpty')}</p>
{/if}
<ul>
  {#each catalog.list ?? [] as entry (entry.id)}
    {@const here = installedOf(extensions.list, entry.id)}
    <li>
      <span class="who">
        <span>
          <strong>{text(entry.name, locale())}<span class="version">{entry.version}</span></strong>
          {#if entry.description}<small>{text(entry.description, locale())}</small>{/if}
          {#if asks(entry).length}<small>{t('extensions.asks', { list: asks(entry).join(' · ') })}</small>{/if}
        </span>
      </span>
      {#if here?.manifest && !newer(entry.version, here.manifest.version)}
        <small class="done">{t('extensions.installed')}</small>
      {:else}
        <button class="btn btn-primary" disabled={!!busy} onclick={() => get(entry)}>
          {busy === entry.id ? t('extensions.installing') : here ? t('extensions.update', { version: entry.version }) : t('extensions.install')}
        </button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .intro,
  .empty {
    margin: 0 0 14px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .row {
    display: flex;
    gap: 8px;
    margin-bottom: 14px;
  }

  .row .btn {
    height: 30px;
    padding: 0 12px;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    cursor: pointer;
  }

  .who input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .who > span {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .off strong {
    color: var(--text-muted);
  }

  .version {
    margin-left: 6px;
    font: 400 12px var(--mono);
    color: var(--text-dim);
  }

  small {
    color: var(--text-dim);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--mono);
  }

  .ask {
    margin-top: -4px;
    background: var(--wait-bg);
    border: 1px solid var(--wait-border);
    color: var(--wait-text);
  }

  .ask small {
    color: var(--wait-muted);
  }

  .ask .who {
    flex-direction: column;
    gap: 4px;
    cursor: default;
  }

  code {
    align-self: flex-start;
    padding: 2px 6px;
    border-radius: 3px;
    font: 12px var(--mono);
    background: var(--wait-note);
    opacity: 0.7;
  }

  code.new {
    opacity: 1;
  }

  .ask .btn {
    height: 30px;
    padding: 0 12px;
  }

  .remove {
    height: 30px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--err-text);
  }

  .remove:hover {
    background: var(--err-bg);
  }

  .bad {
    color: var(--err-text);
  }

  h3 {
    margin: 24px 0 10px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
  }

  li > .btn {
    height: 30px;
    padding: 0 12px;
  }

  .done {
    align-self: center;
  }

  .secrets {
    margin-top: -4px;
    flex-direction: column;
    gap: 8px;
  }

  .secrets form {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .secrets form > span {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .secrets .field {
    width: 220px;
    height: 30px;
  }

  .secrets .btn {
    height: 30px;
    padding: 0 12px;
  }
</style>
