<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { loadServers, mcp } from './ondemand.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let error = $state('')
  let busy = $state('')

  const refresh = () => loadServers().catch((e) => (error = errorText(e)))
  refresh()

  async function move(command: 'mcp_on_demand' | 'mcp_everywhere', name: string) {
    error = ''
    busy = name
    try {
      await invoke(command, { name })
    } catch (e) {
      error = errorText(e)
    } finally {
      busy = ''
      refresh()
    }
  }

  function describe(name: string, description: string) {
    invoke('mcp_describe', { name, description }).catch((e) => (error = errorText(e)))
  }
</script>

<p class="intro">{t('mcp.intro')}</p>

<h3>{t('mcp.onDemand')}</h3>
{#if mcp.loaded && !mcp.onDemand.length}
  <p class="empty">{t('mcp.noneOnDemand')}</p>
{/if}
<ul>
  {#each mcp.onDemand as server (server.name)}
    <li class="stacked">
      <div class="row">
        <span class="who">
          <strong>{server.name}</strong>
          {#if server.command}<small class="mono">{server.command}</small>{/if}
        </span>
        <button class="btn btn-secondary" disabled={!!busy} onclick={() => move('mcp_everywhere', server.name)}>{t('mcp.toEverywhere')}</button>
      </div>
      <input
        class="field"
        value={server.description}
        placeholder={t('mcp.description')}
        aria-label={t('mcp.description')}
        onchange={(e) => describe(server.name, e.currentTarget.value)}
      />
    </li>
  {/each}
</ul>

<h3>{t('mcp.everywhere')}</h3>
<p class="intro">{t('mcp.everywhereIntro')}</p>
{#if mcp.loaded && !mcp.everywhere.length}
  <p class="empty">{t('mcp.noneEverywhere')}</p>
{/if}
<ul>
  {#each mcp.everywhere as server (server.name)}
    <li>
      <span class="who">
        <strong>{server.name}</strong>
        {#if server.command}<small class="mono">{server.command}</small>{/if}
      </span>
      <button class="btn btn-secondary" disabled={!!busy} onclick={() => move('mcp_on_demand', server.name)}>{t('mcp.toOnDemand')}</button>
    </li>
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

  h3 {
    margin: 22px 0 10px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li,
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  li {
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  li.stacked {
    flex-direction: column;
    align-items: stretch;
  }

  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  small {
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .field {
    height: 34px;
  }

  .bad {
    color: var(--err-text);
  }
</style>
