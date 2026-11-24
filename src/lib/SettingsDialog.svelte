<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Dialog from './Dialog.svelte'
  import AboutSquadron from './AboutSquadron.svelte'
  import ClaudeSettings from './ClaudeSettings.svelte'
  import CodeHosting from './CodeHosting.svelte'
  import { rememberSettingsTab, ui, type SettingsTab } from './actions.svelte'
  import ProjectSettings from './ProjectSettings.svelte'
  import LayoutSettings from './LayoutSettings.svelte'
  import NotificationSettings from './NotificationSettings.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import ExtensionPage from './ExtensionPage.svelte'
  import ExtensionsDiscover from './ExtensionsDiscover.svelte'
  import Icon from './Icon.svelte'
  import { brokenAccounts, checkAccounts, extensions } from './extensions.svelte'
  import { nameOf, pageOf, problemOf, sorted, updateOf, waitingOf } from './extension-pages.svelte'
  import PluginSettings from './PluginSettings.svelte'
  import PreferenceSettings from './PreferenceSettings.svelte'
  import AgentSettings from './AgentSettings.svelte'
  import ShortcutSettings from './ShortcutSettings.svelte'
  import McpServers from './McpServers.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { untrack } from 'svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'

  type Tab = SettingsTab

  let { onclose }: { onclose: () => void } = $props()

  const tab = $derived(ui.settings ?? 'projects')
  $effect(() => {
    rememberSettingsTab(tab)
  })

  const groups: [Key, [Tab, Key][]][] = [
    [
      'settings.projects',
      [
        ['projects', 'settings.projects'],
        ['agents', 'settings.agents'],
      ],
    ],
    [
      'settings.groupTools',
      [
        ['claude', 'settings.tabClaude'],
        ['plugins', 'settings.tabPlugins'],
        ['mcp', 'mcp.tab'],
        ['hosting', 'settings.tabHosting'],
      ],
    ],
    [
      'settings.groupInterface',
      [
        ['preferences', 'settings.tabPreferences'],
        ['notifications', 'settings.tabNotifications'],
        ['shortcuts', 'settings.tabShortcuts'],
        ['layout', 'settings.tabLayout'],
      ],
    ],
    [
      'settings.groupSquadron',
      [
        ['about', 'settings.tabAbout'],
      ],
    ],
  ]
  const LISTED = 5
  untrack(() => extensions.list.forEach((e) => checkAccounts(e, true)))
  let expanded = $state(false)
  const enabled = $derived(sorted(extensions.list.filter((e) => e.enabled)))
  const listed = $derived.by(() => {
    const shown = expanded ? enabled : enabled.slice(0, LISTED)
    const open = extensions.list.find((e) => tab === `ext:${pageOf(e)}`)
    return open && !shown.includes(open) ? [...shown, open] : shown
  })

  const current = $derived.by(() => {
    if (tab === 'discover') return t('settings.tabExtensions')
    const key = groups.flatMap(([, items]) => items).find(([id]) => id === tab)?.[1]
    return key ? t(key) : ''
  })
  let claude = $state(settings.claude)
  let error = $state('')
  let saved = $state(false)

  const form = $derived({ claude })
  let baseline = $state(untrack(() => JSON.stringify(form)))
  const dirty = $derived(JSON.stringify(form) !== baseline)

  async function save() {
    error = ''
    try {
      await saveSettings({ ...$state.snapshot(settings), ...form })
      baseline = JSON.stringify(form)
      saved = true
      setTimeout(() => (saved = false), 1500)
    } catch (e) {
      error = errorText(e)
    }
  }

</script>

