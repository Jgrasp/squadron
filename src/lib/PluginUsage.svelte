<script lang="ts">
  import Dialog from './Dialog.svelte'
  import { time, tokens } from './format'
  import { t, type Key } from './i18n/index.svelte'
  import { pluginColor } from './plugins'
  import { closeUsage, detailsOf, openUsage, reveal, skillCost, usage, type View } from './plugins.svelte'
  import { KINDS, shares, uses, type Kind, type Use } from './pluginuse'
  import type { Agent } from './stream'

  let { agent }: { agent: Agent } = $props()

  const NAMES: Record<Kind | 'start', Key> = { start: 'plugins.kindStart', skill: 'plugins.kindSkill', agent: 'plugins.kindAgent', mcp: 'plugins.kindMcp' }
  const PARTS: Record<Use['parts'][number]['key'], Key> = {
    skillBody: 'plugins.skillBody',
    agentPrompt: 'plugins.agentPrompt',
    agentReport: 'plugins.agentReport',
    request: 'plugins.request',
    answer: 'plugins.answer',
    file: 'plugins.file',
  }
  const ORDER = ['start', ...KINDS] as const

  const refs = $derived(agent.plugins ?? [])
  const list = $derived(uses(agent, skillCost(refs)))
  const all = $derived(shares(refs, list, (ref) => detailsOf(ref.id)?.always ?? 0))
  const sum = $derived(all.reduce((s, x) => s + x.total, 0))
  const view = $derived(usage.stack.at(-1) as View)
  const nested = $derived(usage.stack.length > 1)
  const refOf = (name: string) => refs.find((r) => r.name === name)
  const colorOf = (name: string) => pluginColor(refOf(name)?.id ?? name)

  const resolved = $derived.by(() => {
    if ('kind' in view && view.kind !== 'start' && view.plugin) {
      const own = list.filter((u) => u.kind === view.kind && u.plugin === view.plugin)
      if (own.length === 1) return { use: own[0].id }
    }
    return view
  })
  const use = $derived('use' in resolved ? list.find((u) => u.id === resolved.use) : undefined)
  const title = $derived(use ? use.label : 'plugin' in resolved && !('kind' in resolved) ? resolved.plugin : 'kind' in resolved ? t(NAMES[resolved.kind]) : '')

  const go = (next: View) => openUsage(agent.id, next, true)
  const back = () => {
    usage.stack = usage.stack.slice(0, -1)
  }
  const show = (id: string) => {
    closeUsage()
    setTimeout(() => reveal(id), 50)
  }
</script>

