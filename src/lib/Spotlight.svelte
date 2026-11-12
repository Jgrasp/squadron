<script lang="ts">
  import { perform, ui } from './actions.svelte'
  import { ask, message } from './asking.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { agents, find, ofProject, onDisk, selected, show, unarchive } from './agents.svelte'
  import { ago } from './format'
  import { errorText, locale, t } from './i18n/index.svelte'
  import { extensions, runItem, spotlightItems } from './extensions.svelte'
  import { needsAgent, text, type Item } from './extensions'
  import { showPulls } from './linked.svelte'
  import Icon from './Icon.svelte'
  import { projects, scratch, type Project } from './projects.svelte'
  import { approve, hostName, lists, lookUpLists, merge, moved, needsYou, openReview, requestChanges, reviewerOf, reviewWithAgent, standing, waitsFor, type Pull } from './pulls.svelte'
  import { hint, settings } from './settings.svelte'
  import {
    chains,
    chats,
    match,
    projectsFor,
    projectsNamed,
    reachedBy,
    PULL_ACTS,
    pullActs,
    pullsIn,
    pullsNamed,
    STARTERS,
    VERBS,
    verbsFor,
    verbsOf,
    waits,
    type Focus,
    type Listed,
    type PullAct,
    type Row,
  } from './spotlight'
  import type { Agent } from './stream'

  let { onclose }: { onclose: () => void } = $props()

  type Found = { id: string; name: string; project: string; since: number; text: string }
  type Entry = { agent: Pick<Agent, 'id' | 'name' | 'project' | 'since'>; archived: boolean; text: string; hay: string }
  type Hit = Entry & { excerpt: string }
  type Line =
    | { kind: 'row'; row: Row }
    | { kind: 'project'; project: Project }
    | { kind: 'chat' }
    | { kind: 'hit'; hit: Hit }
    | { kind: 'waiting' }
    | ({ kind: 'pull' } & Listed)
    | { kind: 'act'; act: PullAct }
    | { kind: 'item'; item: Item; agent?: Agent }
  type Group = { title: string; lines: Line[] }

  const textOf = (agent: Agent) =>
    agent.entries
      .map((e) => (e.kind === 'user' ? e.text : e.kind === 'agent' ? e.blocks.map((b) => (b.type === 'text' ? b.text : '')).join('\n') : ''))
      .join('\n')

  const entry = (agent: Entry['agent'], isArchived: boolean, text: string): Entry => ({ agent, archived: isArchived, text, hay: text.toLowerCase() })

  let pool = $state.raw<Entry[]>(agents.list.map((a) => entry(a, false, onDisk.has(a.id) ? '' : textOf(a))))
  invoke<Found[]>('archived_texts').then((list) => (pool = [...pool, ...list.map(({ text, ...agent }) => entry(agent, true, text))]))
  const shelved = agents.list.filter((a) => onDisk.has(a.id)).map((a) => a.id)
  if (shelved.length)
    invoke<Record<string, string>>('agent_texts', { ids: shelved }).then((texts) => {
      pool = pool.map((e) => (!e.archived && e.agent.id in texts ? entry(e.agent, false, texts[e.agent.id]) : e))
    })

  let query = $state('')
  let active = $state(0)
  let only = $state<string>()
  let shelf = $state(false)
  let focus = $state<Focus>()
  let before = $state<Focus>()
  let doomed = $state(false)
  let dialog: HTMLDialogElement
  let field: HTMLInputElement

  const projectOf = (path: string) => [...projects.list, ...projects.archived].find((p) => p.path === path)

  function snippet(text: string, at: number, length: number) {
    const from = Math.max(0, at - 50)
    const to = Math.min(text.length, at + length + 90)
    return (from ? '…' : '') + text.slice(from, to).replace(/\s+/g, ' ') + (to < text.length ? '…' : '')
  }

  const world = $derived({
    projects: projects.list,
    current: projects.list.find((p) => p.path === projects.current),
    agents: agents.list,
    shown: projects.overview || projects.board || projects.pulls ? undefined : selected(projects.current),
    priority: (p: Project) => needsYou(p.path),
  })
  const rows = $derived(focus ? [] : match(query, world))
  const rest = $derived(query.trim().split(/\s+/).slice(1).join(' '))
  const term = $derived(rows.length && rest ? rest : query.trim())
  const within = $derived(focus?.kind === 'project' ? focus.project.path : undefined)

  const matched = $derived.by(() => {
    if (focus && !within) return []
    const q = term.toLowerCase()
    const mine = within ? pool.filter((e) => e.agent.project === within) : pool
    if (!q) return mine.filter((e) => !e.archived).sort((a, b) => b.agent.since - a.agent.since).slice(0, 12).map((e) => ({ ...e, excerpt: '' }))
    const out: (Hit & { named: boolean })[] = []
    for (const e of mine) {
      const named = `${e.agent.name} ${projectOf(e.agent.project)?.name ?? ''}`.toLowerCase().includes(q)
      const at = q.length > 1 ? e.hay.indexOf(q) : -1
      if (named || at >= 0) out.push({ ...e, named, excerpt: at >= 0 ? snippet(e.text, at, q.length) : '' })
    }
    return out.sort((a, b) => Number(b.named) - Number(a.named) || Number(a.archived) - Number(b.archived) || b.agent.since - a.agent.since)
  })

  const facets = $derived.by(() => {
    if (focus || !query.trim()) return []
    const counts = new Map<string, number>()
    for (const hit of matched) if (!shelf || hit.archived) counts.set(hit.agent.project, (counts.get(hit.agent.project) ?? 0) + 1)
    if (only && !counts.has(only)) counts.set(only, 0)
    return [...counts]
      .flatMap(([path, count]) => {
        const project = projectOf(path)
        return project ? [{ project, count }] : []
      })
      .sort((a, b) => b.count - a.count)
  })

  const scoped = $derived(matched.filter((hit) => focus || !only || hit.agent.project === only))
  const shelfCount = $derived(scoped.filter((hit) => hit.archived).length)
  const shelfShown = $derived(!focus && (shelf || (shelfCount > 0 && shelfCount < scoped.length)))

  const hits = $derived(scoped.filter((hit) => focus || !shelf || hit.archived).slice(0, 50))

  const listed = $derived(
    projects.list.flatMap((project) => {
      const found = lists[project.path]
      return found ? [...found.review, ...found.mine, ...found.others].map((pull) => ({ project, pull })) : []
    }),
  )
  const waiting = $derived(
    projects.list.flatMap((project) => {
      const found = lists[project.path]
      return found ? [...found.review.filter((pull) => waitsFor(project.path, pull)), ...found.mine.filter((pull) => pull.changes_requested)].map((pull) => ({ project, pull })) : []
    }),
  )
  const modName = (item: Item) => {
    const manifest = extensions.list.find((m) => m.id === item.extension)?.manifest
    return manifest ? text(manifest.name, locale()) : item.extension
  }
  const label = (item: Item) => text(item.label, locale())
  const itemsFor = (agent: Agent | undefined): Line[] =>
    spotlightItems()
      .filter((item) => (!needsAgent(item) || agent) && reachedBy(query, [label(item), ...(item.words ?? []), modName(item)]))
      .map((item) => ({ kind: 'item', item, agent: needsAgent(item) ? agent : undefined }))

  const own = (project: Project, pull: Pull) => !!lists[project.path]?.mine.some((p) => p.number === pull.number)

  function actTitle(act: PullAct, project: Project, pull: Pull) {
    if (act === 'review') return t('review.review')
    if (act === 'reviewer') return t('review.openConversationOf', { name: reviewerOf(project.path, pull)?.name ?? '' })
    if (act === 'agent') return t('review.askAgent')
    if (act === 'host') return t('review.openOn', { host: hostName(pull) })
    if (act === 'approve') return t('review.verdictApprove')
    if (act === 'changes') return t('review.verdictRequestChanges')
    return t('review.merge')
  }

  function actsOf(project: Project, pull: Pull) {
    const mine = own(project, pull)
    const again = moved(project.path, pull)
    return pullActs(
      query,
      {
        reviewer: !!reviewerOf(project.path, pull),
        approve: !mine && (pull.reviewed !== 'approved' || again),
        changes: !mine && (pull.reviewed !== 'changes_requested' || again),
        merge: pull.mergeable,
      },
      (act) => actTitle(act, project, pull),
    )
  }

  function status(project: Project, pull: Pull): { tone: 'wait' | 'quiet'; text: string } {
    if (own(project, pull) && pull.changes_requested) return { tone: 'wait', text: t('review.changesRequested') }
    if (!own(project, pull) && waitsFor(project.path, pull)) return { tone: 'wait', text: pull.reviewed ? t('review.newChanges') : t('review.waitingForYou') }
    return { tone: 'quiet', text: standing(pull)[1] }
  }

  const groups = $derived.by((): Group[] => {
    const found = hits.map((hit): Line => ({ kind: 'hit', hit }))
    const row = (row: Row): Line => ({ kind: 'row', row })
    if (focus?.kind === 'project')
      return [
        { title: t('spotlight.actions'), lines: verbsOf(focus.project, query).map(row) },
        { title: t('spotlight.agentsIn', { name: focus.project.name }), lines: found },
      ]
    if (focus?.kind === 'verb')
      return [
        { title: t('spotlight.projects'), lines: projectsFor(focus.verb, query, world).map(row) },
        { title: t('spotlight.withoutProject'), lines: focus.verb.id === 'new' && (!query.trim() || chats(query)) ? [{ kind: 'chat' }] : [] },
      ]
    if (focus?.kind === 'agent') {
      const agent = focus.agent
      const mods = query.trim() ? itemsFor(agent).filter((line) => line.kind === 'item' && line.agent) : spotlightItems().filter(needsAgent).map((item): Line => ({ kind: 'item', item, agent }))
      return [{ title: agent.name, lines: [...verbsFor(agent, query, world).map(row), ...mods] }]
    }
    if (focus?.kind === 'waiting') return [{ title: t('spotlight.waiting'), lines: pullsIn(query, waiting).map((found): Line => ({ kind: 'pull', ...found })) }]
    if (focus?.kind === 'pull') {
      const { project, pull } = focus
      return [{ title: `#${pull.number} · ${project.name}`, lines: actsOf(project, pull).map((act): Line => ({ kind: 'act', act })) }]
    }
    const lead: Line[] = [
      ...(waiting.length && waits(query) ? [{ kind: 'waiting' } as const] : []),
      ...(chats(query) ? [{ kind: 'chat' } as const] : []),
      ...projectsNamed(query, world).map((project): Line => ({ kind: 'project', project })),
    ]
    const actions = [...lead, ...rows.map(row), ...itemsFor(world.shown)].slice(0, 8)
    const pulls = pullsNamed(query, listed).map((found): Line => ({ kind: 'pull', ...found }))
    return [
      { title: t('spotlight.waiting'), lines: !query.trim() && waiting.length ? [{ kind: 'waiting' }] : [] },
      { title: t('spotlight.actions'), lines: actions },
      { title: t('spotlight.pulls'), lines: pulls },
      { title: actions.length || pulls.length ? t('spotlight.agents') : query.trim() ? '' : t('spotlight.recent'), lines: found },
    ]
  })
  const lines = $derived(groups.flatMap((g) => g.lines))
  const starts = $derived(groups.map((_, i) => groups.slice(0, i).reduce((n, g) => n + g.lines.length, 0)))

  $effect(() => {
    if (query.trim()) return
    only = undefined
    shelf = false
  })

  function narrow(path?: string) {
    if (path === undefined) shelf = !shelf
    else only = only === path ? undefined : path
    active = 0
    field.focus()
  }

  $effect(() => {
    void query
    void focus
    active = 0
  })

  $effect(() => {
    dialog.showModal()
    field.focus()
  })

  function parts(text: string) {
    const q = term
    if (!q) return [{ text, hit: false }]
    return text.split(new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'i')).map((piece, i) => ({ text: piece, hit: i % 2 === 1 }))
  }

  function next(line: Line | undefined): Focus | undefined {
    if (!line) return
    if (line.kind === 'project') return { kind: 'project', project: line.project }
    if (line.kind === 'waiting') return { kind: 'waiting' }
    if (line.kind === 'pull') return { kind: 'pull', project: line.project, pull: line.pull }
    if (line.kind === 'row' && !focus && chains(line.row, query)) return { kind: 'verb', verb: line.row.verb }
    if (line.kind === 'hit' && !line.hit.archived && focus?.kind !== 'agent') {
      const agent = find(line.hit.agent.id)
      if (agent) return { kind: 'agent', agent }
    }
  }

  async function act(act: PullAct, project: Project, pull: Pull) {
    const dir = project.path
    try {
      if (act === 'review') return openReview(dir, pull.number)
      if (act === 'host') return await invoke('open_link', { url: pull.url })
      if (act === 'reviewer') {
        const agent = reviewerOf(dir, pull)
        return agent && show(agent)
      }
      if (act === 'agent') {
        if (!settings.pilot) {
          ui.settings = 'preferences'
          return
        }
        await reviewWithAgent(dir, pull)
        return showPulls(dir)
      }
      if (act === 'merge') {
        const sure = await ask(t('review.mergeAsk', { number: String(pull.number), title: pull.title, base: pull.base, host: hostName(pull) }), {
          title: t('review.mergeTitle'),
          kind: 'warning',
          okLabel: t('review.merge'),
          cancelLabel: t('common.cancel'),
        })
        if (!sure) return
        await merge(dir, pull)
      } else if (act === 'approve') await approve(dir, pull)
      else await requestChanges(dir, pull)
      await lookUpLists(dir, true)
    } catch (e) {
      message(`#${pull.number}: ${errorText(e)}`, { kind: 'error', title: project.name })
    }
  }

  async function choose(index: number) {
    const line = lines[index]
    if (!line) return
    if (line.kind === 'waiting') {
      before = focus
      focus = { kind: 'waiting' }
      query = ''
      return
    }
    onclose()
    if (line.kind === 'pull') return openReview(line.project.path, line.pull.number)
    if (line.kind === 'act') return focus?.kind === 'pull' && act(line.act, focus.project, focus.pull)
    if (line.kind === 'item') return runItem(line.item, { project: line.agent ? projectOf(line.agent.project) : world.current, agent: line.agent })
    if (line.kind === 'row') return perform(line.row)
    if (line.kind === 'project') return perform({ key: '', verb: VERBS.find((v) => v.id === 'open')!, title: '', project: line.project, complete: '' })
    if (line.kind === 'chat') {
      await scratch(t('board.scratchName'))
      ui.creating = true
      return
    }
    if (line.hit.archived) await unarchive(line.hit.agent.id)
    const agent = find(line.hit.agent.id)
    if (agent) show(agent)
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'Backspace' && focus && !query) {
      event.preventDefault()
      if (doomed) [focus, before] = [before, undefined]
      doomed = !doomed
      return
    }
    doomed = false
    const size = lines.length
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      active = (active + (event.key === 'ArrowDown' ? 1 : size - 1)) % Math.max(1, size)
      dialog.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: 'nearest' })
    } else if (event.key === 'Enter') {
      event.preventDefault()
      choose(active)
    } else if (event.key === 'Tab') {
      event.preventDefault()
      const line = lines[active]
      const chained = next(line)
      if (chained) {
        before = focus
        focus = chained
        query = ''
      } else if (line?.kind === 'row' && !focus) query = line.row.complete
    }
  }

  function start(word: string) {
    query = `${word} `
    field.focus()
  }

  function meta(row: Row): { tone: 'wait' | 'quiet' | 'dim'; text: string } | null {
    const path = row.project?.path
    if (!path) return null
    if (row.verb.id === 'archiveProject') return { tone: 'quiet', text: t('spotlight.agentCount', { count: ofProject(path).length }) }
    if (focus?.kind === 'verb' && row.verb.id !== 'pulls') return ofProject(path).length ? { tone: 'quiet', text: t('spotlight.agentCount', { count: ofProject(path).length }) } : null
    if (row.verb.id !== 'pulls') return null
    const found = lists[path]
    if (!found) return null
    if (found.error) return { tone: 'dim', text: t('spotlight.unavailable') }
    const waiting = needsYou(path)
    if (waiting) return { tone: 'wait', text: t('spotlight.toReview', { count: waiting }) }
    const open = found.review.length + found.mine.length
    return open ? { tone: 'quiet', text: t('spotlight.openPulls', { count: open }) } : { tone: 'dim', text: t('spotlight.noPull') }
  }

  function shortcut(row: Row) {
    const action = row.verb.action
    if (!action) return ''
    const here = row.agent ? row.agent.id === world.shown?.id : !row.project || row.project.path === projects.current
    return here ? hint(action) : ''
  }

  function named(focus: Focus) {
    if (focus.kind === 'project') return focus.project.name
    if (focus.kind === 'agent') return focus.agent.name
    if (focus.kind === 'pull') return `#${focus.pull.number}`
    if (focus.kind === 'waiting') return t('spotlight.toReviewChip')
    return focus.verb.title()
  }

  const placeholder = $derived(
    focus?.kind === 'project'
      ? t('spotlight.placeholderProject', { name: focus.project.name })
      : focus?.kind === 'verb'
        ? t('spotlight.placeholderVerb')
        : focus?.kind === 'agent'
          ? t('spotlight.placeholderAgent', { name: focus.agent.name })
          : focus?.kind === 'pull'
            ? t('spotlight.placeholderPull', { number: String(focus.pull.number) })
            : focus?.kind === 'waiting'
              ? t('spotlight.placeholderWaiting')
              : t('spotlight.placeholder'),
  )

  const help = $derived.by(() => {
    if (focus && doomed) return t('spotlight.hintRemove', { name: named(focus) })
    const line = lines[active]
    if (line?.kind === 'hit' && line.hit.archived && !focus) return t('shell.searchHint')
    const tab = next(line) ? t('spotlight.hintChain') : line?.kind === 'row' && !focus ? t('spotlight.hintComplete') : ''
    return [
      t('spotlight.hintMove'),
      line?.kind === 'hit' || line?.kind === 'project' || line?.kind === 'pull' || line?.kind === 'waiting' ? t('spotlight.hintOpen') : t('spotlight.hintRun'),
      tab,
      focus ? t('spotlight.hintBack', { name: named(focus) }) : '',
      t('spotlight.hintClose'),
    ]
      .filter(Boolean)
      .join(' · ')
  })
