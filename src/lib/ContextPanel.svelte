<script lang="ts" module>
  import { invoke } from '@tauri-apps/api/core'

  const promptSize = invoke<number>('squadron_prompt_size').catch(() => 0)
</script>

<script lang="ts">
  import { ui } from './actions.svelte'
  import { send } from './agents.svelte'
  import { breakdown, type Group, type Line } from './breakdown'
  import { tokens } from './format'
  import { t, type Key } from './i18n/index.svelte'
  import { layout, setContextShown } from './layout.svelte'
  import { pluginColor, SHOWS } from './plugins'
  import { openUsage, skillCost } from './plugins.svelte'
  import { uses } from './pluginuse'
  import Select from './Select.svelte'
  import { COMPACT_WINDOWS, saveSettings, settings } from './settings.svelte'
  import type { Agent } from './stream'

  let { agent, onclose }: { agent: Agent; onclose: () => void } = $props()

  const COUNTED = ['builtinSkills', 'syncedSkills', 'skills', 'agents', 'mcpTools']
  const SHOWN = [
    ['percent', 'agent.contextShownPercent'],
    ['tokens', 'agent.contextShownTokens'],
    ['both', 'agent.contextShownBoth'],
  ] as const
  const FIRST = 4

  let tab = $state<'usage' | 'settings'>('usage')
  let unfolded = $state<string[]>(['plugins', 'calls'])
  let keep = $state('')
  let prompt = $state(0)
  promptSize.then((size) => (prompt = Math.ceil(size / 4)))

  const refs = $derived(agent.plugins ?? [])
  const b = $derived(agent.usage ? breakdown(agent.usage, refs, prompt, uses(agent, skillCost(refs)), agent.hooks) : null)
  const max = $derived(agent.context?.max ?? 0)
  const threshold = $derived(agent.context?.threshold ?? null)
  const busy = $derived(agent.status === 'working' || agent.status === 'waiting')
  const startTotal = $derived(b ? b.start.reduce((s, g) => s + g.tokens, 0) : 0)
  const talkTotal = $derived(b ? b.talk.reduce((s, g) => s + g.tokens, 0) : 0)
  const percent = (n: number) => Math.round((n / max) * 100)

  const isOpen = (id: string) => unfolded.includes(id)
  const fold = (id: string) => (unfolded = isOpen(id) ? unfolded.filter((x) => x !== id) : [...unfolded, id])
  const colorOf = (plugin: string) => pluginColor(refs.find((r) => r.name === plugin)?.id ?? plugin)

  function label(line: Line) {
    if (COUNTED.includes(line.kind)) return t(`usage.${line.kind}` as Key, { count: line.count ?? 0 })
    if (line.kind === 'added') return t('usage.hookOutput')
    if (line.name !== undefined) return line.name
    return t(`usage.${line.kind}` as Key)
  }

  const summary = (g: Group) => (g.id === 'plugins' ? t('usage.loaded', { count: refs.length }) : g.lines.slice(0, 3).map(label).join(' · '))
  const amount = (line: Line) => (line.estimate ? '≈' : '') + tokens(line.tokens)

  function compact() {
    send(agent.id, `/compact ${keep}`.trim())
    keep = ''
    onclose()
  }

  function managePlugins() {
    onclose()
    ui.settingsAnchor = ''
    ui.settings = 'plugins'
  }

  const save = (patch: Partial<typeof settings>) => saveSettings({ ...$state.snapshot(settings), ...patch })
</script>

