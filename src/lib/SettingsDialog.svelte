<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import Dialog from './Dialog.svelte'
  import AboutSquadron from './AboutSquadron.svelte'
  import Changelog from './Changelog.svelte'
  import ClaudeSettings from './ClaudeSettings.svelte'
  import CodeHosting from './CodeHosting.svelte'
  import { ui, type SettingsTab } from './actions.svelte'
  import ProjectSettings from './ProjectSettings.svelte'
  import { reset } from './layout-slots.svelte'
  import PanelSettings from './PanelSettings.svelte'
  import NotificationSettings from './NotificationSettings.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import ExtensionPage from './ExtensionPage.svelte'
  import ExtensionsDiscover from './ExtensionsDiscover.svelte'
  import Icon from './Icon.svelte'
  import { brokenAccounts, checkAccounts, extensions } from './extensions.svelte'
  import { nameOf, pageOf, sorted, updateOf, waitingOf } from './extension-pages.svelte'
  import PluginSettings from './PluginSettings.svelte'
  import PreferenceSettings from './PreferenceSettings.svelte'
  import AgentSettings from './AgentSettings.svelte'
  import McpServers from './McpServers.svelte'
  import SlotPreview from './SlotPreview.svelte'
  import { SLOTS, type Slot } from './slots'
  import { saveSettings, settings } from './settings.svelte'
  import { untrack } from 'svelte'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { ACTIONS, binding, combo, label, usable, type Action } from './shortcuts'

  type Tab = SettingsTab

  let { onclose }: { onclose: () => void } = $props()

  const tab = $derived(ui.settings ?? 'projects')

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
        ['panels', 'settings.tabPanels'],
      ],
    ],
    [
      'settings.groupSquadron',
      [
        ['changelog', 'settings.tabChangelog'],
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
    if (tab === 'discover') return t('extensions.discoverTitle')
    const key = groups.flatMap(([, items]) => items).find(([id]) => id === tab)?.[1]
    return key ? t(key) : ''
  })
  let claude = $state(settings.claude)
  let keys = $state<Record<string, string>>({ ...settings.keys })
  let recording = $state<Action | null>(null)
  let conflict = $state('')

  function record(event: KeyboardEvent) {
    if (!recording) return
    event.preventDefault()
    event.stopPropagation()
    const value = combo(event)
    if (!value) return
    const other = ACTIONS.find(([id]) => id !== recording && binding(keys, id) === value)
    if (other) conflict = t('settings.conflictUsed', { keys: label(value), action: t(other[1]) })
    else if (!usable(value)) conflict = t('settings.conflictTyping', { keys: label(value) })
    else {
      if (value === ACTIONS.find(([id]) => id === recording)![2]) delete keys[recording]
      else keys[recording] = value
      recording = null
      conflict = ''
    }
  }
  let error = $state('')
  let saved = $state(false)

  const form = $derived({ claude, keys: $state.snapshot(keys) })
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

<svelte:window onkeydowncapture={record} />

