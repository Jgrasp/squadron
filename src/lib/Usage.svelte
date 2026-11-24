<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { tokens } from './format'
  import { date, errorText, number, t, type Key } from './i18n/index.svelte'
  import { detailsOf, fetchDetails, loadPlugins, plugins, toggle } from './plugins.svelte'
  import { patchSettings, settings } from './settings.svelte'
  import { daily, dayOf, GROUPS, groupTotal, lastDays, ranked, sum, uncalledPlugins, unusedPlugins, type Group, type Item, type Usage } from './stats'

  const PERIODS = [7, 30, 90]
  const VIEWS = ['source', 'project', 'model', 'context', 'hour'] as const
  const COMPACT = 200_000

  let data = $state<Usage | null>(null)
  let error = $state('')
  let period = $state(30)
  let day = $state<string | null>(null)
  let view = $state<(typeof VIEWS)[number]>('source')
  let more = $state('')
  let hovered = $state<string | null>(null)
  let width = $state(0)
  const open = new SvelteSet<Group>()
  let rows = $state<HTMLElement>()

  onMount(() => {
    invoke<Usage>('usage_stats')
      .then((d) => (data = d))
      .catch((e) => (error = errorText(e)))
    if (!plugins.installed) loadPlugins()
  })

  const days = $derived(lastDays(period))
  const inPeriod = $derived(new Set(days))
  const perDay = $derived(data ? daily(data.hours) : {})
  const totals = $derived(data ? sum(data.hours, (d) => (day ? d === day : inPeriod.has(d))) : null)
  const range = $derived(day ? [day, day] : [days[0], days.at(-1)!])
  const within = (at: number) => {
    const d = dayOf(new Date(at * 1000))
    return d >= range[0] && d <= range[1]
  }

  const percent = (part: number, whole: number) => {
    const r = whole ? part / whole : 0
    return number(r, { style: 'percent', maximumFractionDigits: r < 0.1 ? 1 : 0 })
  }
  const cap = (s: string) => s[0].toUpperCase() + s.slice(1)
  const dayLabel = (d: string) => date(new Date(`${d}T12:00:00`), { day: 'numeric', month: 'short' })
  const itemLabel = (item: Item) => ('fixed' in item ? t(`stats.item${cap(item.fixed)}` as Key) : item.name)
  const groupLabel = (g: Group) => t(`stats.group${cap(g)}` as Key)
  const ordered = $derived(
    totals
      ? GROUPS.map((g) => [g, groupTotal(totals, g)] as const)
          .filter(([, n]) => n > 0)
          .sort((a, b) => Number(a[0] === 'none') - Number(b[0] === 'none') || b[1] - a[1])
      : [],
  )

  const sessions = $derived(data ? data.sessions.filter((s) => within(s.end) || within(s.start)).slice(0, 6) : [])
  const bigs = $derived(data ? data.bigs.filter((b) => within(b.at)) : [])
  const uncalled = $derived(totals ? uncalledPlugins(totals, plugins.installed ?? []) : [])
  const unused = $derived(totals ? unusedPlugins(totals, plugins.installed ?? [], (id) => detailsOf(id)?.hooks) : [])
  $effect(() => fetchDetails(uncalled.map((x) => x.plugin.id)))

  const findings = $derived.by(() => {
    if (!totals) return []
    const all = totals.tokens
    const long = totals.contexts[3] + totals.contexts[4]
    const tools = ranked(totals, 'tools')
    const toolSum = groupTotal(totals, 'tools')
    const hooks = ranked(totals, 'hooks')
    const advisor = groupTotal(totals, 'advisor')
    const advisorCalls = totals.calls['Tool · advisor'] ?? 0
    const bigSum = bigs.reduce((s, b) => s + b.tokens * b.rereads, 0)
    return [
      {
        id: 'long',
        big: percent(long, all),
        small: tokens(long),
        title: t('stats.longTitle'),
        text: t('stats.longText', { share: percent(totals.contexts[4], all) }),
        show: long > 0,
      },
      {
        id: 'bigs',
        big: percent(bigSum, all),
        small: tokens(bigSum),
        title: t('stats.bigsTitle', { count: bigs.length }),
        text: bigs[0] ? t('stats.bigsText', { label: bigs[0].label, size: tokens(bigs[0].tokens), count: bigs[0].rereads }) : '',
        show: bigs.length > 0,
      },
      {
        id: 'advisor',
        big: percent(advisor, all),
        small: tokens(advisor),
        title: t('stats.advisorTitle'),
        text: t('stats.advisorText', { count: advisorCalls, each: tokens(advisorCalls ? advisor / advisorCalls : 0) }),
        show: advisor > 0,
        group: 'advisor' as Group,
      },
      {
        id: 'tools',
        big: percent(toolSum, all),
        small: tokens(toolSum),
        title: t('stats.toolsTitle'),
        text: t('stats.toolsText', { list: tools.slice(0, 2).map((x) => `${itemLabel(x.item)} ${percent(x.tokens, all)}`).join(', ') }),
        show: toolSum > 0,
        group: 'tools' as Group,
      },
      {
        id: 'hooks',
        big: percent(groupTotal(totals, 'hooks'), all),
        small: tokens(groupTotal(totals, 'hooks')),
        title: t('stats.hooksTitle'),
        text: t('stats.hooksText', { name: hooks[0] ? itemLabel(hooks[0].item) : '' }),
        show: hooks.length > 0,
        group: 'hooks' as Group,
      },
      {
        id: 'plugins',
        big: number(unused.length),
        small: t('stats.pluginsUnit', { count: unused.length }),
        title: t('stats.pluginsTitle'),
        text: t('stats.pluginsText'),
        show: unused.length > 0,
      },
    ].filter((f) => f.show)
  })

  function reveal(group: Group) {
    view = 'source'
    open.add(group)
    requestAnimationFrame(() => rows?.scrollIntoView({ behavior: 'smooth', block: 'center' }))
  }

  const list = $derived.by(() => {
    if (!totals) return []
    if (view === 'project' || view === 'model') {
      return Object.entries(view === 'project' ? totals.projects : totals.models)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 15)
    }
    if (view === 'context') {
      const edges = [50_000, 100_000, 200_000, 500_000]
      return totals.contexts.map((n, i) => [
        i === 0 ? t('stats.contextUnder', { n: tokens(edges[0]) }) : i === 4 ? t('stats.contextOver', { n: tokens(edges[3]) }) : t('stats.contextBetween', { from: tokens(edges[i - 1]), to: tokens(edges[i]) }),
        n,
      ] as [string, number])
    }
    return totals.hours.map((n, h) => [date(new Date(2000, 0, 1, h), { hour: 'numeric' }), n] as [string, number]).filter(([, n]) => n > 0)
  })

  const chart = $derived.by(() => {
    const left = 48
    const height = 140
    const w = Math.max(width - left, 1)
    const values = days.map((d) => perDay[d]?.tokens ?? 0)
    const max = Math.max(...values, 1)
    const step = [1e5, 2e5, 5e5, 1e6, 2e6, 5e6, 1e7, 2e7, 5e7, 1e8, 2e8, 5e8, 1e9, 2e9, 5e9, 1e10].find((s) => max / s <= 5) ?? 1e10
    const top = Math.ceil(max / step) * step
    const band = w / days.length
    const gap = Math.max(1, Math.min(4, band * 0.25))
    const every = days.length > 40 ? 14 : days.length > 10 ? 7 : 1
    return {
      left,
      height,
      w,
      ticks: Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step).map((v) => ({ v, y: height - (v / top) * height })),
      bars: days.map((d, i) => ({ d, i, x: left + i * band, band, h: (values[i] / top) * height, gap, label: (days.length - 1 - i) % every === 0 })),
    }
  })

  const tip = $derived(hovered ? perDay[hovered] : null)
