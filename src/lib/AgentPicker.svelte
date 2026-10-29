<script lang="ts">
  import { ABILITY_ORDER, abilities, namespace, type Ability } from './agenttools'
  import { agentTypes, definition, type Definition } from './catalogs.svelte'
  import { t } from './i18n/index.svelte'
  import PersonaChip from './PersonaChip.svelte'

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

<div class="group">
  <span id="agent-type" class="label">{t('persona.field')}</span>
  {#if agent}
    <div class="selected">
      <div class="line">
        <PersonaChip name={agent} />
        <button type="button" class="remove" aria-label={t('persona.remove')} title={t('persona.remove')} onclick={() => pick(undefined)}>×</button>
        <button type="button" class="read" onclick={() => onread(agent!)}>{t('persona.seeDefinition')}</button>
      </div>
      {#if selected?.description}<p class="description">{selected.description}</p>{/if}
      {#if can}
        <div class="abilities">
          {#each ABILITY_ORDER as ability (ability)}<span class:yes={can[ability]}>{can[ability] ? '✓' : '×'} {t(SHORT[ability] as never)}</span>{/each}
        </div>
      {/if}
    </div>
  {:else}
    <div class="combo" bind:this={combo}>
      <button type="button" class="field trigger" aria-labelledby="agent-type" aria-expanded={open} onclick={() => (open = !open)}>{t('persona.none')}</button>
      {#if open}
        <div class="menu">
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field search" bind:value={query} placeholder={t('persona.search')} autofocus onkeydown={(e) => e.key === 'Escape' && (e.stopPropagation(), e.preventDefault(), (open = false))} />
          <div class="list" role="listbox" aria-labelledby="agent-type">
            {#if !types.length}
              <p class="empty">{t('persona.unknown')}</p>
            {:else if !shown.length}
              <p class="empty">{t('persona.noMatch')}</p>
            {/if}
            {#each shown as [, group, list] (group)}
              <div class="group-title">{group}</div>
              {#each list as type (type.name)}
                <button type="button" role="option" aria-selected="false" onclick={() => pick(type.name)}>
                  <span class="name">{type.name}</span>
                  {#if type.description}<span class="desc">{type.description}</span>{/if}
                </button>
              {/each}
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .label {
    font-weight: 600;
  }

  .combo {
    position: relative;
  }

  .trigger {
    width: 100%;
    color: var(--text-dim);
    text-align: left;
    cursor: pointer;
  }

  .menu {
    position: absolute;
    z-index: 5;
    top: calc(100% + 4px);
    right: 0;
    left: 0;
    padding: 6px;
    border-radius: 6px;
    background: var(--bg-main);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 16px 40px rgba(0, 0, 0, 0.5);
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
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .list button:hover {
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

  .selected {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .line {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .remove,
  .read {
    border: 0;
    background: none;
    color: var(--text-dim);
    cursor: pointer;
  }

  .remove {
    font-size: 16px;
    line-height: 1;
  }

  .read {
    margin-left: auto;
    color: var(--persona);
    font-size: 12px;
  }

  .description {
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
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
</style>
