<script lang="ts" module>
  export const cardOf = (text: string) => (/^Usage: \/config\b/.test(text) ? 'config' : /^\d+ MCP server\(s\):/.test(text) ? 'mcp' : undefined)
</script>

<script lang="ts">
  import { untrack } from 'svelte'
  import { ui } from './actions.svelte'
  import { mcpAct, mcpStatus, mcpToggle, servers, type Server } from './agents.svelte'
  import { claudeConfig, labelOf, TERMINAL } from './claudeConfig.svelte'
  import { settingsOf } from './commands'
  import { time } from './format'
  import { t, type Key } from './i18n/index.svelte'

  let { id, text, at, onsend }: { id: string; text: string; at: number; onsend: (text: string) => void } = $props()

  const config = $derived(cardOf(text) === 'config')
  let query = $state('')
  let folded = $state(true)
  let typed = $state<Record<string, string>>({})
  const q = $derived(query.trim().toLowerCase())

  const settings = $derived(config ? settingsOf(text).filter((s) => `${s.key} ${labelOf(s.key)}`.toLowerCase().includes(q)) : [])
  const own = $derived(settings.filter((s) => !TERMINAL.has(s.key)))
  const terminal = $derived(settings.filter((s) => TERMINAL.has(s.key)))

  const ORDER = ['needs-auth', 'failed', 'pending', 'connected', 'disabled']
  const TONES: Record<string, string> = { connected: 'ok', failed: 'bad', 'needs-auth': 'wait', pending: 'wait' }
  const all = $derived(config ? undefined : servers[id])
  const listed = $derived((all ?? []).filter((s) => s.name.toLowerCase().includes(q)).toSorted((a, b) => ORDER.indexOf(a.status) - ORDER.indexOf(b.status)))

  $effect(() => {
    if (!config) untrack(() => mcpStatus(id))
  })

  function set(key: string, value: string) {
    if (!value.trim()) return
    claudeConfig.chosen[key] = value.trim()
    onsend(`/config ${key}=${value.trim()}`)
  }

  function act(server: Server) {
    if (server.status === 'connected') mcpToggle(id, server.name, false)
    else if (server.status === 'disabled') mcpToggle(id, server.name, true)
    else mcpAct(id, server)
  }

  const actionOf = (status: string): Key =>
    status === 'needs-auth' ? 'conversation.mcpAuthorize' : status === 'connected' ? 'conversation.mcpDisable' : status === 'disabled' ? 'conversation.mcpEnable' : 'conversation.mcpReconnect'
</script>

