<script lang="ts">
  import { tick, type Component } from 'svelte'
  import type { SettingsTab } from './actions.svelte'
  import AgentSettings from './AgentSettings.svelte'
  import ClaudeSettings from './ClaudeSettings.svelte'
  import CodeHosting from './CodeHosting.svelte'
  import LayoutSettings from './LayoutSettings.svelte'
  import McpServers from './McpServers.svelte'
  import NotificationSettings from './NotificationSettings.svelte'
  import PreferenceSettings from './PreferenceSettings.svelte'
  import ProjectAgents from './ProjectAgents.svelte'
  import ProjectPage from './ProjectPage.svelte'
  import ProjectReview from './ProjectReview.svelte'
  import ProjectRuns from './ProjectRuns.svelte'
  import ProjectsRules from './ProjectsRules.svelte'
  import ShortcutSettings from './ShortcutSettings.svelte'
  import Icon from './Icon.svelte'
  import { ordered } from './projects.svelte'
  import { errorText, t } from './i18n/index.svelte'

  export type Page = { id: SettingsTab; group: string; name: string }

  let {
    query,
    pages,
    claude = $bindable(),
    counts = $bindable({}),
    onopen,
  }: { query: string; pages: Page[]; claude: string; counts?: Record<string, number>; onopen: (tab: SettingsTab, row?: string, project?: string) => void } = $props()

  const PAGES: Partial<Record<SettingsTab, Component>> = {
    agents: AgentSettings,
    mcp: McpServers,
    hosting: CodeHosting,
    preferences: PreferenceSettings,
    notifications: NotificationSettings,
    shortcuts: ShortcutSettings,
    layout: LayoutSettings,
  }
  const shown = $derived(pages.filter((p) => p.id in PAGES || p.id === 'claude' || p.id === 'projects'))
  const first = $derived(shown.find((p) => counts[p.id])?.id)

  let root = $state<HTMLElement>()
  let error = $state('')

  async function act(work: () => Promise<unknown>) {
    error = ''
    try {
      await work()
    } catch (e) {
      error = errorText(e)
    }
  }

  const fold = (text: string) =>
    text
      .split('')
      .map((c) => c.normalize('NFD')[0].toLowerCase()[0])
      .join('')

  function headingOf(option: Element) {
    let at = option.closest('.options')?.previousElementSibling
    while (at && !at.matches('.options')) {
      if (at.matches('h3, h4')) return at
      at = at.previousElementSibling
    }
  }

  const said = (el: Element) => [el.textContent, ...[el, ...el.querySelectorAll('[title], [aria-label]')].flatMap((e) => [e.getAttribute('title'), e.getAttribute('aria-label')])].join(' ')

  let kept = new Set<Element>()

  function filter() {
    if (!root) return
    const words = fold(query).split(/\s+/).filter(Boolean)
    const next: Record<string, number> = {}
    const scopes: Element[] = []
    const found: Element[] = []
    for (const result of root.querySelectorAll<HTMLElement>('.hit-page')) {
      const page = result.querySelector(':scope > .page')!
      for (const el of page.querySelectorAll('[data-out], [data-ctx], [data-first]')) ['data-out', 'data-ctx', 'data-first'].forEach((a) => el.removeAttribute(a))
      const where = fold(`${result.dataset.group} ${result.dataset.name}`)
      const hits = [...page.querySelectorAll('.option')].filter((o) => kept.has(o) || words.every((w) => fold(`${where} ${headingOf(o)?.textContent ?? ''} ${said(o)}`).includes(w)))
      next[result.dataset.tab!] = hits.length
      found.push(...hits)
      const leaves = new Set<Element>(hits)
      for (const o of hits) {
        const heading = headingOf(o)
        if (heading) leaves.add(heading)
        if (!o.matches('.sub')) {
          for (let sub = o.nextElementSibling; sub?.matches('.option.sub'); sub = sub.nextElementSibling) leaves.add(sub)
          continue
        }
        let parent = o.previousElementSibling
        while (parent?.matches('.sub')) parent = parent.previousElementSibling
        if (parent?.matches('.option') && !leaves.has(parent)) leaves.add(parent), parent.setAttribute('data-ctx', '')
      }
      const keep = new Set<Element>()
      for (const leaf of leaves) for (let at: Element | null = leaf; at && at !== page; at = at.parentElement) keep.add(at)
      const hide = (el: Element) => {
        for (const child of el.children) {
          if (!keep.has(child)) child.setAttribute('data-out', '')
          else if (!leaves.has(child)) hide(child)
        }
      }
      hide(page)
      for (const list of page.querySelectorAll('.options')) list.querySelector(':scope > .option:not([data-out])')?.setAttribute('data-first', '')
      scopes.push(result.querySelector(':scope > .crumb .path')!, ...[...leaves].filter((l) => !l.hasAttribute('data-ctx')))
    }
    kept = new Set(found)
    counts = next
    highlight(words, scopes)
  }

  function highlight(words: string[], scopes: Element[]) {
    if (!('highlights' in CSS)) return
    const ranges: Range[] = []
    for (const scope of scopes) {
      const walk = document.createTreeWalker(scope, NodeFilter.SHOW_TEXT)
      for (let node = walk.nextNode(); node; node = walk.nextNode()) {
        const text = fold(node.nodeValue ?? '')
        for (const w of words)
          for (let i = text.indexOf(w); i >= 0; i = text.indexOf(w, i + w.length)) {
            const range = new Range()
            range.setStart(node, i)
            range.setEnd(node, i + w.length)
            ranges.push(range)
          }
      }
    }
    CSS.highlights.set('settings-search', new Highlight(...ranges))
  }

  $effect(() => {
    void query
    kept = new Set()
    tick().then(filter)
  })

  $effect(() => {
    if (!root) return
    let frame = 0
    const watch = new MutationObserver(() => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(filter)
    })
    watch.observe(root, { childList: true, subtree: true })
    return () => {
      watch.disconnect()
      cancelAnimationFrame(frame)
      if ('highlights' in CSS) CSS.highlights.delete('settings-search')
    }
  })

  function open(id: SettingsTab) {
    const row = root?.querySelector(`.hit-page[data-tab="${id}"] .option:not([data-out]):not([data-ctx]) strong`)
    onopen(id, row?.textContent ?? undefined)
  }