<Dialog title={t('settings.title')} {onclose} full>
  <div class="frame">
  <div class="side" role="tablist" aria-label={t('settings.sections')} aria-orientation="vertical">
    {#each groups as [title, items] (title)}
      {#if title === 'settings.groupSquadron'}
        <div class="group tools">
          <span>{t('settings.tabExtensions')}</span>
          <button class="tool" role="tab" aria-selected={tab === 'discover'} title={t('extensions.discoverTitle')} aria-label={t('extensions.discoverTitle')} onclick={() => (ui.settings = 'discover')}><Icon name="plus" size={15} /></button>
        </div>
        {#each listed as e (e.dir)}
          {@const update = updateOf(e)}
          <button role="tab" class="ext" class:off={!e.enabled} aria-selected={tab === `ext:${pageOf(e)}`} onclick={() => (ui.settings = `ext:${pageOf(e)}`)}>
            <ExtensionLogo id={e.id} name={nameOf(e)} logo={e.logo} size={16} />
            <span class="name">{nameOf(e)}</span>
            {#if waitingOf(e)}<span class="wait" title={t('extensions.toAllow')}></span>{:else if brokenAccounts(e)}<span class="broken" title={t('extensions.accountBroken')}></span>{:else if update}<span class="tag" title={t('extensions.update', { version: update.version })}>{t('extensions.updateNow')}</span>{/if}
          </button>
        {/each}
        {#if enabled.length > LISTED}
          <button class="more" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
            <span class="chevron" class:up={expanded}><Icon name="chevron" size={12} /></span>{expanded ? t('extensions.showLess') : t('extensions.moreCount', { count: enabled.length - LISTED })}
          </button>
        {/if}
      {/if}
      <span class="group">{t(title)}</span>
      {#each items as [id, label] (id)}
        <button role="tab" aria-selected={tab === id} onclick={() => (ui.settings = id)}>{t(label)}</button>
      {/each}
    {/each}
  </div>

  <div class="content">
  {#if !tab.startsWith('ext:')}<h2 class="heading">{current}</h2>{/if}
  <div class="body" class:flush={tab === 'projects'} class:page={tab.startsWith('ext:')}>
    {#if tab === 'projects'}
      <ProjectSettings />
    {:else if tab === 'changelog'}
      <Changelog />
    {:else if tab === 'about'}
      <AboutSquadron />
    {:else if tab === 'hosting'}
      <CodeHosting />
    {:else if tab === 'layout'}
      <div class="intro row-intro">
        <p class="intro">{t('settings.layoutIntro')}</p>
        <button class="btn btn-secondary" onclick={() => ((ui.settings = null), (ui.customizing = true))}>{t('settings.customizeInPlace')}</button>
      </div>
      {#each Object.entries(SLOTS) as [slot, def] (slot)}
        <section>
          <h2>{t(def.title)} <span class="hint-text">{t(def.hint)}</span></h2>
          <SlotPreview slot={slot as Slot} />
          <button class="link" onclick={() => reset(slot as Slot)}>{t('settings.backToDefault')}</button>
        </section>
      {/each}
    {:else if tab === 'notifications'}
      <NotificationSettings />
    {:else if tab === 'panels'}
      <PanelSettings />
    {:else if tab === 'discover'}
      <ExtensionsDiscover />
    {:else if tab.startsWith('ext:')}
      <ExtensionPage target={tab.slice('ext:'.length)} />
    {:else if tab === 'plugins'}
      <PluginSettings />
    {:else if tab === 'mcp'}
      <McpServers />
    {:else if tab === 'shortcuts'}
      <p class="intro">{t('settings.shortcutsIntro')}</p>
      <ul class="keys">
        {#each ACTIONS as [id, name] (id)}
          <li>
            <span class="what">{t(name)}</span>
            {#if recording === id}
              <span class="listening" role="status">{t('settings.pressKeys')}</span>
              <button class="remove" onclick={() => ((recording = null), (conflict = ''))}>{t('common.cancel')}</button>
            {:else}
              <button class="combo" class:custom={!!keys[id]} title={t('settings.change')} onclick={() => ((recording = id), (conflict = ''))}>
                <kbd>{label(binding(keys, id))}</kbd>
              </button>
              {#if keys[id]}
                <button class="remove" onclick={() => delete keys[id]}>{t('common.default')}</button>
              {/if}
            {/if}
          </li>
        {/each}
      </ul>
      {#if conflict}<p class="bad" role="alert">{conflict}</p>{/if}
    {:else if tab === 'preferences'}
      <PreferenceSettings />
    {:else if tab === 'agents'}
      <AgentSettings />
    {:else}
      <ClaudeSettings bind:path={claude} />
    {/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
  </div>

  {#if !['layout', 'panels', 'discover', 'projects', 'agents', 'preferences', 'changelog', 'about', 'hosting', 'notifications'].includes(tab) && !tab.startsWith('ext:')}
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

  .group.tools {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 4px 2px 10px;
  }

  .group.tools span {
    flex: 1;
  }

  .side .tool {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    padding: 0;
    color: var(--text-dim);
  }

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

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  h2 span {
    margin-left: 6px;
    font: 400 12px var(--mono);
    color: var(--text-dim);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .remove {
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 12.5px;
  }

  .remove:hover {
    background: #3a2420;
    color: #f8d5cc;
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

  .keys {
    gap: 2px;
  }

  h2 .hint-text {
    font: 400 12.5px var(--sans);
  }

  .row-intro {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .row-intro .btn {
    flex: none;
    height: 34px;
  }






  .remove:disabled {
    opacity: 0.3;
  }

  .link {
    align-self: flex-start;
    font-size: 12.5px;
    color: var(--accent-link);
  }

  .keys li {
    padding: 6px 6px 6px 12px;
  }

  .what {
    flex: 1;
  }

  .combo {
    min-width: 96px;
    height: 30px;
    padding: 0 12px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12.5px var(--mono);
  }

  .combo:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .combo.custom {
    box-shadow: inset 0 0 0 1px var(--accent);
    color: var(--accent-soft);
  }

  .listening {
    min-width: 96px;
    height: 30px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text-2);
    font-size: 12.5px;
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
