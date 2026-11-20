<script lang="ts">
  import { REVIEW_PROMPT } from './pulls.svelte'
  import { projects } from './projects.svelte'
  import { names } from './pipelines.svelte'
  import { saveSettings, settings } from './settings.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import HostLogo from './HostLogo.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import InstallDialog from './InstallDialog.svelte'
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

  const LINGERS = [1, 2, 5, 10, 15, 30, 60]
  let hide = $state('')
  const seen = $derived([...new Set(projects.list.flatMap(names))].filter((n) => !settings.runs_hidden.includes(n)).sort())
  const change = (changes: Partial<typeof settings>) => saveSettings({ ...$state.snapshot(settings), ...changes }).catch((e) => (error = errorText(e)))

  function hideName(event: Event) {
    event.preventDefault()
    const name = hide.trim()
    if (name && !settings.runs_hidden.includes(name)) change({ runs_hidden: [...settings.runs_hidden, name] })
    hide = ''
  }
</script>

<p class="intro">{t('settings.hostingIntro')}</p>

<section>
  <h4>{t('settings.forges')}</h4>
  <div class="rows">
    {#each providers() as e (e.id)}
      {@const state = stateOf(e)}
      <div class="line">
        <span class="mark"><ExtensionLogo id={e.id} name={nameOf(e)} logo={e.logo} size={18} /></span>
        <span class="what">
          <strong>{nameOf(e)}<span class="pill ext">{t('settings.forgeExtension')}</span></strong>
          <small class:bad={state.bad}>{state.text}</small>
        </span>
        <button class="btn btn-ghost" onclick={() => (ui.settings = `ext:${e.id}`)}>{t('settings.forgeAccounts')}</button>
      </div>
    {/each}
    {#each installOffers() as offer (offer.entry.id)}
      <div class="line offer">
        <span class="mark">{@render logo(offer.entry.id, named(offer.entry.name))}</span>
        <span class="what">
          <strong>{named(offer.entry.name)}<span class="pill ext">{t('settings.forgeExtension')}</span></strong>
          <small>{t('settings.forgeWaiting', { count: offer.projects.length, host: offer.host })}</small>
          <span class="projects">{offer.projects.join(' · ')}</span>
        </span>
        <button class="btn btn-primary" onclick={() => (chosen = offer.entry)}>{t('settings.forgeInstall', { name: named(offer.entry.name) })}</button>
      </div>
    {/each}
    <button class="discover" onclick={() => (ui.settings = 'discover')}>{t('settings.forgeDiscover')}</button>
  </div>
</section>
{#if chosen}<InstallDialog entry={chosen} onclose={() => (chosen = null)} />{/if}

{#snippet logo(id: string, name: string)}
  {#if id === 'github'}
    <HostLogo host="github" size={18} />
  {:else}
    <ExtensionLogo {id} {name} size={18} />
  {/if}
{/snippet}
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<section>
  <h3>{t('review.mergeOthers')}</h3>
  <p class="intro">{t('review.mergeOthersIntro')}</p>
  <select class="field" aria-label={t('review.mergeOthers')} value={settings.merge_others} onchange={(e) => change({ merge_others: e.currentTarget.value as typeof settings.merge_others })}>
    <option value="second">{t('review.mergeOthersSecond')}</option>
    <option value="menu">{t('review.mergeOthersMenu')}</option>
    <option value="hidden">{t('review.mergeOthersHidden')}</option>
  </select>
</section>

<section>
  <h3>{t('review.promptTitle')}</h3>
  <p class="intro">{t('review.promptIntro')}</p>
  <textarea
    class="field prompt"
    rows="5"
    aria-label={t('review.promptTitle')}
    value={settings.review_prompt || REVIEW_PROMPT}
    onchange={(e) => change({ review_prompt: e.currentTarget.value.trim() === REVIEW_PROMPT ? '' : e.currentTarget.value.trim() })}
  ></textarea>
  {#if settings.review_prompt}
    <div class="row"><button class="btn btn-secondary" onclick={() => change({ review_prompt: '' })}>{t('review.promptReset')}</button></div>
  {/if}
</section>

<section>
  <h3>{t('runs.title')}</h3>
  <p class="intro">{t('runs.settingsIntro')}</p>
  <label class="check">
    <input type="checkbox" checked={settings.runs} onchange={(e) => change({ runs: e.currentTarget.checked })} />
    <span>
      <strong>{t('runs.show')}</strong>
      <small>{t('runs.showHint')}</small>
    </span>
  </label>
  <label for="runs-linger">{t('runs.linger')}</label>
  <select id="runs-linger" class="field" disabled={!settings.runs} value={settings.runs_linger} onchange={(e) => change({ runs_linger: Number(e.currentTarget.value) })}>
    {#each LINGERS as minutes (minutes)}<option value={minutes}>{t('runs.lingerMinutes', { count: minutes })}</option>{/each}
  </select>
  <span class="label">{t('runs.hiddenEverywhere')}</span>
  {#if settings.runs_hidden.length}
    <span class="names">
      {#each settings.runs_hidden as name (name)}
        <button class="name" aria-label={t('runs.unhide', { name })} title={t('runs.unhide', { name })} onclick={() => change({ runs_hidden: settings.runs_hidden.filter((n) => n !== name) })}>
          {name}<span aria-hidden="true">×</span>
        </button>
      {/each}
    </span>
  {:else}
    <p class="intro">{t('runs.hiddenNone')}</p>
  {/if}
  <form class="row" onsubmit={hideName}>
    <input class="field" list="runs-seen" placeholder={t('runs.hideName')} aria-label={t('runs.hideName')} bind:value={hide} />
    <datalist id="runs-seen">{#each seen as name (name)}<option value={name}></option>{/each}</datalist>
    <button class="btn btn-secondary" disabled={!hide.trim()}>{t('common.add')}</button>
  </form>
</section>

<style>
  .check {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .check input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .check span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check strong {
    font-weight: 600;
  }

  .check small {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-dim);
  }

  .label {
    font-weight: 600;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .name:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  form.row {
    flex-direction: row;
  }

  .row .field {
    flex: 1;
    height: 34px;
  }

  .row .btn {
    height: 34px;
    padding: 0 14px;
  }

  section,
  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  h3 {
    margin: 10px 0 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  h4 {
    margin: 14px 0 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .line.offer {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .line .btn {
    height: 30px;
    padding: 0 12px;
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

  .what {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .what strong {
    font-weight: 600;
  }

  .what small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .what small.bad {
    color: var(--err-text);
  }

  .pill {
    margin-left: 8px;
    padding: 1px 7px;
    border-radius: 10px;
    font-size: 11px;
    font-weight: 600;
    vertical-align: 1px;
  }

  .pill.ext {
    background: var(--background-bg);
    color: var(--background);
  }

  .projects {
    margin-top: 2px;
    font: 11.5px var(--mono);
    color: var(--text-dim);
    overflow-wrap: anywhere;
  }

  .discover {
    height: 34px;
    border: 0;
    border-radius: 6px;
    background: none;
    box-shadow: inset 0 0 0 1px var(--border-dashed);
    color: var(--text-dim);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }

  .discover:hover {
    color: var(--text-2);
    background: var(--surface-1);
  }

  label {
    font-weight: 600;
  }

  .prompt {
    height: auto;
    padding: 8px 10px;
    resize: vertical;
    line-height: 1.5;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }
</style>
