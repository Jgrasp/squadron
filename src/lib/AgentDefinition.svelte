<script lang="ts">
  import { tick } from 'svelte'
  import { ABILITY_ORDER, abilities, groups, type Ability, type ToolGroup } from './agenttools'
  import { agents } from './agents.svelte'
  import { agentTypes, definition, type Definition } from './catalogs.svelte'
  import Dialog from './Dialog.svelte'
  import Icon from './Icon.svelte'
  import Markdown from './Markdown.svelte'
  import { t } from './i18n/index.svelte'

  let { project, name, live, onclose }: { project: string; name: string; live?: string[]; onclose: () => void } = $props()

  let found = $state<Definition | null | undefined>(undefined)
  let doc = $state<HTMLElement>()
  let headings = $state<{ id: string; text: string }[]>([])
  let active = $state('identity')

  $effect(() => {
    definition(project, name).then((d) => (found = d))
  })

  const type = $derived(agentTypes(project).find((a) => a.name === name))
  const description = $derived(found?.description || type?.description || '')
  const model = $derived(found?.model ?? type?.model ?? null)
  const tools = $derived(live ?? found?.tools ?? null)
  const using = $derived(agents.list.filter((a) => a.options.agent === name).length)

  const LONG: Record<Ability, string> = {
    read: 'persona.longRead',
    write: 'persona.longWrite',
    run: 'persona.longRun',
    web: 'persona.longWeb',
    delegate: 'persona.longDelegate',
  }

  const GROUP: Record<Ability, string> = {
    read: 'persona.groupRead',
    write: 'persona.groupWrite',
    run: 'persona.groupRun',
    web: 'persona.groupWeb',
    delegate: 'persona.groupDelegate',
  }

  function heading(group: ToolGroup) {
    if (group.kind === 'mcp') return t('persona.groupMcp', { server: group.server ?? '' })
    if (group.kind === 'other') return t('persona.groupOther')
    return t(GROUP[group.kind] as never)
  }

  const source = $derived(
    !found
      ? t('persona.sourceBuiltin')
      : found.source === 'plugin'
        ? t('persona.sourcePlugin', { plugin: found.plugin ?? '' })
        : found.source === 'user'
          ? t('persona.sourceUser')
          : t('persona.sourceProject'),
  )

  $effect(() => {
    if (!found?.body || !doc) return
    tick().then(() => {
      headings = [...doc!.querySelectorAll<HTMLElement>('.prompt h1, .prompt h2')].map((h, i) => {
        h.id = `prompt-${i}`
        return { id: h.id, text: h.textContent ?? '' }
      })
    })
  })

  function spy() {
    if (!doc) return
    const top = doc.getBoundingClientRect().top + 24
    let current = 'identity'
    for (const id of ['identity', 'tools', ...headings.map((h) => h.id)]) {
      const element = doc.querySelector(`#${id}`)
      if (element && element.getBoundingClientRect().top <= top) current = id
    }
    active = doc.scrollTop + doc.clientHeight >= doc.scrollHeight - 2 ? (headings.at(-1)?.id ?? current) : current
  }

  function go(id: string) {
    doc?.querySelector(`#${id}`)?.scrollIntoView({ block: 'start', behavior: 'smooth' })
  }
</script>

