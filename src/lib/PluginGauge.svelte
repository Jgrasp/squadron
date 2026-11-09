<script lang="ts">
  import { ui } from './actions.svelte'
  import { tokens } from './format'
  import { t } from './i18n/index.svelte'
  import { pluginColor } from './plugins'
  import { detailsOf, fetchDetails, openUsage } from './plugins.svelte'
  import { KINDS, shares, uses } from './pluginuse'
  import type { Agent } from './stream'

  let { agent }: { agent: Agent } = $props()

  let open = $state(false)

  const refs = $derived(agent.plugins ?? [])
  const list = $derived(shares(refs, uses(agent), (ref) => detailsOf(ref.id)?.always ?? 0))
  const total = $derived(list.reduce((s, x) => s + x.total, 0))
  const max = $derived(Math.max(1, ...list.map((x) => x.total)))
  const LEGEND = ['start', ...KINDS] as const

  $effect(() => {
    fetchDetails(refs.filter((r) => !r.id.endsWith('@inline')).map((r) => r.id))
  })

  const amount = (x: (typeof list)[number], kind: (typeof LEGEND)[number]) => (kind === 'start' ? x.start : x.kinds[kind])

  function settingsAt(anchor: string) {
    open = false
    ui.settingsAnchor = anchor
    ui.settings = 'plugins'
  }
</script>

<svelte:window onclick={(e) => open && !(e.target as HTMLElement).closest('[data-plugins]') && !(e.target as HTMLElement).closest('dialog') && (open = false)} onkeydown={(e) => e.key === 'Escape' && (open = false)} />

<span class="anchor" data-plugins>
  <button class="pill" aria-haspopup="dialog" aria-expanded={open} title={t('plugins.pillLabel')} onclick={() => (open = !open)}>
    <span class="mark" aria-hidden="true">◆</span>{t('plugins.pill', { count: refs.length })}<span class="mono">{tokens(total)}</span>
  </button>
  {#if open}
    <div class="panel" role="dialog" aria-label={t('plugins.pillLabel')}>
      <strong>{t('plugins.pillLabel')}</strong>
      <p class="sub">{t('plugins.panelTotal', { tokens: tokens(total), percent: agent.tokens ? Math.round((total / agent.tokens) * 100) : 0 })}</p>
      <div class="legend">
        {#each LEGEND as kind (kind)}
          <button class="key" onclick={() => openUsage(agent.id, { kind })}><i class="k-{kind}"></i>{t(kind === 'start' ? 'plugins.kindStart' : kind === 'skill' ? 'plugins.kindSkill' : kind === 'agent' ? 'plugins.kindAgent' : 'plugins.kindMcp')}</button>
        {/each}
      </div>
      <div class="bars">
        {#each list as x (x.ref.name)}
          <button class="bar" onclick={() => openUsage(agent.id, { plugin: x.ref.name })}>
            <span class="name"><span class="logo" style:background={pluginColor(x.ref.id)}>{x.ref.name.charAt(0).toUpperCase()}</span>{x.ref.name}</span>
            <span class="track"
              ><span class="fill" style:width="{(x.total / max) * 100}%"
                >{#each LEGEND as kind (kind)}{#if amount(x, kind)}<i class="k-{kind}" style:flex={amount(x, kind)}></i>{/if}{/each}</span
              ></span
            >
            <span class="mono">{tokens(x.total)}</span>
          </button>
        {:else}
          <p class="sub">{t('plugins.panelEmpty')}</p>
        {/each}
      </div>
      <p class="note">{t('plugins.estimate')}</p>
      <div class="foot">
        <button class="btn btn-secondary" onclick={() => settingsAt('display')}>{t('plugins.configure')}</button>
        <button class="btn btn-secondary" onclick={() => settingsAt('')}>{t('plugins.manage')}</button>
      </div>
    </div>
  {/if}
</span>

<style>
  .anchor {
    position: relative;
    display: inline-flex;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 8px;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 13px;
    white-space: nowrap;
  }

  .pill:hover,
  .pill[aria-expanded='true'] {
    background: var(--surface-3);
    color: var(--text);
  }

  .mark {
    color: var(--accent);
    font-size: 11px;
  }

  .mono {
    font-family: var(--mono);
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .panel {
    position: absolute;
    top: 34px;
    right: 0;
    z-index: 30;
    width: 440px;
    max-height: 70vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
    font-size: 13px;
    color: var(--text);
    white-space: normal;
  }

  strong {
    font-weight: 600;
    color: var(--text-strong);
  }

  p {
    margin: 0;
  }

  .sub,
  .note {
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .note {
    font-size: 11.5px;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
  }

  .key {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px;
    margin: 0 -6px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 11.5px;
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

  .bars {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 6px;
    border-radius: 4px;
    text-align: left;
  }

  .bar:hover {
    background: var(--surface-2);
  }

  .name {
    width: 160px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
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

  .track {
    flex: 1;
    height: 10px;
  }

  .fill {
    display: flex;
    gap: 1px;
    height: 100%;
    min-width: 4px;
    border-radius: 3px;
    overflow: hidden;
  }

  .fill i {
    display: block;
  }

  .bar .mono {
    width: 44px;
    text-align: right;
    color: var(--text-2);
  }

  .foot {
    display: flex;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .foot .btn {
    flex: 1;
    height: 30px;
    background: var(--surface-4);
  }
</style>