<Dialog title={t('settings.title')} {onclose} full>
  <div class="frame">
  <div class="side" role="tablist" aria-label={t('settings.sections')} aria-orientation="vertical">
    {#each groups as [title, items] (title)}
      <span class="group">{t(title)}</span>
      {#each items as [id, label] (id)}
        <button role="tab" aria-selected={tab === id} onclick={() => (ui.settings = id)}>{t(label)}</button>
      {/each}
    {/each}
    <span class="group">{t('settings.tabExtensions')}</span>
    <button role="tab" class="all" aria-selected={tab === 'discover'} onclick={() => (ui.settings = 'discover')}><Icon name="grid" />{t('extensions.allExtensions')}</button>
    {#each listed as e (e.dir)}
      {@const update = updateOf(e)}
      <button role="tab" class="ext" class:off={!e.enabled} aria-selected={tab === `ext:${pageOf(e)}`} onclick={() => (ui.settings = `ext:${pageOf(e)}`)}>
        <ExtensionLogo id={e.id} name={nameOf(e)} logo={e.logo} size={16} />
        <span class="name">{nameOf(e)}</span>
        {#if waitingOf(e)}<span class="wait" title={t('extensions.toAllow')}></span>{:else if problemOf(e) || brokenAccounts(e)}<span class="broken" title={problemOf(e) || t('extensions.accountBroken')}></span>{:else if update}<span class="tag" title={t('extensions.update', { version: update.version })}>{t('extensions.updateNow')}</span>{/if}
      </button>
    {/each}
    {#if enabled.length > LISTED}
      <button class="more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
        <span class="chevron" class:up={expanded}><Icon name="chevron" size={12} /></span>{expanded ? t('extensions.showLess') : t('extensions.moreCount', { count: enabled.length - LISTED })}
      </button>
    {/if}
  </div>

  <div class="content">
  {#if !tab.startsWith('ext:')}<h2 class="heading">{current}</h2>{/if}
  <div class="body" class:flush={tab === 'projects'} class:page={tab.startsWith('ext:')}>
    {#if tab === 'projects'}
      <ProjectSettings />
    {:else if tab === 'about'}
      <AboutSquadron />
    {:else if tab === 'hosting'}
      <CodeHosting />
    {:else if tab === 'layout'}
      <LayoutSettings />
    {:else if tab === 'notifications'}
      <NotificationSettings />
    {:else if tab === 'discover'}
      <ExtensionsDiscover />
    {:else if tab.startsWith('ext:')}
      <ExtensionPage target={tab.slice('ext:'.length)} />
    {:else if tab === 'plugins'}
      <PluginSettings />
    {:else if tab === 'mcp'}
      <McpServers />
    {:else if tab === 'shortcuts'}
      <ShortcutSettings />
    {:else if tab === 'preferences'}
      <PreferenceSettings />
    {:else if tab === 'agents'}
      <AgentSettings />
    {:else}
      <ClaudeSettings bind:path={claude} />
    {/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
  </div>

  {#if tab === 'claude'}
    <footer>
      {#if saved}<span class="good">{t('common.saved')}</span>{/if}
      <button class="btn btn-secondary" onclick={onclose}>{t('common.close')}</button>
      <button class="btn btn-primary" onclick={save} disabled={!dirty}>{t('common.save')}</button>
    </footer>
  {/if}
  </div>
  </div>
</Dialog>

<style>
  .frame {
    flex: 1;
    min-height: 0;
    display: flex;
    border-top: 1px solid var(--border);
    margin-top: 14px;
  }

  .side {
    flex: none;
    width: 230px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 14px 12px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
  }

  .group {
    padding: 14px 10px 6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .group:first-child {
    padding-top: 2px;
  }

  .side button {
    flex: none;
    height: 34px;
    padding: 0 10px;
    border-radius: 4px;
    text-align: left;
    color: var(--text-muted);
  }

  .side button:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .side button[aria-selected='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    font-weight: 600;
  }

  .side .all,
  .side .ext {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .side .ext.off .name,
  .side .ext.off :global(.logo) {
    opacity: 0.5;
  }

  .ext .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ext .wait,
  .ext .broken {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }

  .ext .broken {
    background: var(--err);
  }

  .ext .tag {
    flex: none;
    height: 18px;
    padding: 0 6px;
    display: grid;
    place-items: center;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 11px;
    font-weight: 600;
  }

  .side .more {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .more .chevron {
    display: grid;
  }

  .more .chevron.up {
    transform: rotate(180deg);
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .heading {
    margin: 0;
    padding: 20px 32px 4px;
    font-size: 16px;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
    padding: 14px 32px 24px;
  }

  .body > :global(*) {
    max-width: 860px;
  }

  .body.page {
    padding-top: 20px;
  }

  .body.page > :global(*) {
    max-width: 860px;
  }

  .body.page {
    gap: 0;
  }

  .body.flush {
    gap: 0;
    padding: 0;
    overflow: hidden;
  }

  .body.flush > :global(*) {
    max-width: none;
  }

  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  .good {
    margin: 0;
    font: 12.5px var(--mono);
    color: var(--done-text);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }
</style>
