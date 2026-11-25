<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { loadServers, mcp } from './ondemand.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let error = $state('')
  let busy = $state('')

  const servers = $derived(
    [...mcp.onDemand.map((s) => ({ ...s, onDemand: true })), ...mcp.everywhere.map((s) => ({ ...s, onDemand: false }))].sort((a, b) =>
      a.name.localeCompare(b.name),
    ),
  )

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

<div class="mcp">
  <p class="intro">{t('mcp.intro')}</p>
  {#if servers.some((s) => s.command === 'claude.ai')}<p class="intro">{t('mcp.connectors')}</p>{/if}
  {#if mcp.loaded && !servers.length}
    <p class="intro">{t('mcp.none')}</p>
  {:else if servers.length}
    <div class="options">
      {#each servers as server (server.name)}
        <div class="server">
          <div class="option">
            <span class="label">
              <strong>{server.name}</strong>
              {#if server.command}<code>{server.command}</code>{/if}
            </span>
            <div class="segments" role="group" aria-label={server.name}>
              <button
                aria-pressed={!server.onDemand}
                disabled={!!busy}
                onclick={() => server.onDemand && move('mcp_everywhere', server.name)}>{t('mcp.everywhere')}</button
              >
              <button
                aria-pressed={server.onDemand}
                disabled={!!busy}
                onclick={() => !server.onDemand && move('mcp_on_demand', server.name)}>{t('mcp.onDemand')}</button
              >
            </div>
          </div>
          {#if server.onDemand}
            <div class="sub">
              <input
                class="field"
                value={server.description}
                placeholder={t('mcp.description')}
                aria-label={t('mcp.description')}
                onchange={(e) => describe(server.name, e.currentTarget.value)}
              />
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
</div>

<style>
  .mcp {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .server + .server {
    border-top: 1px solid var(--border);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  strong {
    font-size: 13px;
    font-weight: 600;
  }

  code {
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .segments {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-2);
    font-size: 12px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .segments button:disabled {
    opacity: 0.5;
  }

  .sub {
    padding: 0 14px 12px;
  }

  .field {
    width: 100%;
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }
</style>