<Dialog title={name} width={900} {onclose}>
  <div class="head">
    <span class="hex"><Icon name="persona" size={18} /></span>
    <div class="id">
      <div class="source">{source}{#if found}<span class="path">{found.path}</span>{/if}</div>
    </div>
    {#if found}
      <button class="copy" type="button" onclick={() => navigator.clipboard.writeText(found!.path)}>{t('persona.copyPath')}</button>
    {/if}
  </div>
  <div class="body">
    <nav class="toc">
      <div class="toc-title">{t('persona.card')}</div>
      <button type="button" class:on={active === 'identity'} onclick={() => go('identity')}>{t('persona.identity')}</button>
      <button type="button" class:on={active === 'tools'} onclick={() => go('tools')}>{t('persona.tools')}</button>
      {#if headings.length}
        <div class="toc-title">{t('persona.prompt')}</div>
        {#each headings as h (h.id)}
          <button type="button" class:on={active === h.id} onclick={() => go(h.id)}>{h.text}</button>
        {/each}
      {/if}
    </nav>
    <div class="doc" bind:this={doc} onscroll={spy}>
      <section id="identity">
        {#if description}<p class="lead">{description}</p>{/if}
        <dl class="facts">
          <div><dt>{t('persona.model')}</dt><dd>{model ?? t('persona.none')}</dd></div>
          {#if found?.effort}<div><dt>{t('persona.effort')}</dt><dd>{found.effort}</dd></div>{/if}
          {#if found?.skills.length}
            <div><dt>{t('persona.skills')}</dt><dd>{#each found.skills as skill (skill)}<span class="tag">{skill}</span>{/each}</dd></div>
          {/if}
          <div><dt>{t('persona.usedBy')}</dt><dd>{t('persona.sessions', { count: using })}</dd></div>
        </dl>
      </section>
      <section id="tools">
        <h3>
          {t('persona.tools')}
          {#if tools}<span class="count">{tools.length}</span><span class="note">{live ? t('persona.toolsLive') : t('persona.toolsDeclared')}</span>{/if}
        </h3>
        <div class="abilities">
          {#each ABILITY_ORDER as ability (ability)}
            <span class="ability" class:yes={abilities(tools)[ability]}>{abilities(tools)[ability] ? '✓' : '×'} {t(LONG[ability] as never)}</span>
          {/each}
        </div>
        {#if tools}
          <div class="groups">
            {#each groups(tools) as group (group.kind + (group.server ?? ''))}
              <div class="group">
                <div class="group-title">{heading(group)}</div>
                <div class="list">{#each group.tools as tool (tool)}<span class="tool">{tool}</span>{/each}</div>
              </div>
            {/each}
          </div>
        {:else}
          <p class="muted">{t('persona.toolsAll')}</p>
        {/if}
      </section>
      {#if found?.body}
        <section class="prompt"><Markdown text={found.body} {project} /></section>
      {:else if found === null}
        <p class="muted">{t('persona.noFile')}</p>
      {/if}
    </div>
  </div>
</Dialog>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 22px 16px;
    border-bottom: 1px solid var(--border);
  }

  .hex {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 6px;
    background: var(--persona-bg);
    box-shadow: inset 0 0 0 1px var(--persona-line);
    color: var(--persona);
  }

  .id {
    flex: 1;
    min-width: 0;
  }

  .source {
    display: flex;
    gap: 8px;
    color: var(--text-dim);
    font-size: 12px;
  }

  .path {
    overflow: hidden;
    font-family: var(--mono);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy {
    padding: 4px 10px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: none;
    color: var(--text);
    font-size: 12px;
    cursor: pointer;
  }

  .body {
    display: grid;
    grid-template-columns: 180px 1fr;
    height: min(640px, calc(100vh - 180px));
  }

  .toc {
    display: flex;
    flex-direction: column;
    gap: 1px;
    overflow-y: auto;
    padding: 14px 10px;
    border-right: 1px solid var(--border);
  }

  .toc-title {
    margin: 10px 8px 4px;
    color: var(--text-faint);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .toc-title:first-child {
    margin-top: 0;
  }

  .toc button {
    overflow: hidden;
    padding: 4px 8px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: var(--text-dim);
    font-size: 12.5px;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }

  .toc button:hover {
    color: var(--text);
  }

  .toc button.on {
    background: var(--persona-bg);
    color: var(--persona);
  }

  .doc {
    overflow-y: auto;
    padding: 18px 26px 40px;
  }

  .lead {
    margin: 0 0 16px;
    font-size: 14.5px;
    line-height: 1.55;
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 10px 20px;
    margin: 0 0 22px;
  }

  dt {
    color: var(--text-faint);
    font-size: 11px;
  }

  dd {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 2px 0 0;
    font: 13px var(--mono);
  }

  .tag,
  .tool {
    padding: 1px 7px;
    border-radius: 3px;
    background: var(--surface-2);
    font: 12px var(--mono);
  }

  h3 {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0 0 10px;
    font-size: 14px;
  }

  .count {
    color: var(--persona);
    font: 12px var(--mono);
  }

  .note,
  .muted {
    color: var(--text-faint);
    font-size: 12px;
    font-weight: 400;
  }

  .abilities {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 14px;
  }

  .ability {
    padding: 3px 9px;
    border-radius: 3px;
    color: var(--text-faint);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 12px;
  }

  .ability.yes {
    background: var(--persona-bg);
    color: var(--persona);
    box-shadow: inset 0 0 0 1px var(--persona-line);
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 26px;
  }

  .group {
    display: grid;
    grid-template-columns: 130px 1fr;
    align-items: baseline;
  }

  .list {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }

  .group-title {
    color: var(--text-dim);
    font-size: 12px;
  }

  .prompt {
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
</style>