</script>

<main>
  <div class="top">
    <div>
      <h1>{t('stats.title')}</h1>
      <span>{t('stats.subtitle', { count: days.filter((d) => perDay[d]).length })}</span>
    </div>
    <div class="segments" role="group" aria-label={t('stats.period')}>
      {#each PERIODS as n (n)}
        <button aria-pressed={period === n} onclick={() => ((period = n), (day = null))}>{t('stats.days', { count: n })}</button>
      {/each}
    </div>
  </div>

  <div class="scroll">
    {#if error}
      <p class="empty" role="alert">{error}</p>
    {:else if !totals}
      <p class="empty">{t('stats.reading')}</p>
    {:else}
      <div class="tiles">
        <div class="tile"><b>{tokens(totals.tokens)}</b><span>{day ? t('stats.tokensOn', { day: dayLabel(day) }) : t('stats.tokensUsed')}</span></div>
        <div class="tile"><b>{percent(totals.cacheRead, totals.tokens)}</b><span>{t('stats.fromCache')}</span></div>
        <div class="tile">
          {#if day}<b>{number(totals.turns)}</b><span>{t('stats.turns')}</span>
          {:else}<b>{tokens(totals.tokens / Math.max(totals.days.size, 1))}</b><span>{t('stats.perDay')}</span>{/if}
        </div>
        <div class="tile"><b>{tokens(totals.output)}</b><span>{t('stats.written')}</span></div>
      </div>

      <section class="card">
        <div class="caps">
          {t('stats.perDayTitle')}
          {#if day}
            <span class="chip">{t('stats.only', { day: dayLabel(day) })}<button aria-label={t('stats.clearDay')} onclick={() => (day = null)}>×</button></span>
          {:else}
            <span class="hint">{t('stats.pickDay')}</span>
          {/if}
        </div>
        <div class="chart" bind:clientWidth={width}>
          <svg viewBox="0 0 {chart.left + chart.w} {chart.height + 24}" role="img" aria-label={t('stats.perDayTitle')}>
            {#each chart.ticks as tick (tick.v)}
              <line class="grid" x1={chart.left} x2={chart.left + chart.w} y1={tick.y} y2={tick.y} />
              <text class="axis" x={chart.left - 8} y={tick.y + 4} text-anchor="end">{tick.v ? tokens(tick.v) : '0'}</text>
            {/each}
            {#each chart.bars as bar (bar.d)}
              {#if bar.h > 0}
                <rect class="bar" class:on={day === bar.d} class:hot={hovered === bar.d} class:off={day && day !== bar.d} x={bar.x + bar.gap / 2} y={chart.height - bar.h} width={bar.band - bar.gap} height={bar.h} rx={Math.min(3, (bar.band - bar.gap) / 2)} />
              {/if}
              <rect
                class="hit"
                role="button"
                tabindex="-1"
                aria-label={dayLabel(bar.d)}
                x={bar.x}
                y="0"
                width={bar.band}
                height={chart.height}
                onmouseenter={() => (hovered = bar.d)}
                onmouseleave={() => (hovered = null)}
                onclick={() => perDay[bar.d] && (day = day === bar.d ? null : bar.d)}
                onkeydown={() => {}}
              />
              {#if bar.label}<text class="axis" x={bar.x + bar.band / 2} y={chart.height + 18} text-anchor="middle">{dayLabel(bar.d)}</text>{/if}
            {/each}
          </svg>
          {#if hovered}
            {@const bar = chart.bars.find((b) => b.d === hovered)}
            <div class="tip" style:left="{Math.min((bar?.x ?? 0) + 12, width - 220)}px">
              {#if tip}<b>{tokens(tip.tokens)}</b> <span>· {dayLabel(hovered)} · {t('stats.turnsCount', { count: tip.turns })}</span>
              {:else}<span>{dayLabel(hovered)} · {t('stats.noActivity')}</span>{/if}
            </div>
          {/if}
        </div>
      </section>

      {#if findings.length}
        <section>
          <div class="caps">{t('stats.standsOut')}</div>
          <div class="findings">
            {#each findings as f (f.id)}
              <div class="finding">
                <div class="big">{f.big}<small>{f.small}</small></div>
                <div><h3>{f.title}</h3><p>{f.text}</p></div>
                <div class="acts">
                  {#if f.id === 'long'}
                    <button class="btn btn-ghost" class:done={settings.compact_window === COMPACT} disabled={settings.compact_window === COMPACT} onclick={() => patchSettings({ compact_window: COMPACT })}>
                      {settings.compact_window === COMPACT ? t('stats.compactsAt', { n: tokens(COMPACT) }) : t('stats.compactAt', { n: tokens(COMPACT) })}
                    </button>
                    <button class="btn btn-ghost" aria-expanded={more === f.id} onclick={() => (more = more === f.id ? '' : f.id)}>{t('stats.longest')}</button>
                  {:else if f.id === 'bigs'}
                    <button class="btn btn-ghost" aria-expanded={more === f.id} onclick={() => (more = more === f.id ? '' : f.id)}>{t('stats.showThem')}</button>
                  {:else if f.id === 'plugins'}
                    <button class="btn btn-ghost" aria-expanded={more === f.id} onclick={() => (more = more === f.id ? '' : f.id)}>{t('stats.review')}</button>
                  {:else if f.group}
                    {@const g = f.group}
                    <button class="btn btn-ghost" onclick={() => reveal(g)}>{t('stats.showInBreakdown')}</button>
                  {/if}
                </div>
                {#if more === f.id}
                  <div class="more">
                    {#if f.id === 'long'}
                      <div class="mini head"><span>{t('stats.conversation')}</span><span>{t('stats.started')}</span><span>{t('stats.turnsHead')}</span><span>{t('stats.peak')}</span><span>{t('stats.tokensHead')}</span></div>
                      {#each sessions as s (s.id)}
                        <div class="mini">
                          <span>{s.title || t('stats.untitled')} <em>· {s.project}</em></span><span>{date(s.start * 1000, { day: 'numeric', month: 'short' })}</span><span>{number(s.turns)}</span><span>{tokens(s.peak)}</span><span>{tokens(s.tokens)}</span>
                        </div>
                      {/each}
                    {:else if f.id === 'bigs'}
                      <div class="mini head"><span>{t('stats.output')}</span><span>{t('stats.size')}</span><span>{t('stats.rereads')}</span><span>{t('stats.project')}</span><span>{t('stats.cost')}</span></div>
                      {#each bigs.slice(0, 8) as b, i (i)}
                        <div class="mini">
                          <span title={b.label}>{b.label}</span><span>{tokens(b.tokens)}</span><span>× {number(b.rereads)}</span><span class="dim">{b.project}</span><span>{tokens(b.tokens * b.rereads)}</span>
                        </div>
                      {/each}
                    {:else if f.id === 'plugins'}
                      {#each unused as u (u.plugin.id)}
                        <div class="plug">
                          <span class="dot g-plugins"></span><span class="grow">{itemLabel(u.item)}</span><span class="num">{tokens(u.tokens)}</span>
                          <button class="btn btn-ghost" disabled={!!plugins.busy} onclick={() => toggle(u.plugin.id, false)}>{plugins.busy === u.plugin.id ? t('stats.disabling') : t('stats.disable')}</button>
                        </div>
                      {/each}
                      {#if plugins.error}<p class="bad" role="alert">{plugins.error}</p>{/if}
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <section>
        <div class="caps">
          {t('stats.whereItWent')}
          <div class="segments" role="group" aria-label={t('stats.groupBy')}>
            {#each VIEWS as v (v)}<button aria-pressed={view === v} onclick={() => (view = v)}>{t(`stats.by${cap(v)}` as Key)}</button>{/each}
          </div>
        </div>
        <div bind:this={rows}>
          {#if view === 'source'}
            <div class="progress" aria-hidden="true">
              {#each ordered as [g, n] (g)}<i class="g-{g}" style:width="{(100 * n) / totals.tokens}%"></i>{/each}
            </div>
            {#each ordered as [g, n] (g)}
              {@const items = ranked(totals, g)}
              <button class="row" aria-expanded={open.has(g)} onclick={() => (open.has(g) ? open.delete(g) : open.add(g))}>
                <span class="chev" class:on={open.has(g)} aria-hidden="true">▶</span>
                <span class="dot g-{g}"></span>
                <span class="name">{groupLabel(g)}<small>{items.slice(0, 3).map((x) => itemLabel(x.item)).join(' · ')}</small></span>
                <span class="bar"><i class="g-{g}" style:width="{(100 * n) / ordered[0][1]}%"></i></span>
                <span class="num">{tokens(n)}</span>
                <span class="pct">{percent(n, totals.tokens)}</span>
              </button>
              {#if open.has(g)}
                <div class="nested">
                  {#each items.slice(0, 12) as x ('fixed' in x.item ? x.item.fixed : x.item.name)}
                    <div class="row sub">
                      <span class="name">{itemLabel(x.item)}</span>
                      <span class="bar"><i class="g-{g}" style:width="{(100 * x.tokens) / items[0].tokens}%"></i></span>
                      <span class="num">{tokens(x.tokens)}</span>
                      <span class="pct">{percent(x.tokens, totals.tokens)}</span>
                    </div>
                  {/each}
                  {#if items.length > 12}<div class="row sub"><span class="name dim">{t('stats.more', { count: items.length - 12 })}</span></div>{/if}
                </div>
              {/if}
            {/each}
          {:else}
            {@const max = Math.max(...list.map(([, n]) => n), 1)}
            {#each list as [name, n] (name)}
              <div class="row">
                <span></span><span class="dot plain"></span><span class="name">{name}</span>
                <span class="bar"><i class="plain" style:width="{(100 * n) / max}%"></i></span>
                <span class="num">{tokens(n)}</span>
                <span class="pct">{percent(n, totals.tokens)}</span>
              </div>
            {/each}
          {/if}
        </div>
        <p class="foot">{t('stats.foot')}</p>
      </section>
    {/if}
  </div>
</main>

<style>
  main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-main);
  }

  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 24px 32px 16px;
    border-bottom: 1px solid var(--border);
  }

  h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 700;
  }

  .top span,
  .empty {
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 32px 32px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .segments {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
    text-transform: none;
    letter-spacing: 0;
    font-weight: 400;
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

  .tiles {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
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
    align-items: center;
    min-height: 30px;
    margin-bottom: 8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .card {
    padding: 14px 16px 10px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .hint {
    font-size: 12px;
    font-weight: 400;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-dim);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 4px 0 10px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    color: var(--text-2);
  }

  .chip button {
    width: 18px;
    height: 18px;
    border-radius: 3px;
    color: var(--text-dim);
  }

  .chip button:hover {
    background: var(--surface-4);
    color: var(--text);
  }

  .chart {
    position: relative;
    height: 164px;
  }

  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }

  .grid {
    stroke: var(--border);
  }

  .axis {
    fill: var(--text-faint);
    font: 11px var(--mono);
  }

  .bar {
    fill: var(--text-faint);
  }

  .bar.off {
    fill: var(--surface-5);
  }

  .bar.hot {
    fill: var(--text-muted);
  }

  .bar.on {
    fill: var(--text);
  }

  .hit {
    fill: transparent;
    cursor: pointer;
    outline: none;
  }

  .tip {
    position: absolute;
    top: -6px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 14px 36px rgba(0, 0, 0, 0.5),
      0 0 0 1px var(--border-strong);
    font-size: 12px;
    white-space: nowrap;
    pointer-events: none;
  }

  .tip b {
    font-family: var(--mono);
    font-weight: 500;
    color: var(--text-strong);
  }

  .tip span {
    color: var(--text-dim);
  }

  .findings {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .finding {
    display: grid;
    grid-template-columns: 96px 1fr auto;
    gap: 16px;
    align-items: start;
    padding: 14px 16px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .big {
    font-family: var(--mono);
    font-size: 22px;
    line-height: 1.1;
    color: var(--text-strong);
  }

  .big small {
    display: block;
    margin-top: 4px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  h3 {
    margin: 0 0 2px;
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .finding p {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .acts {
    display: flex;
    gap: 6px;
  }

  .btn-ghost.done {
    opacity: 1;
    color: var(--ok-text);
    box-shadow: inset 0 0 0 1px var(--ok-line);
  }

  .more {
    grid-column: 2 / 4;
    margin-top: 4px;
  }

  .mini {
    display: grid;
    grid-template-columns: 1fr 110px 80px 130px 90px;
    gap: 10px;
    align-items: center;
    height: 28px;
    padding: 0 8px;
    border-radius: 4px;
    font-size: 12.5px;
  }

  .mini:nth-child(even) {
    background: var(--surface-2);
  }

  .mini span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mini span:not(:first-child) {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text-2);
    text-align: right;
  }

  .mini em,
  .mini .dim,
  .dim {
    font-style: normal;
    color: var(--text-dim);
  }

  .mini.head span {
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .plug {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
  }

  .grow {
    flex: 1;
  }

  .bad {
    margin: 6px 0 0;
    font-size: 12.5px;
    color: var(--err-text);
  }

  .progress {
    display: flex;
    gap: 2px;
    height: 8px;
    margin-bottom: 10px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--surface-3);
  }

  .progress i {
    display: block;
    height: 100%;
  }

  .row {
    display: grid;
    grid-template-columns: 18px 9px 1fr 200px 72px 52px;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 30px;
    padding: 0 6px;
    border-radius: 4px;
    text-align: left;
  }

  .row:hover {
    background: var(--surface-1);
  }

  .row.sub {
    height: 26px;
    grid-template-columns: 1fr 200px 72px 52px;
  }

  .chev {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-dim);
    font-size: 9px;
    transition: transform 0.12s;
  }

  .chev.on {
    transform: rotate(90deg);
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }

  .name small {
    margin-left: 8px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .bar-track,
  .row .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .row .bar i {
    display: block;
    height: 100%;
    border-radius: 2px;
  }

  .num {
    font-family: var(--mono);
    font-size: 12.5px;
    color: var(--text-2);
    text-align: right;
  }

  .pct {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-dim);
    text-align: right;
  }

  .nested {
    margin: 0 0 4px 33px;
    padding-left: 10px;
    border-left: 1px solid var(--border-strong);
  }

  .foot {
    margin: 10px 0 0 6px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .g-claude {
    background: var(--text-faint);
  }

  .g-setup {
    background: var(--persona);
  }

  .g-mcp {
    background: var(--background);
  }

  .g-squadron {
    background: var(--remote-dot);
  }

  .g-plugins {
    background: var(--advisor);
  }

  .g-advisor {
    background: var(--advisor-text);
  }

  .g-tools,
  .g-replies,
  .g-messages {
    background: var(--ok);
  }

  .g-hooks {
    background: var(--ok-line);
  }

  .g-none {
    background: repeating-linear-gradient(135deg, var(--surface-5) 0 3px, var(--surface-3) 3px 6px);
  }

  .plain {
    background: var(--text-faint);
  }

  .row .bar i.plain {
    background: var(--text-muted);
  }
</style>
