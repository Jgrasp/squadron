<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { open } from '@tauri-apps/plugin-dialog'
  import { errorText, locale, t } from './i18n/index.svelte'
  import { text, type Mod } from './mods'
  import { approve, commandLine, loadMods, mods } from './mods.svelte'
  import { home } from './projects.svelte'

  let error = $state('')

  const attempt = async (job: () => Promise<unknown>) => {
    error = ''
    try {
      await job()
      await loadMods()
    } catch (e) {
      error = errorText(e)
    }
  }

  async function add() {
    const path = await open({ directory: true, title: t('mods.addTitle') })
    if (typeof path === 'string') attempt(() => invoke('mods_link', { path }))
  }

  const enable = (m: Mod, on: boolean) => attempt(() => invoke('mods_enable', { id: m.id, on }))
  const unlink = (m: Mod) => attempt(() => invoke('mods_unlink', { path: m.dir }))
</script>

<p class="intro">{t('mods.intro')}</p>
<div class="row">
  <button class="btn btn-secondary" onclick={add}>{t('mods.add')}</button>
  <button class="btn btn-secondary" onclick={() => attempt(() => invoke('open_mods_folder'))}>{t('mods.openFolder')}</button>
</div>
{#if !mods.list.length}
  <p class="empty">{t('mods.none')}</p>
{/if}
<ul>
  {#each mods.list as m (m.dir)}
    <li class:off={!m.enabled || !m.manifest}>
      <label class="who">
        <input type="checkbox" checked={m.enabled && !!m.manifest} disabled={!m.manifest} onchange={(e) => enable(m, e.currentTarget.checked)} />
        <span>
          <strong>{m.manifest ? text(m.manifest.name, locale()) : m.id}{#if m.manifest}<span class="version">{m.manifest.version}</span>{/if}</strong>
          {#if m.manifest?.description}<small>{text(m.manifest.description, locale())}</small>{/if}
          <small class="mono">{home(m.dir)}{m.linked ? ` · ${t('mods.linked')}` : ''}</small>
          {#each m.problems as [key, values], i (i)}
            <small class="bad">{t(key, values)}</small>
          {/each}
          {#if m.enabled && mods.errors[m.id]}<small class="bad">{mods.errors[m.id]}</small>{/if}
          {#if m.manifest?.contributes?.tools?.length}
            <small>{t('mods.tools', { tools: m.manifest.contributes.tools.map((tool) => `${m.id}_${tool.name}`).join(', ') })}</small>
          {/if}
        </span>
      </label>
      {#if m.linked}
        <button class="remove" onclick={() => unlink(m)}>{t('common.remove')}</button>
      {/if}
    </li>
    {#if m.manifest && m.pending.length}
      <li class="ask">
        <span class="who">
          <strong>{t('mods.pending', { name: text(m.manifest.name, locale()) })}</strong>
          <small>{t('mods.pendingHint')}</small>
          {#each m.commands as command (command)}
            <code class:new={m.pending.includes(command)}>{commandLine(command)}</code>
          {/each}
        </span>
        <button class="btn btn-primary" onclick={() => attempt(() => approve(m))}>{t('mods.allow')}</button>
      </li>
    {/if}
  {/each}
</ul>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

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
</style>
