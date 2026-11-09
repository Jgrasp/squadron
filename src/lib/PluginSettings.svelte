<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { ui } from './actions.svelte'
  import Dialog from './Dialog.svelte'
  import { t } from './i18n/index.svelte'
  import PluginProjects from './PluginProjects.svelte'
  import { ALL, OFFICIAL, marketplaceOf, nameOf, pluginColor, type Entry, type Installed } from './plugins'
  import { addMarketplace, choose, detailsOf, fetchDetails, install, loadCatalog, loadPlugins, plugins, remembered, removeMarketplace, toggle, uninstall, update, where } from './plugins.svelte'
  import { projects } from './projects.svelte'
  import { saveSettings, settings, type Config } from './settings.svelte'
  import { tokens } from './format'

  type From = 'official' | 'imported' | 'local'
  type Card = { id: string; name: string; version: string; description: string; marketplace: string; from: From; installed?: Installed; entry?: Entry }

  const FILTERS = [
    ['all', 'plugins.filterAll'],
    ['installed', 'plugins.filterInstalled'],
    ['discover', 'plugins.filterDiscover'],
    ['official', 'plugins.filterOfficial'],
    ['imported', 'plugins.filterImported'],
    ['local', 'plugins.filterLocal'],
  ] as const
  const SHOWS = [
    ['off', 'plugins.showOff', 'plugins.showOffHint'],
    ['tag', 'plugins.showTag', 'plugins.showTagHint'],
    ['card', 'plugins.showCard', 'plugins.showCardHint'],
    ['gutter', 'plugins.showGutter', 'plugins.showGutterHint'],
  ] as const
  const SHOWN = 60

  let query = $state('')
  let filter = $state<(typeof FILTERS)[number][0]>('installed')
  let shown = $state<string | null>(null)
  let picking = $state<{ id: string; install: boolean } | null>(null)
  let adding = $state(false)
  let source = $state('')
  let display: HTMLElement | undefined = $state()
  let lit = $state(false)

  const sources = $derived(new Map(plugins.marketplaces.map((m) => [m.name, m.source])))
  const fromOf = (marketplace: string): From => (marketplace === OFFICIAL ? 'official' : sources.get(marketplace) === 'directory' ? 'local' : 'imported')

  const cards = $derived.by((): Card[] => {
    const entries = new Map((plugins.catalog ?? []).map((e) => [e.id, e]))
    const mine = (plugins.installed ?? []).map((p): Card => {
      const entry = entries.get(p.id)
      const marketplace = marketplaceOf(p.id)
      return { id: p.id, name: p.name ?? nameOf(p.id), version: p.version, description: p.description ?? entry?.description ?? '', marketplace, from: fromOf(marketplace), installed: p }
    })
    const ids = new Set(mine.map((c) => c.id))
    const rest = [...entries.values()]
      .filter((e) => !ids.has(e.id))
      .sort((a, b) => (b.installs ?? 0) - (a.installs ?? 0))
      .map((e): Card => ({ id: e.id, name: e.name, version: '', description: e.description ?? '', marketplace: e.marketplace, from: fromOf(e.marketplace), entry: e }))
    return [...mine, ...rest]
  })
  const needle = $derived(query.trim().toLowerCase())
  const matching = $derived(
    cards.filter(
      (c) =>
        (filter === 'all' ||
          (filter === 'installed' && c.installed) ||
          (filter === 'discover' && !c.installed) ||
          filter === c.from) &&
        (!needle || `${c.name} ${c.id} ${c.description}`.toLowerCase().includes(needle)),
    ),
  )
  const visible = $derived([...matching.filter((c) => c.installed), ...matching.filter((c) => !c.installed).slice(0, SHOWN)])
  const hidden = $derived(matching.length - visible.length)
  const detail = $derived(cards.find((c) => c.id === shown))
  const picked = $derived(cards.find((c) => c.id === picking?.id))

  $effect(() => {
    if (filter !== 'installed' || needle) loadCatalog()
  })

  $effect(() => {
    if (plugins.installed) fetchDetails(plugins.installed.map((p) => p.id))
  })

  onMount(async () => {
    loadPlugins()
    if (ui.settingsAnchor !== 'display') return
    ui.settingsAnchor = ''
    await tick()
    display?.scrollIntoView({ block: 'start' })
    lit = true
    setTimeout(() => (lit = false), 1800)
  })

  function count(c: Card) {
    const w = where(c.id)
    const kept = remembered(c.id)
    if (c.installed && !c.installed.enabled && !w.on.length && kept) return { tone: 'off', n: kept.includes(ALL) ? -1 : kept.length, total: w.total }
    if (w.all) return { tone: 'all', n: -1, total: w.total }
    return { tone: w.on.length ? 'some' : 'none', n: w.on.length, total: w.total }
  }

  const isOn = (c: Card) => where(c.id).on.length > 0

  const setShow = (plugin_lines: Config['plugin_lines']) => saveSettings({ ...$state.snapshot(settings), plugin_lines })
  const setCard = (plugin_card: Config['plugin_card']) => saveSettings({ ...$state.snapshot(settings), plugin_card })

  function save(paths: string[]) {
    const target = picking
    picking = null
    if (!target) return
    if (target.install) install(target.id, paths.length ? paths : 'off')
    else choose(target.id, paths.length ? paths : 'off')
  }

  function addFrom(e: SubmitEvent) {
    e.preventDefault()
    if (!source.trim()) return
    addMarketplace(source.trim()).then(() => (source = ''))
  }