</script>

<div class="results" bind:this={root}>
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
  {#each shown as p (p.id)}
    {@const Page = PAGES[p.id]}
    <section class="hit-page" class:first={p.id === first} style:display={counts[p.id] ? null : 'none'} data-tab={p.id} data-group={p.group} data-name={p.name}>
      <div class="crumb">
        <div class="path"><span>{p.group}</span><span class="sep">›</span>{p.name}</div>
        <button class="btn btn-ghost" onclick={() => open(p.id)}>{t('settings.searchOpen', { page: p.name })}<Icon name="jump" size={13} /></button>
      </div>
      <div class="page">
        {#if p.id === 'projects'}
          <ProjectPage>
            <ProjectAgents {act} pick={(path) => onopen('projects', undefined, path)} />
            {#if ordered().length}<ProjectsRules {act} />{/if}
            <ProjectRuns {act} />
            <ProjectReview {act} />
          </ProjectPage>
        {:else if p.id === 'claude'}
          <ClaudeSettings bind:path={claude} />
        {:else if Page}
          <Page />
        {/if}
      </div>
    </section>
  {/each}
  {#if !first}
    <div class="none">
      <b>{t('settings.searchNone', { query: query.trim() })}</b>
      {t('settings.searchNoneHint')}
    </div>
  {/if}
</div>

<style>
  .results {
    display: flex;
    flex-direction: column;
  }

  .hit-page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-top: 22px;
    border-top: 1px solid var(--border);
    margin-top: 22px;
    scroll-margin-top: 8px;
  }

  .hit-page.first {
    padding-top: 0;
    border-top: none;
    margin-top: 0;
  }

  .crumb {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .path {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .path span {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
  }

  .path .sep {
    color: var(--text-faint);
  }

  .crumb .btn-ghost {
    height: 30px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .page :global([data-out]) {
    display: none !important;
  }

  .page :global([data-first]) {
    border-top: none !important;
  }

  .page :global([data-ctx]) {
    opacity: 0.55;
  }

  .none {
    padding: 18px 20px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 12.5px;
  }

  .none b {
    display: block;
    margin-bottom: 4px;
    color: var(--text);
    font-size: 13.5px;
    font-weight: 600;
  }

  .bad {
    margin: 0 0 16px;
    color: var(--err-text);
  }

  :global(::highlight(settings-search)) {
    background-color: color-mix(in srgb, var(--accent-link) 30%, transparent);
    color: inherit;
  }
</style>
