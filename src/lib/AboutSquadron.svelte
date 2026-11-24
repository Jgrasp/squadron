<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { getVersion } from '@tauri-apps/api/app'
  import { ui } from './actions.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { checkNow, install, update } from './update.svelte'
  import { locale, t } from './i18n/index.svelte'

  let version = $state('')
  getVersion().then((v) => (version = v)).catch(() => {})

  const files = import.meta.glob<string>('../../CHANGELOG*.md', { query: '?raw', import: 'default', eager: true })
  const releases = $derived(
    (files[`../../CHANGELOG.${locale()}.md`] ?? files['../../CHANGELOG.md'])
      .split(/^## /m)
      .slice(1)
      .map((block) => {
        const [head, ...body] = block.split('\n')
        const [name, date] = head.split(' · ')
        return { name, date, text: body.join('\n').trim() }
      }),
  )
  let toggled = $state<Record<string, boolean>>({})
  const opened = (name: string, first: boolean) => toggled[name] ?? first

  let news = $state<HTMLElement>()
  onMount(async () => {
    if (ui.settingsAnchor !== 'news') return
    ui.settingsAnchor = ''
    await tick()
    news?.scrollIntoView({ block: 'start' })
  })
</script>

<div class="head">
  <span class="triangle" aria-hidden="true"></span>
  <span class="who">
    <strong>Squadron</strong>
    <small>{t('settings.versionLine', { version: version || '…', system: navigator.platform.startsWith('Mac') ? 'macOS' : navigator.platform.startsWith('Win') ? 'Windows' : 'Linux' })}</small>
  </span>
  <span class="updates">
    {#if update.found}
      <span>{t('settings.updateAvailable', { version: update.found.version })}</span>
      <button class="btn btn-primary" disabled={update.progress !== null} onclick={install}>{t('settings.update')}</button>
    {:else}
      <span>{t('settings.checkedEvery', { count: 10 })}</span>
      <button class="btn btn-secondary" disabled={update.checking} onclick={checkNow}>{update.checking ? t('common.checking') : t('settings.itemUpdates')}</button>
    {/if}
  </span>
</div>
<p>{t('settings.aboutText')}</p>

<section class="news" bind:this={news}>
  <h3>{t('settings.changelog')}</h3>
  <div class="block">
    {#each releases as release, i (release.name)}
      {@const open = opened(release.name, i === 0)}
      <div class="release">
        <button class="row" aria-expanded={open} onclick={() => (toggled[release.name] = !open)}>
          <span class="chevron" class:down={open}><Icon name="chevron" size={12} /></span>
          <strong>{release.name}</strong>
          <small>{release.date}</small>
          {#if release.name === version}<small class="current">{t('settings.currentVersion')}</small>{/if}
        </button>
        {#if open}
          <div class="notes"><Markdown text={release.text} /></div>
        {/if}
      </div>
    {/each}
  </div>
</section>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }

  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .head strong {
    font-size: 16px;
  }

  small {
    color: var(--text-dim);
  }

  .triangle {
    width: 28px;
    height: 28px;
    background: var(--accent);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.6;
  }

  .updates {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .updates .btn {
    height: 32px;
    padding: 0 12px;
  }

  .news {
    margin-top: 28px;
  }

  h3 {
    margin: 0 0 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .block {
    background: var(--surface-1);
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--border);
    overflow: hidden;
  }

  .release + .release {
    border-top: 1px solid var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 0 14px;
    text-align: left;
    color: var(--text);
  }

  .row:hover {
    background: var(--surface-2);
  }

  .row strong {
    font-size: 13px;
    font-weight: 600;
  }

  .row small {
    font-size: 12px;
  }

  .current {
    margin-left: auto;
  }

  .chevron {
    display: grid;
    color: var(--text-dim);
    transform: rotate(-90deg);
  }

  .chevron.down {
    transform: none;
  }

  .notes {
    padding: 14px;
    border-top: 1px solid var(--border);
  }

  .notes :global(h3) {
    margin: 14px 0 6px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-muted);
  }

  .notes :global(h3:first-child) {
    margin-top: 0;
  }

  .notes :global(ul) {
    margin: 0;
    padding-left: 18px;
  }

  .notes :global(li) {
    margin: 0 0 4px;
    line-height: 1.55;
  }
</style>
