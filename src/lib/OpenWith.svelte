<script lang="ts" module>
  import { invoke } from '@tauri-apps/api/core'
  import type { App } from './settings.svelte'

  let detection: Promise<App[]> | null = null
  export const detect = () => (detection ??= invoke<App[]>('detect_editors').catch(() => []))
</script>

<script lang="ts">
  import { message } from './asking.svelte'
  import { settings } from './settings.svelte'
  import { errorText, t } from './i18n/index.svelte'

  let { project, path = '', up = false }: { project: string; path?: string; up?: boolean } = $props()

  let detected = $state<App[]>([])
  let menu = $state(false)
  let last = $state('')
  try {
    last = localStorage.getItem('squadron.editor-app') ?? ''
  } catch {}

  const apps = $derived(settings.editors.length ? settings.editors : detected)
  const chosen = $derived(apps.find((a) => a.name === last) ?? apps[0])

  detect().then((list) => (detected = list))

  function run(app: App) {
    menu = false
    last = app.name
    try {
      localStorage.setItem('squadron.editor-app', app.name)
    } catch {}
    invoke('open_with', { command: app.command, project, path }).catch((e) => message(errorText(e), { title: t('shell.couldNotOpen', { name: app.name }), kind: 'error' }))
  }
</script>

<svelte:window onclick={(e) => !(e.target as HTMLElement).closest('[data-open-with]') && (menu = false)} />

{#if chosen}
  <span class="split" data-open-with>
    <button class="btn btn-secondary main" class:alone={apps.length < 2} title={chosen.command} onclick={() => run(chosen)}>{t('shell.openIn', { name: chosen.name })}</button>
    {#if apps.length > 1}
      <button class="btn btn-secondary more" aria-label={t('shell.chooseApp')} aria-haspopup="menu" aria-expanded={menu} onclick={() => (menu = !menu)}>
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
    {/if}
    {#if menu}
      <div class="menu" class:up role="menu" aria-label={t('shell.openWith')}>
        {#each apps as app (app.name)}
          <button role="menuitemradio" aria-checked={app === chosen} onclick={() => run(app)}>
            <strong>{app.name}</strong><span>{app.command}</span>
          </button>
        {/each}
      </div>
    {/if}
  </span>
{:else}
  <button class="btn btn-secondary main alone" onclick={() => invoke('open_in_editor', { project, path })}>{t('shell.openInExternal')}</button>
{/if}

<style>
  .split {
    position: relative;
    display: inline-flex;
    flex: none;
  }

  .btn {
    height: 32px;
    padding: 0 12px;
    font-size: 13px;
  }

  .main:not(.alone) {
    border-radius: 4px 0 0 4px;
  }

  .more {
    width: 30px;
    padding: 0;
    margin-left: 1px;
    border-radius: 0 4px 4px 0;
  }

  .more svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .menu {
    position: absolute;
    top: 38px;
    right: 0;
    z-index: 30;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .menu.up {
    top: auto;
    bottom: 38px;
  }

  .menu button {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 10px;
    border-radius: 4px;
    text-align: left;
  }

  .menu button:hover,
  .menu button[aria-checked='true'] {
    background: var(--menu-hover);
  }

  .menu strong {
    font-weight: 600;
  }

  .menu span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-muted);
  }
</style>
