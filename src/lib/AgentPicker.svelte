<script lang="ts">
  import { ABILITY_ORDER, abilities, namespace, type Ability } from './agenttools'
  import { agentTypes, definition, type Definition } from './catalogs.svelte'
  import { t } from './i18n/index.svelte'

  let {
    project,
    agent,
    onpick,
    onread,
  }: { project: string; agent?: string; onpick: (name: string | undefined, found: Definition | null) => void; onread: (name: string) => void } = $props()

  let open = $state(false)
  let combo = $state<HTMLElement>()
  let query = $state('')
  let sources = $state<Record<string, Definition | null>>({})

  const types = $derived(agentTypes(project))

  $effect(() => {
    for (const type of types) {
      if (type.name in sources) continue
      definition(project, type.name).then((found) => (sources[type.name] = found))
    }
  })

  const SHORT: Record<Ability, string> = {
    read: 'persona.abilityRead',
    write: 'persona.abilityWrite',
    run: 'persona.abilityRun',
    web: 'persona.abilityWeb',
    delegate: 'persona.abilityDelegate',
  }

  function groupOf(name: string): [number, string] {
    const plugin = namespace(name).slice(0, -1)
    if (plugin) return [2, t('persona.groupPlugin', { plugin })]
    const found = sources[name]
    if (found?.source === 'project') return [0, t('persona.groupProject')]
    if (found?.source === 'user') return [1, t('persona.groupUser')]
    return [3, t('persona.groupBuiltin')]
  }

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const matching = types.filter((a) => !q || a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q))
    const grouped: [number, string, typeof matching][] = []
    for (const type of matching) {
      const [rank, group] = groupOf(type.name)
      const entry = grouped.find(([, g]) => g === group)
      if (entry) entry[2].push(type)
      else grouped.push([rank, group, [type]])
    }
    return grouped.sort((a, b) => a[0] - b[0] || a[1].localeCompare(b[1]))
  })

  const selected = $derived(agent ? types.find((a) => a.name === agent) : undefined)
  const can = $derived(agent ? abilities(sources[agent]?.tools ?? null) : null)

  function pick(name: string | undefined) {
    open = false
    query = ''
    onpick(name, name ? (sources[name] ?? null) : null)
  }
</script>

<svelte:window onpointerdown={(e) => open && !combo?.contains(e.target as Node) && (open = false)} />

<div class="option-row" bind:this={combo}>
  <button type="button" class="option-trigger" aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="option-icon" aria-hidden="true"
      ><svg width="14" height="14" viewBox="0 0 24 24"><circle cx="12" cy="8" r="4" /><path d="M4 21c1-4 4-6 8-6s7 2 8 6" /></svg></span
    >
    <span id="agent-type" class="option-key">{t('persona.field')}</span>
    <span class="option-value" class:persona={!!agent}>{agent ?? t('persona.noneShort')}</span>
    <svg class="option-updown" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 9l4-4 4 4M16 15l-4 4-4-4" /></svg>
  </button>
  {#if agent && (selected?.description || can)}
    <div class="about">
      {#if selected?.description}<p class="description">{selected.description}</p>{/if}
      <div class="line">
        {#if can}
          <div class="abilities">
            {#each ABILITY_ORDER as ability (ability)}<span class:yes={can[ability]}>{can[ability] ? '✓' : '×'} {t(SHORT[ability] as never)}</span>{/each}
          </div>
        {/if}
        <button type="button" class="read" onclick={() => onread(agent!)}>{t('persona.seeDefinition')}</button>
      </div>
    </div>
  {/if}
  {#if open}
    <div class="menu">
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field search" bind:value={query} placeholder={t('persona.search')} autofocus onkeydown={(e) => e.key === 'Escape' && (e.stopPropagation(), e.preventDefault(), (open = false))} />
      <div class="list" role="listbox" aria-labelledby="agent-type">
        {#if agent}
          <button type="button" role="option" aria-selected="false" onclick={() => pick(undefined)}>
            <span class="desc">{t('persona.none')}</span>
          </button>
        {/if}
        {#if !types.length}
          <p class="empty">{t('persona.unknown')}</p>
        {:else if !shown.length}
          <p class="empty">{t('persona.noMatch')}</p>
        {/if}
        {#each shown as [, group, list] (group)}
          <div class="group-title">{group}</div>
          {#each list as type (type.name)}
            <button type="button" role="option" aria-selected={type.name === agent} onclick={() => pick(type.name)}>
              <span class="name">{type.name}</span>
              {#if type.description}<span class="desc">{type.description}</span>{/if}
            </button>
          {/each}
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .persona {
    color: var(--persona);
    font-family: var(--mono);
    font-size: 12.5px;
  }

  .menu {
    position: absolute;
    z-index: 30;
    bottom: calc(100% + 2px);
    right: 0;
    left: 0;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.55);
  }

  .search {
    width: 100%;
    margin-bottom: 6px;
  }

  .list {
    display: flex;
    flex-direction: column;
    max-height: 280px;
    overflow-y: auto;
  }

  .group-title {
    margin: 8px 8px 3px;
    color: var(--text-faint);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .list button {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 5px 8px;
    border-radius: 4px;
    text-align: left;
  }

  .list button:hover,
  .list button[aria-selected='true'] {
    background: var(--persona-bg);
  }

  .name {
    color: var(--persona);
    font: 12.5px var(--mono);
  }

  .desc {
    overflow: hidden;
    color: var(--text-dim);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    margin: 8px;
    color: var(--text-faint);
    font-size: 12px;
  }

  .about {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 2px 10px 8px 34px;
  }

  .description {
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .abilities {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    color: var(--text-faint);
    font-size: 12px;
  }

  .abilities .yes {
    color: var(--persona);
  }

  .read {
    margin-left: auto;
    color: var(--persona);
    font-size: 12px;
  }
</style>