{#snippet logo(name: string, size = 'sm')}
  <span class="logo {size}" style:background={colorOf(name)} aria-hidden="true">{name.charAt(0).toUpperCase()}</span>
{/snippet}

{#snippet row(u: Use, withPlugin: boolean)}
  <button class="line" onclick={() => go({ use: u.id })}>
    <span class="when">{u.at ? time(u.at) : ''}</span>
    {#if withPlugin}{@render logo(u.plugin)}{/if}
    <span class="kind">{t(NAMES[u.kind])}</span>
    <span class="what">{u.label}</span>
    <span class="n" class:hot={u.tokens >= 1000}>{tokens(u.tokens)}</span>
    <span class="go" aria-hidden="true">›</span>
  </button>
{/snippet}

<Dialog {title} width={560} onclose={closeUsage}>
  {#snippet badge()}
    {#if nested}<button class="back" onclick={back}>‹ {t('plugins.back')}</button>{/if}
  {/snippet}
  <div class="body">
    {#if use}
      {@const same = list.filter((u) => u.plugin === use.plugin && u.kind === use.kind && u.label === use.label).length}
      <p class="sub">{@render logo(use.plugin)}{use.plugin} · {t(NAMES[use.kind])}</p>
      <div class="stats">
        <div><b class="hot">{tokens(use.tokens)}</b><span>{t('plugins.added')}</span></div>
        <div><b>{use.at ? time(use.at) : '—'}</b><span>{t('plugins.when')}</span></div>
        <div><b>{same}×</b><span>{t('plugins.times', { count: same })}</span></div>
      </div>
      <h4>{t('plugins.loaded')}</h4>
      {@const top = Math.max(1, ...use.parts.map((p) => p.tokens))}
      <div class="parts">
        {#each use.parts as part, i (i)}
          <div class="part">
            <span class="what">{part.label ?? t(PARTS[part.key])}</span>
            <span class="meter"><i class="k-{use.kind}" style:width="{(part.tokens / top) * 100}%"></i></span>
            <span class="n">{tokens(part.tokens)}</span>
          </div>
        {/each}
      </div>
      {#if use.excerpt}
        <h4>{t('plugins.excerpt')}</h4>
        <pre>{use.excerpt}</pre>
      {/if}
      <div class="foot">
        <span class="note">{t('plugins.estimate')}</span>
        <button class="btn btn-secondary" onclick={closeUsage}>{t('common.close')}</button>
        {#if !use.id.startsWith('command-')}<button class="btn btn-primary" onclick={() => show(use.id)}>{t('plugins.seeInConversation')}</button>{/if}
      </div>
    {:else if 'kind' in resolved && resolved.kind === 'start'}
      {@const names = resolved.plugin ? [resolved.plugin] : all.map((x) => x.ref.name)}
      {@const rows = names.flatMap((name) => (detailsOf(refOf(name)?.id ?? '')?.components ?? []).map((c) => ({ name, c })))}
      {@const total = names.reduce((s, name) => s + (detailsOf(refOf(name)?.id ?? '')?.always ?? 0), 0)}
      {@const top = Math.max(1, ...rows.map((r) => r.c.always))}
      {#if resolved.plugin}<p class="sub">{@render logo(resolved.plugin)}{resolved.plugin}</p>{/if}
      <div class="stats two">
        <div><b class="hot">{tokens(total)}</b><span>{t('plugins.startupTotal')}</span></div>
        <div><b>{rows.length}</b><span>{t('plugins.startupCount', { count: rows.length })}</span></div>
      </div>
      <h4>{t('plugins.loaded')}</h4>
      <div class="parts">
        {#each rows as r, i (i)}
          <div class="part">
            {#if !resolved.plugin}{@render logo(r.name)}{/if}
            <span class="kind">{t(NAMES[r.c.kind])}</span>
            <span class="what">{r.c.name}</span>
            <span class="meter"><i class="k-start" style:width="{(r.c.always / top) * 100}%"></i></span>
            <span class="n">{tokens(r.c.always)}</span>
          </div>
        {:else}
          <p class="sub">{t('plugins.nothing')}</p>
        {/each}
      </div>
      <div class="foot"><span class="note">{t('plugins.estimate')}</span><button class="btn btn-secondary" onclick={closeUsage}>{t('common.close')}</button></div>
    {:else if 'kind' in resolved}
      {@const kind = resolved.kind}
      {@const own = list.filter((u) => u.kind === kind && (!resolved.plugin || u.plugin === resolved.plugin))}
      {#if resolved.plugin}<p class="sub">{@render logo(resolved.plugin)}{resolved.plugin}</p>{/if}
      <div class="stats two">
        <div><b class="hot">{tokens(own.reduce((s, u) => s + u.tokens, 0))}</b><span>{t('plugins.conversationTokens')}</span></div>
        <div><b>{own.length}×</b><span>{t('plugins.activations', { count: own.length })}</span></div>
      </div>
      <h4>{t('plugins.triggers')}</h4>
      <div class="lines">
        {#each own as u (u.id)}{@render row(u, !resolved.plugin)}{:else}<p class="sub">{t('plugins.noneCalled')}</p>{/each}
      </div>
      <div class="foot"><span class="note">{t('plugins.estimate')}</span><button class="btn btn-secondary" onclick={closeUsage}>{t('common.close')}</button></div>
    {:else if 'plugin' in resolved}
      {@const x = all.find((s) => s.ref.name === resolved.plugin)}
      {#if x}
        {@const amount = (k: (typeof ORDER)[number]) => (k === 'start' ? x.start : x.kinds[k])}
        <p class="sub">{@render logo(x.ref.name)}{t('plugins.consumption')}</p>
        <div class="stats">
          <div><b class="hot">{tokens(x.total)}</b><span>{t('plugins.totalTokens')}</span></div>
          <div><b>{sum ? Math.round((x.total / sum) * 100) : 0} %</b><span>{t('plugins.share')}</span></div>
          <div><b>{x.uses.length}×</b><span>{t('plugins.activations', { count: x.uses.length })}</span></div>
        </div>
        <div class="stack">
          {#each ORDER as k (k)}{#if amount(k)}<button class="k-{k}" style:flex={amount(k)} title="{t(NAMES[k])} · {tokens(amount(k))}" aria-label={t(NAMES[k])} onclick={() => go({ kind: k, plugin: x.ref.name })}></button>{/if}{/each}
        </div>
        <div class="legend">
          {#each ORDER as k (k)}
            {#if amount(k)}<button class="key" onclick={() => go({ kind: k, plugin: x.ref.name })}><i class="k-{k}"></i>{t(NAMES[k])}<b>{tokens(amount(k))}</b></button>{/if}
          {/each}
        </div>
        <h4>{t('plugins.lineByLine')}</h4>
        <div class="lines">
          <button class="line" onclick={() => go({ kind: 'start', plugin: x.ref.name })}>
            <span class="when"></span>
            <span class="kind">{t('plugins.kindStart')}</span>
            <span class="what plain">{t('plugins.startupLine')}</span>
            <span class="n">{tokens(x.start)}</span>
            <span class="go" aria-hidden="true">›</span>
          </button>
          {#each x.uses as u (u.id)}{@render row(u, false)}{/each}
        </div>
        <div class="foot"><span class="note">{t('plugins.estimate')}</span><button class="btn btn-secondary" onclick={closeUsage}>{t('common.close')}</button></div>
      {/if}
    {/if}
  </div>
</Dialog>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 20px 18px;
  }

  .back {
    flex: none;
    margin-left: 8px;
    padding: 2px 8px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .back:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .sub {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .logo {
    flex: none;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    color: var(--ink);
    font-size: 10px;
    font-weight: 700;
  }

  .stats {
    display: grid;
    grid-template-columns: 1fr 1.3fr 1fr;
    gap: 8px;
  }

  .stats.two {
    grid-template-columns: 1fr 1fr;
  }

  .stats div {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .stats b {
    font: 600 16px var(--mono);
    color: var(--text-strong);
  }

  .stats b.hot {
    font-size: 20px;
    color: var(--accent-soft);
  }

  .stats span {
    font-size: 12px;
    line-height: 1.35;
    color: var(--text-dim);
  }

  h4 {
    margin: 6px 0 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .stack {
    display: flex;
    gap: 2px;
    height: 12px;
    border-radius: 4px;
    overflow: hidden;
  }

  .stack button {
    display: block;
    height: 100%;
    padding: 0;
    border-radius: 0;
  }

  .stack button:hover {
    filter: brightness(1.25);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
  }

  .key {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 6px;
    margin: 0 -6px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .key:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .key i {
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }

  .key b {
    font: 12px var(--mono);
    color: var(--text-2);
  }

  .k-start {
    background: var(--text-faint);
  }

  .k-skill {
    background: var(--accent);
  }

  .k-agent {
    background: var(--ok);
  }

  .k-mcp {
    background: var(--background);
  }

  .lines,
  .parts {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .line,
  .part {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    padding: 0 6px;
    margin: 0 -6px;
    border-radius: 4px;
    text-align: left;
    font-size: 12.5px;
  }

  .line:hover {
    background: var(--surface-2);
  }

  .when {
    width: 40px;
    flex: none;
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .kind {
    width: 92px;
    flex: none;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
    color: var(--text-2);
  }

  .what.plain {
    font-family: inherit;
    color: var(--text-muted);
  }

  .part .what {
    flex: none;
    width: 200px;
  }

  .meter {
    flex: 1;
    height: 8px;
    border-radius: 4px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .meter i {
    display: block;
    height: 100%;
    border-radius: 4px;
  }

  .n {
    width: 44px;
    flex: none;
    text-align: right;
    font: 12.5px var(--mono);
    color: var(--text-2);
  }

  .n.hot {
    color: var(--accent-soft);
  }

  .go {
    width: 10px;
    color: var(--text-faint);
  }

  pre {
    margin: 0;
    padding: 10px 12px;
    max-height: 160px;
    overflow: auto;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font: 12px/1.5 var(--mono);
    color: var(--text-muted);
    white-space: pre-wrap;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 6px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .note {
    flex: 1;
    font-size: 11.5px;
    color: var(--text-faint);
  }

  .btn {
    height: 30px;
    padding: 0 12px;
  }

  .btn-secondary {
    background: var(--surface-4);
  }
</style>