{#if !config && !all}
  <div class="plain">
    <span class="from">Claude Code · {time(at)}</span>
    {text}
  </div>
{:else}
  <div class="card">
    <div class="head">
      <div class="titles">
        <span class="from">Claude Code · {time(at)}</span>
        <strong>{config ? t('settings.claudeConfig') : `${t('conversation.mcpServers')} · ${all?.length}`}</strong>
      </div>
      <input class="search" type="search" placeholder={t(config ? 'settings.claudeConfigSearch' : 'conversation.mcpSearch')} aria-label={t(config ? 'settings.claudeConfigSearch' : 'conversation.mcpSearch')} bind:value={query} />
      {#if config}
        <button class="link" onclick={() => ((claudeConfig.query = query), (ui.settings = 'claude'))}>{t('agent.configWindow')} ›</button>
      {:else}
        <button class="link" onclick={() => mcpStatus(id)}>{t('conversation.mcpRefresh')}</button>
      {/if}
    </div>
    <div class="body">
      {#if config}
        {#each own as setting (setting.key)}{@render row(setting)}{/each}
        {#if terminal.length}
          <button class="fold" aria-expanded={!folded || (!!q && !own.length)} onclick={() => (folded = !folded)}>
            <svg width="10" height="10" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
            {t('settings.claudeConfigTerminal')} <small>{t('conversation.terminalCount', { count: terminal.length })}</small>
          </button>
          {#if !folded || (q && !own.length)}
            <div class="terminal">{#each terminal as setting (setting.key)}{@render row(setting)}{/each}</div>
          {/if}
        {/if}
        {#if !settings.length}<p class="none">{t('settings.claudeConfigNone', { query: query.trim() })}</p>{/if}
      {:else}
        {#each listed as server (server.name)}
          <div class="row">
            <div class="what">
              <span class="label"><i class="dot" data-tone={TONES[server.status]}></i>{server.name}</span>
              <span class="key" data-tone={TONES[server.status]}>{t(`agent.mcpStatus_${server.status}` as Key)}{#if server.error}{` · ${server.error}`}{/if}</span>
            </div>
            <button class="action" onclick={() => act(server)}>{t(actionOf(server.status))}</button>
          </div>
        {/each}
        {#if !listed.length}<p class="none">{t('conversation.mcpNone', { query: query.trim() })}</p>{/if}
      {/if}
    </div>
  </div>
{/if}

{#snippet row(setting: { key: string; values: string[] })}
  {@const chosen = claudeConfig.chosen[setting.key]}
  <div class="row">
    <div class="what">
      <span class="label">{labelOf(setting.key) || setting.key}</span>
      <span class="key">{setting.key}</span>
    </div>
    {#if !setting.values.length}
      <form class="free" onsubmit={(e) => (e.preventDefault(), set(setting.key, typed[setting.key] ?? ''))}>
        <input aria-label={setting.key} placeholder={chosen} bind:value={typed[setting.key]} />
        <button disabled={!typed[setting.key]?.trim()}>{t('settings.claudeConfigApply')}</button>
      </form>
    {:else if setting.values.length <= 3}
      <div class="seg" role="group" aria-label={setting.key}>
        {#each setting.values as value (value)}
          <button aria-pressed={chosen === value} title="/config {setting.key}={value}" onclick={() => set(setting.key, value)}>
            {value === 'true' ? t('conversation.yes') : value === 'false' ? t('conversation.no') : value}
          </button>
        {/each}
      </div>
    {:else}
      <select class="field" aria-label={setting.key} value={chosen ?? ''} onchange={(e) => set(setting.key, e.currentTarget.value)}>
        <option value="" disabled>{t('settings.claudeConfigPick')}</option>
        {#each setting.values as value (value)}<option {value}>{value}</option>{/each}
      </select>
    {/if}
  </div>
{/snippet}

<style>
  .plain {
    align-self: flex-start;
    display: flex;
    flex-direction: column;
    gap: 3px;
    max-width: 640px;
    padding: 8px 12px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .from {
    color: var(--text-faint);
    font-size: 11.5px;
  }

  .card {
    align-self: stretch;
    max-width: 680px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }

  .titles {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .titles strong {
    font-size: 13px;
    font-weight: 600;
  }

  .search {
    width: 190px;
    height: 28px;
    padding: 0 9px;
    font: 12.5px var(--sans);
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    outline: none;
  }

  .search:focus {
    border-color: var(--border-strong);
  }

  .link {
    padding: 4px 2px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .link:hover {
    color: var(--accent-soft);
  }

  .body {
    max-height: 380px;
    overflow: auto;
    scrollbar-width: thin;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 40px;
    padding: 7px 14px;
  }

  .row + .row {
    border-top: 1px solid var(--border);
  }

  .row:hover {
    background: var(--surface-1);
  }

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .label {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
  }

  .key {
    font: 11px var(--mono);
    color: var(--text-faint);
    overflow-wrap: anywhere;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .dot[data-tone='ok'] {
    background: var(--ok);
  }

  .dot[data-tone='bad'] {
    background: var(--err);
  }

  .dot[data-tone='wait'] {
    background: var(--accent-soft);
  }

  .key[data-tone='ok'] {
    color: var(--ok);
  }

  .key[data-tone='bad'] {
    color: var(--err);
  }

  .key[data-tone='wait'] {
    color: var(--accent-soft);
  }

  .seg {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .seg button {
    padding: 3px 9px;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .seg button:hover {
    color: var(--text);
    background: var(--surface-3);
  }

  .seg button[aria-pressed='true'] {
    color: var(--bg-main);
    background: var(--accent-soft);
  }

  select.field {
    flex: none;
    width: 170px;
    height: 28px;
    font: 12px var(--mono);
  }

  .free {
    flex: none;
    display: flex;
    gap: 4px;
  }

  .free input {
    width: 140px;
    height: 28px;
    padding: 0 8px;
    font: 12px var(--sans);
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    outline: none;
  }

  .free button,
  .action {
    flex: none;
    height: 28px;
    padding: 0 10px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-dim);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .free button:hover:not(:disabled),
  .action:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }

  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 9px 14px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    text-align: left;
  }

  .fold:hover {
    color: var(--text);
  }

  .fold small {
    font-weight: 400;
    color: var(--text-faint);
  }

  .fold svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    transition: transform 0.15s;
  }

  .fold[aria-expanded='true'] svg {
    transform: rotate(90deg);
  }

  .terminal .row {
    opacity: 0.55;
  }

  .terminal .row:hover {
    opacity: 0.85;
  }

  .none {
    margin: 0;
    padding: 14px;
    font-size: 12.5px;
    color: var(--text-faint);
  }
</style>