</script>

{#snippet badges(c: Card)}
  <span class="pill {c.from}">{c.from === 'official' ? `✓ ${t('plugins.official')}` : c.from === 'local' ? `▣ ${c.marketplace}` : `⇄ ${c.marketplace}`}</span>
{/snippet}

{#snippet scope(c: Card)}
  {@const k = count(c)}
  <button class="count {k.tone}" disabled={!!plugins.busy} onclick={() => (picking = { id: c.id, install: false })}>
    {#if k.tone === 'all'}
      <b>{t('plugins.everyProject')}</b>
    {:else if k.tone === 'off'}
      <b>{k.n < 0 ? t('plugins.everyProject') : k.n}</b>{k.n < 0 ? '' : t('plugins.projectsWord', { count: k.n })}<span>· {t('plugins.paused')}</span>
    {:else if k.tone === 'some'}
      <b>{k.n}</b>{t('plugins.projectsWord', { count: k.n })}<span>{t('plugins.outOf', { total: k.total })}</span>
    {:else}
      {t('plugins.noProject')}
    {/if}
  </button>
{/snippet}

<div class="top">
  <label class="search">
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
    <input placeholder={t('plugins.search')} aria-label={t('plugins.searchLabel')} spellcheck="false" autocomplete="off" bind:value={query} />
  </label>
  <div class="chips" role="group" aria-label={t('plugins.filters')}>
    {#each FILTERS as [id, key] (id)}
      <button class="chip" aria-pressed={filter === id} onclick={() => (filter = id)}>{t(key)}</button>
    {/each}
  </div>
</div>

<section class="display" class:lit bind:this={display}>
  <h3>{t('plugins.inConversation')}</h3>
  <p>{t('plugins.inConversationIntro')}</p>
  <div class="shows" role="radiogroup" aria-label={t('plugins.inConversation')}>
    {#each SHOWS as [id, label, hint] (id)}
      <button class="show" role="radio" aria-checked={settings.plugin_lines === id} onclick={() => setShow(id)}>
        <span class="pv {id}" class:minimal={settings.plugin_card === 'minimal'} aria-hidden="true"><i></i><i></i></span>
        <strong>{t(label)}</strong>
        <small>{t(hint)}</small>
      </button>
    {/each}
  </div>
  {#if settings.plugin_lines === 'card'}
    <div class="style">
      <span>{t('plugins.cardStyle')}</span>
      <div class="segments" role="group" aria-label={t('plugins.cardStyle')}>
        <button aria-pressed={settings.plugin_card !== 'minimal'} onclick={() => setCard('normal')}>{t('plugins.cardNormal')}</button>
        <button aria-pressed={settings.plugin_card === 'minimal'} onclick={() => setCard('minimal')}>{t('plugins.cardMinimal')}</button>
      </div>
    </div>
  {/if}
</section>

{#if plugins.changed}<div class="banner wait"><span>{t('plugins.restartHint')}</span></div>{/if}
{#if plugins.error}<p class="bad" role="alert">{plugins.error}</p>{/if}

<div class="grid">
  <button class="card new" onclick={() => (adding = true)}>
    <span class="plus" aria-hidden="true">+</span>
    <strong>{t('plugins.add')}</strong>
    <small>{t('plugins.addHint')}</small>
  </button>
  {#each visible as c (c.id)}
    <article class="card" class:off={c.installed && !isOn(c)}>
      <div class="head">
        <span class="logo" style:background={pluginColor(c.id)} aria-hidden="true">{c.name.charAt(0).toUpperCase()}</span>
        <button class="open" onclick={() => (shown = c.id)}><strong>{c.name}</strong>{#if c.version}<span class="version">{c.version}</span>{/if}</button>
        {#if c.installed}
          <input class="switch" type="checkbox" role="switch" checked={isOn(c)} disabled={!!plugins.busy} aria-label={t('plugins.enabledFor', { name: c.name })} onchange={(e) => toggle(c.id, e.currentTarget.checked)} />
        {/if}
      </div>
      <div class="badges">{@render badges(c)}</div>
      <p class="desc">{c.description}</p>
      <div class="foot">
        {#if c.installed}
          {@render scope(c)}
          {@const d = detailsOf(c.id)}
          <small class="mono">{d ? t('plugins.tokens', { count: tokens(d.always) }) : ''}</small>
        {:else}
          <small>{c.entry?.installs ? t('plugins.installs', { count: tokens(c.entry.installs) }) : ''}</small>
          <button class="btn btn-primary" disabled={!!plugins.busy} onclick={() => (picking = { id: c.id, install: true })}>
            {plugins.busy === c.id ? t('plugins.installing') : t('plugins.install')}
          </button>
        {/if}
      </div>
    </article>
  {/each}
  {#if plugins.loading && !plugins.installed}
    <p class="empty">{t('plugins.loading')}</p>
  {:else if filter !== 'installed' && !plugins.catalog}
    <p class="empty">{t('plugins.catalogLoading')}</p>
  {:else if !visible.length}
    <p class="empty">{needle ? t('plugins.noMatch', { query: query.trim() }) : t('plugins.noneHere')}</p>
  {/if}
  {#if hidden > 0}<p class="empty">{t('plugins.more', { count: hidden })}</p>{/if}
</div>

{#if detail}
  {@const installed = detail.installed}
  {@const d = installed ? detailsOf(detail.id) : null}
  {@const k = count(detail)}
  {@const w = where(detail.id)}
  <Dialog title={detail.name} width={680} onclose={() => (shown = null)}>
    {#snippet badge()}{#if detail.version}<span class="version">{detail.version}</span>{/if}<span class="badges">{@render badges(detail)}</span>{/snippet}
    <div class="sheet">
      {#if installed}
        <label class="enabled">{t('plugins.enabled')}<input class="switch" type="checkbox" role="switch" checked={isOn(detail)} disabled={!!plugins.busy} onchange={(e) => toggle(detail.id, e.currentTarget.checked)} /></label>
      {/if}
      {#if detail.description}<p class="about">{detail.description}</p>{/if}
      {#if installed}
        <div class="where">
          <div class="big">
            <b>{k.tone === 'all' ? t('plugins.everyShort') : w.on.length}</b>
            <span>{k.tone === 'all' ? t('plugins.everyProjectLong') : t('plugins.projectsOutOf', { count: w.on.length, total: w.total })}</span>
          </div>
          <p class="names">
            {#if k.tone === 'all'}{t('plugins.everyProjectHint')}{:else if w.on.length}{projects.list.filter((p) => w.on.includes(p.path)).map((p) => p.name).join(', ')}{:else}{t('plugins.noProjectHint')}{/if}
          </p>
          <button class="btn btn-primary" disabled={!!plugins.busy} onclick={() => (picking = { id: detail.id, install: false })}>{t('plugins.chooseProjects')}</button>
        </div>
        <div class="duo">
          <section>
            <h4>{t('plugins.contents')}</h4>
            {#if d}
              {#each [['skill', 'plugins.kindSkills'], ['agent', 'plugins.kindAgents'], ['mcp', 'plugins.kindMcp']] as const as [kind, label] (kind)}
                {@const names = d.components.filter((x) => x.kind === kind)}
                {#if names.length}<p class="line"><span>{t(label)}</span>{#each names as x (x.name)}<code>{x.name}</code>{/each}</p>{/if}
              {:else}
                <p class="line muted">{t('plugins.nothing')}</p>
              {/each}
            {:else}
              <p class="line muted">{t('plugins.loading')}</p>
            {/if}
          </section>
          <section>
            <h4>{t('plugins.cost')}</h4>
            {#if d}
              <p class="cost"><b>~{tokens(d.always)}</b> {t('plugins.costAlways')}</p>
              {#if d.components.length}<p class="line">{t('plugins.costInvoke', { count: tokens(Math.max(...d.components.map((x) => x.invoke))) })}</p>{/if}
            {:else}
              <p class="line muted">{t('plugins.loading')}</p>
            {/if}
          </section>
        </div>
      {/if}
      <div class="actions">
        <span class="mono muted">{detail.id}</span>
        <span class="grow"></span>
        {#if installed}
          <button class="btn btn-secondary" disabled={!!plugins.busy} onclick={() => update(installed)}>{t('plugins.update')}</button>
          <button class="btn danger" disabled={!!plugins.busy} onclick={() => uninstall(installed).then(() => (shown = null))}>{t('plugins.uninstall')}</button>
        {:else}
          <button class="btn btn-primary" disabled={!!plugins.busy} onclick={() => (picking = { id: detail.id, install: true })}>{t('plugins.install')}</button>
        {/if}
      </div>
    </div>
  </Dialog>
{/if}

{#if picking && picked}
  {@const install = picking.install}
  {@const w = where(picked.id)}
  {@const kept = remembered(picked.id)}
  <PluginProjects
    title={install ? t('plugins.installTitle', { name: picked.name }) : t('plugins.projectsTitle', { name: picked.name })}
    intro={install ? t('plugins.installIntro') : t('plugins.projectsIntro')}
    chosen={install ? projects.list.filter((p) => p.path === projects.current).map((p) => p.path) : w.on.length ? w.on : kept && !kept.includes(ALL) ? kept : kept ? projects.list.map((p) => p.path) : []}
    confirm={(n) => (install ? t('plugins.installIn', { count: n }) : t('common.save'))}
    onsave={save}
    onclose={() => (picking = null)}
  />
{/if}

{#if adding}
  <Dialog title={t('plugins.add')} width={580} onclose={() => (adding = false)}>
    <div class="ways">
      <button class="way" onclick={() => ((filter = 'discover'), (query = ''), (adding = false))}>
        <span class="ico" aria-hidden="true">◎</span>
        <span class="text"><strong>{t('plugins.fromCatalog')}</strong><small>{t('plugins.fromCatalogHint')}</small></span>
        <span class="btn btn-secondary">{t('plugins.browse')}</span>
      </button>
      <div class="way column">
        <div class="row">
          <span class="ico" aria-hidden="true">⇄</span>
          <span class="text"><strong>{t('plugins.marketplaces')}</strong><small>{t('plugins.marketplacesHint')}</small></span>
        </div>
        <ul class="sources">
          {#each plugins.marketplaces as m (m.name)}
            <li>
              <span class="mono">{m.name}</span>
              <small class="muted">{m.repo ?? m.url ?? m.path ?? m.source}</small>
              {#if m.name === OFFICIAL}<span class="pill official">✓ {t('plugins.official')}</span>{:else}<button class="btn danger" disabled={!!plugins.busy} onclick={() => removeMarketplace(m.name)}>{t('common.remove')}</button>{/if}
            </li>
          {/each}
        </ul>
        <form class="add" onsubmit={addFrom}>
          <input class="field mono" placeholder={t('plugins.marketplacePlaceholder')} aria-label={t('plugins.marketplacePlaceholder')} spellcheck="false" autocomplete="off" bind:value={source} />
          <button class="btn btn-secondary" disabled={!source.trim() || !!plugins.busy}>{plugins.busy === 'marketplace' ? t('common.checking') : t('plugins.marketplaceAdd')}</button>
        </form>
        {#if plugins.error}<p class="bad" role="alert">{plugins.error}</p>{/if}
      </div>
    </div>
  </Dialog>
{/if}

<style>
  svg {
    flex: none;
    fill: none;
    stroke: var(--text-dim);
    stroke-width: 2;
  }

  .top {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: none;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-size: 12px;
  }

  .chip[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  .display {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    transition: box-shadow 0.3s, background 0.3s;
  }

  .display.lit {
    background: color-mix(in srgb, var(--accent) 7%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .display h3 {
    margin: 0;
    font-size: 13.5px;
  }

  .display p {
    margin: 0;
    color: var(--text-dim);
    font-size: 13px;
  }

  .shows {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
  }

  .show {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  .show:hover {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .show[aria-checked='true'] {
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }

  .show strong {
    font-size: 13px;
    color: var(--text-strong);
  }

  .show small {
    font-size: 11.5px;
    line-height: 1.35;
    color: var(--text-dim);
  }

  .pv {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    height: 44px;
    margin-bottom: 6px;
    padding: 0 8px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .pv i {
    position: relative;
    display: block;
    height: 5px;
    margin-left: 18px;
    border-radius: 3px;
    background: var(--surface-5);
  }

  .pv i::before {
    content: '';
    position: absolute;
    left: -18px;
    top: 0;
    width: 13px;
    height: 5px;
    border-radius: 3px;
    background: var(--text-faint);
  }

  .pv.tag i:first-child::before {
    background: var(--persona);
  }

  .pv.card i:first-child {
    height: 14px;
    margin-left: 0;
    background: color-mix(in srgb, var(--persona) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--persona) 45%, transparent);
  }

  .pv.card.minimal i:first-child {
    height: 9px;
    box-shadow: inset 2px 0 0 var(--persona);
  }

  .pv.card i:first-child::before,
  .pv.gutter i:first-child::before {
    left: 4px;
    top: 3px;
    width: 8px;
    height: 8px;
    background: var(--persona);
  }

  .pv.card.minimal i:first-child::before {
    top: 1px;
    width: 7px;
    height: 7px;
  }

  .pv.gutter i:first-child::before {
    left: -16px;
    top: -1px;
  }

  .style {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .segments {
    display: flex;
    padding: 2px;
    border-radius: 5px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .segments button {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
  }

  .banner.wait {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--wait-text);
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .btn-secondary {
    background: var(--surface-4);
  }

  .btn.danger {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px var(--err-border);
    color: var(--err-text);
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }

  .switch:focus-visible {
    outline: 2px solid var(--border-focus);
    outline-offset: 2px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 10px;
  }

  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  .card:hover {
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .card.off .head,
  .card.off .desc {
    opacity: 0.55;
  }

  .card.new {
    align-items: center;
    justify-content: center;
    min-height: 150px;
    gap: 4px;
    text-align: center;
    background: none;
    box-shadow: none;
    border: 1px dashed var(--border-dashed);
  }

  .card.new:hover {
    border-color: var(--border-focus);
    background: var(--surface-1);
  }

  .card.new .plus {
    font-size: 24px;
    line-height: 1;
    color: var(--accent);
  }

  .card.new strong {
    color: var(--text-strong);
  }

  .card.new small {
    color: var(--text-dim);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 7px;
    color: var(--ink);
    font-weight: 700;
  }

  .open {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 6px;
  }

  .open strong {
    color: var(--text-strong);
  }

  .switch,
  .foot .btn,
  .count {
    position: relative;
    z-index: 1;
  }

  .version {
    margin-left: 6px;
    font: 400 12px var(--mono);
    color: var(--text-dim);
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 9px;
    border-radius: 4px;
    font-size: 11.5px;
    font-weight: 700;
    white-space: nowrap;
  }

  .pill.official {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
    color: var(--ok);
  }

  .pill.imported {
    background: var(--option-off);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
  }

  .pill.local {
    background: var(--background-bg);
    box-shadow: inset 0 0 0 1px var(--background-line);
    color: var(--remote-text);
  }

  .desc {
    margin: 0;
    min-height: 36px;
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.45;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
  }

  .foot small {
    color: var(--text-dim);
  }

  .mono {
    font-family: var(--mono);
  }

  .muted {
    color: var(--text-dim);
  }

  .count {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, transparent);
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 30px;
    white-space: nowrap;
  }

  .count b {
    font-size: 15px;
    color: var(--accent-soft);
  }

  .count span {
    color: var(--text-dim);
  }

  .count:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .count.all {
    background: var(--ok-bg);
    box-shadow: inset 0 0 0 1px var(--ok-line);
  }

  .count.all b {
    font-size: 13px;
    color: var(--ok);
  }

  .count.off,
  .count.none {
    background: none;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-dim);
  }

  .count.off b {
    color: var(--text-dim);
  }

  .empty,
  .bad {
    margin: 0;
    color: var(--text-dim);
  }

  .grid .empty {
    grid-column: 1 / -1;
    padding: 12px;
    text-align: center;
  }

  .bad {
    color: var(--err-text);
  }

  .sheet {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px 20px 20px;
  }

  .enabled {
    align-self: flex-end;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: -6px;
    color: var(--text-muted);
  }

  .about {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.55;
  }

  .where {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 16px 18px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 7%, var(--surface-1));
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .big {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 84px;
    text-align: center;
  }

  .big b {
    font-size: 30px;
    line-height: 1;
    color: var(--accent-soft);
  }

  .big span {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .names {
    flex: 1;
    min-width: 0;
    margin: 0;
    padding-left: 20px;
    border-left: 1px solid var(--border-strong);
    color: var(--text-muted);
    font-size: 12.5px;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .duo {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .duo section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  h4 {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .line {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .line span {
    width: 56px;
    color: var(--text-dim);
  }

  .line code {
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-4);
    font-size: 12px;
  }

  .cost {
    margin: 0;
    color: var(--text-muted);
  }

  .cost b {
    font-size: 18px;
    color: var(--text-strong);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }

  .grow {
    flex: 1;
  }

  .ways {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 4px 20px 20px;
  }

  .way {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    text-align: left;
  }

  .way.column {
    flex-direction: column;
    align-items: stretch;
  }

  .way .row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .ico {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 7px;
    background: var(--surface-3);
    color: var(--accent);
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .text small {
    color: var(--text-dim);
  }

  .sources {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .sources li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-radius: 4px;
    background: var(--bg-main);
  }

  .sources small {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .add {
    display: flex;
    gap: 8px;
  }

  .add .field {
    flex: 1;
  }
</style>