{#snippet logo(plugin: string)}
  <span class="logo" style:background={colorOf(plugin)} aria-hidden="true">{plugin.charAt(0).toUpperCase()}</span>
{/snippet}

{#snippet group(g: Group)}
  {@const open = isOpen(g.id)}
  {@const foldable = g.id === 'plugins' ? refs.length > 0 : g.lines.length > 0}
  <button class="row" disabled={!foldable} aria-expanded={foldable ? open : undefined} onclick={() => fold(g.id)}>
    <span class="chev" class:on={open} class:none={!foldable} aria-hidden="true">›</span>
    <span class="name"><i class="c-{g.id}"></i>{t(`usage.${g.id}` as Key)}</span>
    <span class="what">{summary(g)}</span>
    <span class="mono">{tokens(g.tokens)}</span>
  </button>
  {#if open && g.id === 'plugins' && b}
    {@const all = isOpen('all-plugins') ? b.plugins : b.plugins.slice(0, FIRST)}
    <div class="nested">
      {#each all as p (p.name)}
        {@const own = isOpen(`p:${p.name}`)}
        <button class="row" disabled={!p.lines.length} onclick={() => fold(`p:${p.name}`)}>
          <span class="name">{@render logo(p.name)}{p.name}</span>
          <span class="what">{p.lines.length ? p.lines.map((l) => `${label(l)} ${tokens(l.tokens)}`).join(' · ') : t('usage.nothingLoaded')}</span>
          <span class="mono">{tokens(p.tokens)}</span>
        </button>
        {#if own}
          <div class="nested">
            {#each p.lines as l, i (i)}<div class="row"><span class="name wide">{label(l)}</span><span class="what"></span><span class="mono">{amount(l)}</span></div>{/each}
          </div>
        {/if}
      {/each}
      {#if b.plugins.length > FIRST && !isOpen('all-plugins')}
        <button class="row" onclick={() => fold('all-plugins')}>
          <span class="name more">{t('usage.more', { count: b.plugins.length - FIRST })}</span>
          <span class="what"></span>
          <span class="mono">{tokens(b.plugins.slice(FIRST).reduce((s, p) => s + p.tokens, 0))}</span>
        </button>
      {/if}
    </div>
  {:else if open}
    <div class="nested">
      {#each g.lines as l, i (i)}
        {#if l.plugin && l.use}
          <button class="row" onclick={() => (onclose(), openUsage(agent.id, { use: l.use! }))}>
            <span class="name">{@render logo(l.plugin)}{l.plugin}</span><span class="what">{label(l)}</span><span class="mono" title={l.estimate ? t('usage.estimate') : undefined}>{amount(l)}</span>
          </button>
        {:else if l.plugin}
          <div class="row"><span class="name">{@render logo(l.plugin)}{l.plugin}</span><span class="what">{label(l)}</span><span class="mono">{amount(l)}</span></div>
        {:else}
          <div class="row"><span class="name wide" title={label(l)}>{label(l)}</span><span class="what"></span><span class="mono" title={l.estimate ? t('usage.estimate') : undefined}>{amount(l)}</span></div>
        {/if}
      {/each}
    </div>
  {/if}
{/snippet}

<div class="head">
  <strong>{t('agent.context')}</strong>
  <div class="segments" role="group" aria-label={t('usage.tabs')}>
    <button aria-pressed={tab === 'usage'} onclick={() => (tab = 'usage')}>{t('usage.tabUsage')}</button>
    <button aria-pressed={tab === 'settings'} onclick={() => (tab = 'settings')}>{t('usage.tabSettings')}</button>
  </div>
</div>

{#if tab === 'settings'}
  <div class="form">
    <div class="field-row">
      <span>{t('usage.gaugeShows')}</span>
      <div class="segments" role="group" aria-label={t('usage.gaugeShows')}>
        {#each SHOWN as [shown, key] (shown)}
          <button aria-pressed={layout.contextShown === shown} onclick={() => setContextShown(shown)}>{t(key)}</button>
        {/each}
      </div>
    </div>
    <div class="field-row">
      <span>{t('usage.autoCompaction')}<small>{t('usage.autoCompactionHint')}</small></span>
      <Select
        label={t('usage.autoCompaction')}
        value={String(settings.compact_window)}
        options={COMPACT_WINDOWS.map((size) => ({ value: String(size), label: size ? t('settings.compactAround', { tokens: tokens(size) }) : t('settings.compactDefault') }))}
        onpick={(value) => save({ compact_window: Number(value) })}
      />
    </div>
    <div class="field-row">
      <span>{t('usage.pluginLines')}</span>
      <Select
        label={t('usage.pluginLines')}
        value={settings.plugin_lines}
        options={SHOWS.map(([id, key]) => ({ value: id, label: t(key) }))}
        onpick={(value) => save({ plugin_lines: value as typeof settings.plugin_lines })}
      />
    </div>
    {#if settings.plugin_lines === 'card'}
      <div class="field-row">
        <span>{t('usage.cardStyle')}</span>
        <div class="segments" role="group" aria-label={t('usage.cardStyle')}>
          <button aria-pressed={settings.plugin_card !== 'minimal'} onclick={() => save({ plugin_card: 'normal' })}>{t('plugins.cardNormal')}</button>
          <button aria-pressed={settings.plugin_card === 'minimal'} onclick={() => save({ plugin_card: 'minimal' })}>{t('plugins.cardMinimal')}</button>
        </div>
      </div>
    {/if}
    <div class="field-row">
      <span>{t('usage.plugins')}<small>{t('usage.pluginsInstalled', { count: refs.length })}</small></span>
      <button class="btn btn-secondary" onclick={managePlugins}>{t('usage.managePlugins')}</button>
    </div>
  </div>
{:else}
  {#if b}
    {#if max}
      <div class="progress" aria-hidden="true">
        <span class="fill" style:width="{Math.min(100, (b.total / max) * 100)}%">
          {#each [...b.start, ...b.talk] as g (g.id)}{#if g.tokens}<i class="c-{g.id}" style:flex={g.tokens}></i>{/if}{/each}
        </span>
        {#if threshold}<span class="limit" style:left="{(threshold / max) * 100}%"></span>{/if}
      </div>
      <p class="sub">{t('usage.used', { used: tokens(b.total), max: tokens(max), percent: percent(b.total) })}</p>
    {:else}
      <p class="sub">{t('usage.usedTokens', { used: tokens(b.total) })}</p>
    {/if}
    <div class="tiles">
      <div class="tile"><b>{tokens(startTotal)}</b><span>{t('usage.atStartup')}</span></div>
      <div class="tile"><b>{tokens(talkTotal)}</b><span>{t('usage.byConversation')}</span></div>
    </div>
    <div class="scroll">
      <div class="caps">{t('usage.sectionStart')}<span class="mono">{tokens(startTotal)}</span></div>
      <div class="rows">{#each b.start as g (g.id)}{@render group(g)}{/each}</div>
      <div class="caps">{t('usage.sectionTalk')}<span class="mono">{tokens(talkTotal)}</span></div>
      <div class="rows">{#each b.talk as g (g.id)}{@render group(g)}{/each}</div>
    </div>
  {:else}
    <p class="sub">{t('usage.noBreakdown')}</p>
  {/if}

  <div class="caps rule">
    {t('usage.compaction')}<span class="mono">{threshold ? t('usage.autoAt', { at: tokens(threshold), left: tokens(Math.max(0, threshold - agent.tokens)) }) : t('usage.autoOff')}</span>
  </div>
  <div class="compactor">
    <input bind:value={keep} placeholder={t('usage.keep')} aria-label={t('usage.keep')} onkeydown={(e) => e.key === 'Enter' && !busy && agent.tokens && compact()} />
    <button class="btn btn-secondary" disabled={busy || !agent.tokens} onclick={compact}>{t('agent.compactNow')}</button>
  </div>
  {#if busy}<p class="hint">{t('agent.compactWhenIdle')}</p>{/if}
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  strong {
    font-weight: 600;
    color: var(--text-strong);
  }

  p {
    margin: 0;
  }

  .sub,
  .hint {
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .hint {
    font-size: 12px;
  }

  .segments {
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

  .progress {
    position: relative;
    height: 8px;
    border-radius: 4px;
    background: var(--surface-3);
  }

  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    display: flex;
    min-width: 6px;
    border-radius: 4px;
    overflow: hidden;
  }

  .fill i {
    display: block;
  }

  .limit {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--text-faint);
  }

  .c-claude {
    background: var(--text-faint);
  }

  .c-squadron {
    background: var(--remote-dot);
  }

  .c-you {
    background: var(--persona);
  }

  .c-mcp {
    background: var(--background);
  }

  .c-plugins,
  .c-calls {
    background: var(--advisor);
  }

  .c-tools,
  .c-replies,
  .c-messages {
    background: var(--ok);
  }

  .c-added {
    background: var(--ok-line);
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .tile b {
    font-family: var(--mono);
    font-size: 18px;
    font-weight: 500;
    color: var(--text-strong);
  }

  .tile span {
    color: var(--text-dim);
    font-size: 12px;
  }

  .caps {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding-top: 4px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .scroll {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 52vh;
    margin-right: -8px;
    padding-right: 8px;
    overflow-y: auto;
  }

  .caps.rule {
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .mono {
    font-family: var(--mono);
    font-size: 11.5px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-dim);
  }

  .rows,
  .nested {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .nested {
    margin-left: 26px;
    padding-left: 8px;
    box-shadow: inset 1px 0 0 var(--border-strong);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 30px;
    padding: 0 6px;
    border-radius: 4px;
    text-align: left;
  }

  .nested .row {
    height: 26px;
  }

  button.row:not(:disabled):hover {
    background: var(--surface-2);
  }

  button.row:disabled {
    cursor: default;
  }

  .name {
    width: 150px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12.5px;
  }

  .nested .name {
    width: 128px;
    font-size: 12px;
  }

  .nested .name.wide {
    display: block;
    width: 200px;
    color: var(--text-muted);
  }

  .name i {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }

  .more {
    color: var(--text-dim);
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .row > .mono {
    width: 48px;
    flex: none;
    text-align: right;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .chev {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-muted);
    font-size: 10px;
    transition: transform 0.15s;
  }

  .chev.on {
    transform: rotate(90deg);
  }

  .chev.none {
    visibility: hidden;
  }

  .logo {
    flex: none;
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    color: var(--ink);
    font-size: 9.5px;
    font-weight: 700;
  }

  .compactor {
    display: flex;
    align-items: center;
    height: 36px;
    padding: 0 3px 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .compactor:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .compactor input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    background: none;
    color: var(--text);
    font: 12.5px var(--sans);
    outline: none;
  }

  .compactor input::placeholder {
    color: var(--text-faint);
  }

  .compactor .btn,
  .field-row .btn {
    height: 28px;
    padding: 0 12px;
    font-size: 12.5px;
  }

  .compactor .btn {
    background: var(--surface-4);
  }

  .compactor .btn:not(:disabled):hover {
    background: var(--surface-5);
  }

  .form {
    display: flex;
    flex-direction: column;
  }

  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 48px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }

  .field-row:last-child {
    border-bottom: 0;
  }

  .field-row small {
    display: block;
    color: var(--text-dim);
    font-size: 12px;
  }
</style>