</script>

<dialog
  bind:this={dialog}
  class="spotlight"
  aria-label={t('shell.searchTitle')}
  oncancel={(e) => (e.preventDefault(), onclose())}
  onclick={(e) => e.target === dialog && onclose()}
>
  <div class="box">
    <div class="field">
      {#if focus?.kind === 'project'}
        <span class="chip tinted" class:doomed style:--project={focus.project.color}><i></i><span class="name">{focus.project.name}</span></span>
      {:else if focus?.kind === 'agent'}
        <span class="chip tinted agent" class:doomed style:--project={projectOf(focus.agent.project)?.color}><i></i><span class="name">{focus.agent.name}</span></span>
      {:else if focus?.kind === 'verb'}
        <span class="chip" class:doomed><Icon name={focus.verb.icon} /><span class="name">{focus.verb.title()}</span></span>
      {:else if focus?.kind === 'pull'}
        <span class="chip" class:doomed><Icon name="pull" /><span class="num">#{focus.pull.number}</span><span class="name">{focus.pull.title}</span></span>
      {:else if focus?.kind === 'waiting'}
        <span class="chip" class:doomed><Icon name="eye" /><span class="name">{t('spotlight.toReviewChip')}</span></span>
      {/if}
      <input bind:this={field} bind:value={query} class="query" {placeholder} spellcheck="false" onkeydown={keys} />
    </div>
    {#if !focus && !query.trim()}
      <div class="starters">
        <span>{t('spotlight.actions')}</span>
        {#each STARTERS as word (word)}<button type="button" onclick={() => start(word)}>{word}</button>{/each}
      </div>
    {:else if facets.length > 1 || only || shelfShown}
      <div class="filters">
        <span>{t('spotlight.filter')}</span>
        {#each facets as { project, count } (project.path)}
          <button type="button" class="facet" class:on={only === project.path} aria-pressed={only === project.path} style:--project={project.color} onclick={() => narrow(project.path)}>
            <i></i><span class="name">{project.name}</span><small>{count}</small>
          </button>
        {/each}
        {#if shelfShown}
          <button type="button" class="facet shelf" class:on={shelf} aria-pressed={shelf} onclick={() => narrow()}>
            <span class="name">{t('shell.searchArchivedTag')}</span><small>{shelfCount}</small>
          </button>
        {/if}
      </div>
    {/if}
    <ul role="listbox" aria-label={t('shell.searchTitle')}>
      {#each groups as group, g (g)}
        {#if group.lines.length && group.title}<li class="section" role="presentation">{group.title}</li>{/if}
        {#each group.lines as line, j (j)}
          {@const i = starts[g] + j}
          {@const chained = i === active && !!next(line)}
          <li role="option" aria-selected={i === active} data-index={i}>
            {#if line.kind === 'hit'}
              {@const hit = line.hit}
              {@const project = projectOf(hit.agent.project)}
              <button class="hit" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="line" style:--project={project?.color}>
                  <span class="square" aria-hidden="true"></span>
                  <strong>{#each parts(hit.agent.name) as part, p (p)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}</strong>
                  {#if !focus}<span class="where">{project?.name ?? ''}</span>{/if}
                  {#if hit.archived}<span class="tag">{t('shell.searchArchivedTag')}</span>{/if}
                  <span class="when">{ago(hit.agent.since, Date.now())}</span>
                  {#if chained}<kbd>Tab</kbd>{/if}
                </span>
                {#if hit.excerpt}
                  <span class="excerpt">{#each parts(hit.excerpt) as part, p (p)}{#if part.hit}<mark>{part.text}</mark>{:else}{part.text}{/if}{/each}</span>
                {/if}
              </button>
            {:else if line.kind === 'project'}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><i class="swatch" style:--project={line.project.color}></i></span>
                <span class="title">{line.project.name}</span>
                <span class="kind">{line.project.path === projects.current ? t('spotlight.currentProject') : t('spotlight.project')}</span>
                <span class="grow"></span>
                {#if ofProject(line.project.path).length}<span class="note quiet">{t('spotlight.agentCount', { count: ofProject(line.project.path).length })}</span>{/if}
                {@render kbds(i, chained, '')}
              </button>
            {:else if line.kind === 'waiting'}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><Icon name="eye" /></span>
                <span class="title">{t('spotlight.waitingRow')}</span>
                <span class="kind">{t('spotlight.allProjects')}</span>
                <span class="grow"></span>
                <span class="note wait">{t('spotlight.toReview', { count: waiting.length })}</span>
                {@render kbds(i, chained, '')}
              </button>
            {:else if line.kind === 'pull'}
              {@const note = status(line.project, line.pull)}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><Icon name="pull" /></span>
                <span class="num">#{line.pull.number}</span>
                <span class="title">{line.pull.title}</span>
                <span class="target" style:--project={line.project.color}><i></i>{line.project.name}</span>
                <span class="grow"></span>
                <span class="kind">{line.pull.author}</span>
                <span class="note {note.tone}">{note.text}</span>
                {@render kbds(i, chained, '')}
              </button>
            {:else if line.kind === 'act'}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><Icon name={PULL_ACTS[line.act].icon} /></span>
                {#if focus?.kind === 'pull'}<span class="title">{actTitle(line.act, focus.project, focus.pull)}</span>{/if}
                <span class="grow"></span>
                {@render kbds(i, false, '')}
              </button>
            {:else if line.kind === 'item'}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><Icon name={line.item.icon ?? 'plugins'} /></span>
                <span class="title">{label(line.item)}</span>
                {#if line.agent && focus?.kind !== 'agent'}
                  <span class="target agent" style:--project={projectOf(line.agent.project)?.color}><i></i>{line.agent.name}</span>
                {/if}
                <span class="tag">{modName(line.item)}</span>
                <span class="grow"></span>
                {@render kbds(i, false, '')}
              </button>
            {:else if line.kind === 'chat'}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                <span class="icon"><Icon name="plus" /></span>
                <span class="title">{t('spotlight.newChat')}</span>
                <span class="kind">{t('spotlight.newChatDetail')}</span>
                <span class="grow"></span>
                {@render kbds(i, false, '')}
              </button>
            {:else}
              {@const row = line.row}
              {@const note = meta(row)}
              <button class="action" onclick={() => choose(i)} onmousemove={() => (active = i)}>
                {#if focus?.kind === 'verb' && row.project}
                  <span class="icon"><i class="swatch" style:--project={row.project.color}></i></span>
                  <span class="title">{row.project.name}</span>
                  {#if row.project.path === projects.current}<span class="kind">{t('spotlight.currentProject')}</span>{/if}
                {:else}
                  <span class="icon"><Icon name={row.verb.icon} /></span>
                  <span class="title">{row.title}</span>
                  {#if !focus && row.agent}
                    <span class="target agent" style:--project={row.project?.color}><i></i>{row.agent.name}</span>
                  {:else if !focus && row.project}
                    <span class="target" style:--project={row.project.color}><i></i>{row.project.name}</span>
                  {/if}
                {/if}
                <span class="grow"></span>
                {#if note}<span class="note {note.tone}">{note.text}</span>{/if}
                {@render kbds(i, chained, shortcut(row))}
              </button>
            {/if}
          </li>
        {/each}
      {/each}
      {#if !lines.length}<li class="none">{t('shell.searchNoResult')}</li>{/if}
    </ul>
    <div class="hint">{help}</div>
  </div>
</dialog>

{#snippet kbds(i: number, chained: boolean, key: string)}
  {#if i === active}{#if chained}<kbd>Tab</kbd>{/if}<kbd class="enter">↵</kbd>{:else if key}<kbd>{key}</kbd>{/if}
{/snippet}

<style>
  .spotlight {
    width: 680px;
    max-width: calc(100vw - 48px);
    margin: 14vh auto auto;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: var(--bg-main);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
  }

  .spotlight::backdrop {
    background: rgba(0, 0, 0, 0.45);
  }

  .box {
    display: flex;
    flex-direction: column;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 52px;
    padding: 0 18px;
    border-bottom: 1px solid var(--border);
  }

  .query {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-strong);
    font-size: 16px;
    outline: none;
  }

  .chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 260px;
    height: 24px;
    padding: 0 9px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-strong);
    white-space: nowrap;
  }

  .chip :global(.icon) {
    color: var(--text-muted);
  }

  .chip .num,
  .num {
    flex: none;
    font: 500 12px var(--mono);
    color: var(--text-dim);
  }

  .chip .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip.tinted {
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 60%, transparent);
  }

  .chip i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .chip.agent i {
    border-radius: 50%;
  }

  .chip.doomed {
    background: var(--surface-5);
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  ul {
    max-height: 58vh;
    overflow-y: auto;
    margin: 0;
    padding: 6px;
    list-style: none;
  }

  li button {
    width: 100%;
    padding: 9px 12px;
    border-radius: 6px;
    text-align: left;
  }

  .hit {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .action {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
  }

  .starters {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-dim);
  }

  .starters span {
    margin-right: 4px;
  }

  .starters button {
    height: 22px;
    padding: 0 8px;
    border-radius: 3px;
    background: var(--surface-3);
    font: 500 12px var(--mono);
    color: var(--text-2);
  }

  .filters {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-dim);
  }

  .filters > span {
    margin-right: 4px;
  }

  .facet {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 220px;
    height: 22px;
    padding: 0 8px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-2);
  }

  .facet:hover {
    background: var(--surface-4);
  }

  .facet i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .facet .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .facet small {
    flex: none;
    font: 500 11px var(--mono);
    color: var(--text-dim);
  }

  .facet.shelf {
    --project: var(--text-muted);
  }

  .facet.on {
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 60%, transparent);
    font-weight: 600;
    color: var(--text-strong);
  }

  .section {
    padding: 8px 12px 3px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-muted);
  }

  li[aria-selected='true'] .icon {
    background: rgba(224, 138, 91, 0.16);
    box-shadow: inset 0 0 0 1px rgba(224, 138, 91, 0.4);
    color: var(--accent-soft);
  }

  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .target {
    flex: none;
    max-width: 240px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 60%, transparent);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .target i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .target.agent i {
    border-radius: 50%;
  }

  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    background: var(--project);
  }

  .kind {
    flex: none;
    font-size: 12px;
    color: var(--text-dim);
  }

  .grow {
    flex: 1;
  }

  .note {
    flex: none;
    font-size: 12px;
    color: var(--text-dim);
  }

  .note.wait,
  .note.quiet {
    padding: 2px 8px;
    border-radius: 3px;
    font-size: 11.5px;
  }

  .note.wait {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-border);
    font-weight: 700;
    color: var(--accent-soft);
  }

  .note.quiet {
    background: var(--surface-3);
    color: var(--text-muted);
  }

  kbd {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 500 11px var(--mono);
    color: var(--text-dim);
  }

  kbd.enter {
    background: var(--accent);
    box-shadow: none;
    color: var(--accent-ink);
  }

  li[aria-selected='true'] button {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .square {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--project, var(--text-faint));
  }

  strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .where,
  .when {
    flex: none;
    font-size: 12px;
    color: var(--text-dim);
  }

  .when {
    margin-left: auto;
  }

  .tag {
    flex: none;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 11px;
    color: var(--text-muted);
  }

  .excerpt {
    padding-left: 17px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-muted);
  }

  mark {
    border-radius: 2px;
    background: rgba(224, 138, 91, 0.35);
    color: var(--text-strong);
  }

  .none {
    padding: 14px 12px;
    color: var(--text-dim);
  }

  .hint {
    padding: 8px 18px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--text-dim);
  }
</style>
