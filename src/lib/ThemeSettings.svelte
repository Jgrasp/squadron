<script lang="ts">
  import { errorText, locale, t } from './i18n/index.svelte'
  import { text, themesOf } from './extensions'
  import { extensions } from './extensions.svelte'
  import { patchSettings, settings } from './settings.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'

  const PREVIEW = ['bg-main', 'bg-sidebar', 'surface-1', 'text', 'accent']
  const root = [...document.styleSheets]
    .flatMap((sheet) => {
      try {
        return [...sheet.cssRules]
      } catch {
        return []
      }
    })
    .find((rule): rule is CSSStyleRule => rule instanceof CSSStyleRule && rule.selectorText === ':root' && !!rule.style.getPropertyValue('--accent'))
  const cuivre: Record<string, string> = Object.fromEntries(PREVIEW.map((name) => [name, root?.style.getPropertyValue(`--${name}`).trim() ?? '']))

  const themes = $derived(themesOf(extensions.list))
  const chosen = $derived(themes.some((m) => m.id === settings.theme) ? settings.theme : '')
  const cards = $derived([
    { id: '', name: 'Cuivre', vars: cuivre, extension: null },
    ...themes.map((m) => ({ id: m.id, name: text(m.manifest.name, locale()), vars: { ...cuivre, ...m.manifest.contributes?.theme }, extension: m })),
  ])
  const preview = (vars: Record<string, string>) => PREVIEW.map((name) => `--${name}: ${vars[name]}`).join('; ')

  let error = $state('')

  async function choose(theme: string) {
    error = ''
    try {
      await patchSettings({ theme })
    } catch (e) {
      error = errorText(e)
    }
  }
</script>

<p class="help">{t('settings.themeHint')}</p>
<div class="cards">
  {#each cards as card (card.id)}
    <button class="card" aria-pressed={card.id === chosen} onclick={() => choose(card.id)}>
      <span class="mini" style={preview(card.vars)} aria-hidden="true">
        <span class="side"><i></i><i></i><i></i></span>
        <span class="main">
          <span class="row"><span class="dot"></span><i class="title"></i></span>
          <i class="block"></i>
          <i class="faint"></i>
        </span>
      </span>
      <span class="line"><span class="name">{card.name}</span>{#if card.id === chosen}<span class="in-use">{t('settings.themeInUse')}</span>{/if}</span>
      <span class="from">
        {#if card.extension}
          <ExtensionLogo id={card.extension.id} name={card.name} logo={card.extension.logo} size={16} />{t('settings.themeExtension')}
        {:else}
          {t('settings.themeDefault')}
        {/if}
      </span>
    </button>
  {/each}
</div>

{#if error}<p class="bad" role="alert">{error}</p>{/if}

<style>
  .help {
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    text-align: left;
    color: var(--text);
  }

  .card:hover {
    background: var(--surface-2);
  }

  .card[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
  }

  .mini {
    height: 84px;
    display: grid;
    grid-template-columns: 38px 1fr;
    overflow: hidden;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .mini i {
    display: block;
    height: 6px;
    border-radius: 2px;
    background: var(--text);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 8px 6px;
    background: var(--bg-sidebar);
  }

  .side i {
    height: 5px;
    opacity: 0.35;
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }

  .mini .title {
    width: 60%;
    opacity: 0.7;
  }

  .mini .block {
    height: 24px;
    border-radius: 3px;
    background: var(--surface-1);
  }

  .mini .faint {
    width: 40%;
    opacity: 0.3;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
  }

  .in-use {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--text);
    color: var(--ink);
    font-size: 11px;
    font-weight: 700;
  }

  .from {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
