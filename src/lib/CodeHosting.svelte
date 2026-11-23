<script lang="ts">
  import { saveSettings, settings } from './settings.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import HostLogo from './HostLogo.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import InstallDialog from './InstallDialog.svelte'
  import Select from './Select.svelte'
  import { onMount } from 'svelte'
  import { ui } from './actions.svelte'
  import type { CatalogEntry, Extension } from './extensions'
  import { catalog, checkAccounts, checks, extensions, loadCatalog, type AccountCheck } from './extensions.svelte'
  import { named, nameOf } from './extension-pages.svelte'
  import { installOffers, lookUpCoverage, providers } from './forges.svelte'

  let error = $state('')
  let chosen = $state<CatalogEntry | null>(null)

  onMount(() => {
    if (!catalog.entries) void loadCatalog()
    for (const e of providers()) checkAccounts(e, true)
  })

  $effect(() => {
    void extensions.list
    void extensions.unloaded
    void lookUpCoverage()
  })

  function stateOf(e: Extension): { text: string; bad: boolean } {
    const list = extensions.accounts[e.id] ?? []
    if (!list.length) return { text: t('settings.forgeNoAccount'), bad: false }
    const found = list.map((a) => checks[e.id]?.[a.id])
    const failed = found.find((c) => c && !c.running && !c.ok)
    if (failed && !failed.running) return { text: said(failed.title) || t('settings.forgeFailed'), bad: true }
    const ok = found.find((c) => c && !c.running && c.ok)
    if (ok && !ok.running) return { text: [said(ok.title), list.length > 1 ? t('settings.forgeAccountsCount', { count: list.length }) : ''].filter(Boolean).join(' · '), bad: false }
    return { text: found.some((c) => c?.running) ? t('extensions.accountChecking') : t('settings.forgeAccountsCount', { count: list.length }), bad: false }
  }

  const said = (value: AccountCheck['title']) => (typeof value === 'string' ? value : named(value ?? undefined))

  const change = (changes: Partial<typeof settings>) => saveSettings({ ...$state.snapshot(settings), ...changes }).catch((e) => (error = errorText(e)))

  const methods = $derived([
    { value: 'repo', label: t('review.methodRepo') },
    { value: 'merge', label: t('review.methodMerge') },
    { value: 'squash', label: t('review.methodSquash') },
    { value: 'rebase', label: t('review.methodRebase') },
  ])
  const others = $derived([
    { value: 'second', label: t('review.mergeOthersSecond') },
    { value: 'menu', label: t('review.mergeOthersMenu') },
    { value: 'hidden', label: t('review.mergeOthersHidden') },
  ])
  const placements = $derived([
    { value: 'banner', label: t('review.mergeReviewBanner') },
    { value: 'header', label: t('review.mergeReviewHeader') },
  ])
</script>

<div class="hosting">
  <p class="intro">{t('settings.hostingIntro')}</p>
  {#if error}<p class="bad" role="alert">{error}</p>{/if}

  <h3>{t('settings.forges')}</h3>
  <div class="options">
    {#each providers() as e (e.id)}
      {@const state = stateOf(e)}
      <div class="option">
        <span class="mark"><ExtensionLogo id={e.id} name={nameOf(e)} logo={e.logo} size={18} /></span>
        <span class="label">
          <strong>{nameOf(e)}<span class="pill">{t('settings.forgeExtension')}</span></strong>
          <small class:bad={state.bad}>{state.text}</small>
        </span>
        <button class="btn btn-ghost" onclick={() => (ui.settings = `ext:${e.id}`)}>{t('settings.forgeAccounts')}</button>
      </div>
    {/each}
    {#each installOffers() as offer (offer.entry.id)}
      <div class="option">
        <span class="mark">{@render logo(offer.entry.id, named(offer.entry.name))}</span>
        <span class="label">
          <strong>{named(offer.entry.name)}<span class="pill">{t('settings.forgeExtension')}</span></strong>
          <small>{t('settings.forgeWaiting', { count: offer.projects.length, host: offer.host })}</small>
          <span class="projects">{offer.projects.join(' · ')}</span>
        </span>
        <button class="btn btn-primary" onclick={() => (chosen = offer.entry)}>{t('settings.forgeInstall', { name: named(offer.entry.name) })}</button>
      </div>
    {/each}
    <div class="option">
      <span class="label">
        <strong>{t('settings.forgeOther')}</strong>
        <small>{t('settings.forgeOtherHint')}</small>
      </span>
      <button class="btn btn-ghost" onclick={() => (ui.settings = 'discover')}>{t('settings.forgeDiscover')}</button>
    </div>
  </div>

  <h3>{t('settings.merging')}</h3>
  <div class="options">
    <div class="option">
      <span class="label"><strong>{t('review.mergeMethod')}</strong><small>{t('review.mergeMethodIntro')}</small></span>
      <Select label={t('review.mergeMethod')} value={settings.merge_method} options={methods} onpick={(value) => change({ merge_method: value as typeof settings.merge_method })} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('review.mergeOthers')}</strong><small>{t('review.mergeOthersIntro')}</small></span>
      <Select label={t('review.mergeOthers')} value={settings.merge_others} options={others} onpick={(value) => change({ merge_others: value as typeof settings.merge_others })} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('review.mergeReview')}</strong><small>{t('review.mergeReviewIntro')}</small></span>
      <Select label={t('review.mergeReview')} value={settings.merge_review} options={placements} onpick={(value) => change({ merge_review: value as typeof settings.merge_review })} />
    </div>
  </div>

  <div class="elsewhere">
    <span>{t('settings.hostingElsewhere')}</span>
    <button class="btn btn-ghost" onclick={() => ((ui.settingsProject = '*'), (ui.settings = 'projects'))}>{t('settings.claudeElsewhereDefaultsGo')}</button>
  </div>
</div>
{#if chosen}<InstallDialog entry={chosen} onclose={() => (chosen = null)} />{/if}

{#snippet logo(id: string, name: string)}
  {#if id === 'github'}
    <HostLogo host="github" size={18} />
  {:else}
    <ExtensionLogo {id} {name} size={18} />
  {/if}
{/snippet}

<style>
  .hosting {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h3 {
    margin: 6px 0 -2px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
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

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .option + .option {
    border-top: 1px solid var(--border);
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .option strong {
    font-size: 13px;
    font-weight: 600;
  }

  .option small {
    color: var(--text-dim);
    font-size: 12px;
  }

  .option small.bad {
    color: var(--err-text);
  }

  .option :global(.select) {
    flex: none;
    width: 260px;
  }

  .option .btn,
  .elsewhere .btn {
    flex: none;
    height: 32px;
  }

  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 6px;
    background: var(--surface-3);
  }

  .pill {
    margin-left: 8px;
    padding: 1px 7px;
    border-radius: 10px;
    background: var(--background-bg);
    color: var(--background);
    font-size: 11px;
    font-weight: 600;
    vertical-align: 1px;
  }

  .projects {
    margin-top: 2px;
    font: 11.5px var(--mono);
    color: var(--text-dim);
    overflow-wrap: anywhere;
  }

  .elsewhere {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .elsewhere span {
    flex: 1;
  }
</style>
