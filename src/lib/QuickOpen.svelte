<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import type { Goto } from './CodeEditor.svelte'
  import type { Change } from './FileTree.svelte'
  import { t } from './i18n/index.svelte'
  import { rank } from './mention.svelte'

  type Hit = { path: string; line: number; text: string }
  type Row = { path: string; line?: number; text?: string }
  type Mode = 'files' | 'text'

  let {
    project,
    recent,
    status,
    authors,
    onpick,
    onclose,
  }: {
    project: string
    recent: string[]
    status: Map<string, Change>
    authors: Map<string, string[]>
    onpick: (path: string, goto: Goto | null) => void
    onclose: () => void
  } = $props()

  const MODE = 'squadron.quickOpen'
  const LIMIT = 500

  let mode = $state<Mode>(localStorage.getItem(MODE) === 'text' ? 'text' : 'files')
  let query = $state('')
  let flags = $state({ caseSensitive: false, wholeWord: false, regexp: false })
  let at = $state(0)
  let files = $state<string[]>([])
  let hits = $state<Hit[]>([])
  let searching = $state(false)
  let failed = $state('')
  let texts = $state<Record<string, string | null>>({})
  let input: HTMLInputElement
  let list = $state<HTMLUListElement>()

  invoke<string[]>('project_files', { dir: project })
    .then((all) => (files = all.filter((p) => !p.endsWith('/'))))
    .catch(() => {})

  $effect(() => input.focus())

  const wanted = $derived.by(() => {
    const found = /^(.*?):(\d+)$/.exec(query.trim())
    return found ? { text: found[1].toLowerCase(), line: Number(found[2]) } : { text: query.trim().toLowerCase(), line: 0 }
  })

  const rows = $derived.by((): Row[] => {
    if (mode === 'text') return hits
    if (!wanted.text) return [...new Set([...recent, ...status.keys()])].filter((p) => status.get(p) !== 'deleted').slice(0, 50).map((path) => ({ path }))
    return files
      .map((path) => ({ path, score: rank(path, wanted.text) }))
      .filter((f) => f.score >= 0)
      .sort((a, b) => a.score - b.score || a.path.length - b.path.length || a.path.localeCompare(b.path))
      .slice(0, 50)
      .map(({ path }) => ({ path, line: wanted.line || undefined }))
  })

  const selected = $derived(rows[Math.min(at, rows.length - 1)])

  $effect(() => {
    if (mode !== 'text') return
    const text = query
    const options = { ...flags }
    if (!text) return void ((hits = []), (failed = ''), (searching = false))
    searching = true
    const timer = setTimeout(() => {
      invoke<Hit[]>('project_grep', { project, query: text, case: options.caseSensitive, word: options.wholeWord, regex: options.regexp })
        .then((found) => {
          if (text !== query) return
          hits = found
          failed = ''
          at = 0
        })
        .catch((e) => text === query && ((hits = []), (failed = String(e))))
        .finally(() => text === query && (searching = false))
    }, 200)
    return () => clearTimeout(timer)
  })

  $effect(() => {
    const path = mode === 'text' ? selected?.path : undefined
    if (!path || path in texts) return
    invoke<string | null>('read_project_file', { project, path })
      .then((text) => (texts[path] = text))
      .catch(() => (texts[path] = null))
  })

  $effect(() => {
    void at
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' })
  })

  const pattern = $derived.by(() => {
    if (!query) return null
    const source = flags.regexp ? query : query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    try {
      return new RegExp(flags.wholeWord ? `\\b(?:${source})\\b` : source, flags.caseSensitive ? '' : 'i')
    } catch {
      return null
    }
  })

  function split(text: string) {
    const found = pattern?.exec(text)
    if (!found || !found[0]) return { before: text, match: '', after: '' }
    return { before: text.slice(0, found.index), match: found[0], after: text.slice(found.index + found[0].length) }
  }

  function snippet(text: string) {
    const parts = split(text.trimStart())
    return parts.before.length > 40 ? { ...parts, before: '…' + parts.before.slice(-30) } : parts
  }

  function marks(path: string) {
    const lower = path.toLowerCase()
    const on = new Array<boolean>(path.length).fill(false)
    const q = wanted.text
    if (!q) return on
    const name = lower.lastIndexOf('/') + 1
    const direct = lower.indexOf(q, name) >= 0 ? lower.indexOf(q, name) : lower.indexOf(q)
    if (direct >= 0) on.fill(true, direct, direct + q.length)
    else for (let i = lower.length - 1, j = q.length - 1; i >= 0 && j >= 0; i--) if (lower[i] === q[j]) (on[i] = true), j--
    return on
  }

  function pieces(path: string, from: number, to: number) {
    const on = marks(path)
    const out: { text: string; on: boolean }[] = []
    for (let i = from; i < to; i++) {
      if (out.at(-1)?.on === on[i]) out.at(-1)!.text += path[i]
      else out.push({ text: path[i], on: on[i] })
    }
    return out
  }

  const nameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)
  const dirOf = (path: string) => path.slice(0, Math.max(path.lastIndexOf('/'), 0))

  const context = $derived.by(() => {
    const text = selected?.line && texts[selected.path]
    if (!text) return []
    const lines = text.split('\n')
    const first = Math.max(selected.line! - 4, 0)
    return lines.slice(first, selected.line! + 3).map((line, i) => ({ n: first + i + 1, line }))
  })

  const fileCount = $derived(new Set(hits.map((h) => h.path)).size)

  function pick(row: Row | undefined) {
    if (!row) return
    onpick(row.path, row.line ? { line: row.line, query: mode === 'text' ? { search: query, ...flags } : undefined } : null)
  }

  function switchMode() {
    mode = mode === 'files' ? 'text' : 'files'
    at = 0
    localStorage.setItem(MODE, mode)
    input.focus()
  }

  function keys(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose()
    else if (event.key === 'Tab') switchMode()
    else if (event.key === 'ArrowDown') at = Math.min(at + 1, rows.length - 1)
    else if (event.key === 'ArrowUp') at = Math.max(at - 1, 0)
    else if (event.key === 'Enter') pick(selected)
    else return
    event.preventDefault()
    event.stopPropagation()
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="veil" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="palette" class:text={mode === 'text'} role="dialog" aria-label={t('shell.quickOpen')} tabindex="-1" onkeydown={keys}>
    <div class="query">
      <span class="glass" aria-hidden="true">⌕</span>
      <input
        bind:this={input}
        bind:value={() => query, (v) => ((query = v), (at = 0))}
        placeholder={mode === 'files' ? t('shell.quickFilesPlaceholder') : t('shell.quickTextPlaceholder')}
        spellcheck="false"
        aria-label={mode === 'files' ? t('shell.quickFilesPlaceholder') : t('shell.quickTextPlaceholder')}
      />
      {#if mode === 'text'}
        <button class="toggle" aria-pressed={flags.caseSensitive} title={t('shell.cmMatchCase')} onclick={() => (flags.caseSensitive = !flags.caseSensitive)}>Aa</button>
        <button class="toggle" aria-pressed={flags.wholeWord} title={t('shell.cmByWord')} onclick={() => (flags.wholeWord = !flags.wholeWord)}>ab</button>
        <button class="toggle" aria-pressed={flags.regexp} title={t('shell.cmRegexp')} onclick={() => (flags.regexp = !flags.regexp)}>.*</button>
      {/if}
      <div class="modes" role="tablist">
        <button role="tab" aria-selected={mode === 'files'} onclick={() => mode !== 'files' && switchMode()}>{t('shell.quickFiles')}</button>
        <button role="tab" aria-selected={mode === 'text'} onclick={() => mode !== 'text' && switchMode()}>{t('shell.quickText')}</button>
      </div>
    </div>

    <ul bind:this={list} role="listbox">
      {#each rows as row, i (row.path + ':' + (row.line ?? ''))}
        {@const change = status.get(row.path)}
        <li role="option" aria-selected={row === selected} class:hit={mode === 'text'}>
          <button onclick={() => pick(row)} onmousemove={() => (at = i)}>
            <span class="square {change ?? ''}"></span>
            {#if mode === 'files'}
              <span class="name">{#each pieces(row.path, row.path.length - nameOf(row.path).length, row.path.length) as p}<span class:on={p.on}>{p.text}</span>{/each}</span>
              <span class="dir">{#each pieces(row.path, 0, dirOf(row.path).length) as p}<span class:on={p.on}>{p.text}</span>{/each}</span>
              {#if authors.get(row.path)?.length}<span class="who">{authors.get(row.path)!.join(', ')}</span>{/if}
            {:else}
              {@const parts = snippet(row.text ?? '')}
              <span class="name">{nameOf(row.path)}</span>
              <span class="line">{row.line}</span>
              <code>{parts.before}<mark>{parts.match}</mark>{parts.after}</code>
            {/if}
          </button>
        </li>
      {:else}
        <li class="none">
          {#if mode === 'files'}{wanted.text ? t('shell.quickNoFile') : ''}{:else if failed}{failed}{:else if searching}{t('shell.quickSearching')}{:else if query}{t('shell.quickNoHit')}{/if}
        </li>
      {/each}
    </ul>

    {#if mode === 'text' && selected}
      <div class="where">
        <span class="square {status.get(selected.path) ?? ''}"></span>
        <span>{selected.path}</span>
        {#if authors.get(selected.path)?.length}<span class="who">{authors.get(selected.path)!.join(', ')}</span>{/if}
      </div>
      <div class="preview">
        {#each context as { n, line } (n)}
          {@const parts = n === selected.line ? split(line) : null}
          <div class="row" class:current={!!parts}>
            <i>{n}</i>
            <span>{#if parts}{parts.before}<mark>{parts.match}</mark>{parts.after}{:else}{line}{/if}</span>
          </div>
        {/each}
      </div>
    {/if}

    <div class="foot">
      {#if mode === 'text' && hits.length}
        <span>{hits.length >= LIMIT ? t('shell.quickMore', { count: LIMIT }) : `${t('shell.quickHits', { count: hits.length })} ${t('shell.quickInFiles', { count: fileCount })}`}</span>
      {/if}
      <span><kbd>↑</kbd><kbd>↓</kbd> <kbd>↵</kbd> {mode === 'text' ? t('shell.quickPickLine') : t('shell.quickPick')}</span>
      <span><kbd>Tab</kbd> {t('shell.quickMode')}</span>
    </div>
  </div>
</div>

<style>
  .veil {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 40px;
    background: var(--overlay);
  }

  .palette {
    width: min(620px, calc(100% - 32px));
    max-height: calc(100% - 64px);
    display: flex;
    flex-direction: column;
    border-radius: 9px;
    border: 1px solid var(--border-strong);
    background: var(--menu);
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.55);
    overflow: hidden;
  }

  .palette.text {
    width: min(720px, calc(100% - 32px));
  }

  .query {
    height: 44px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--border-strong);
  }

  .glass {
    color: var(--text-dim);
    margin-right: 6px;
  }

  input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-strong);
    font: 14px var(--sans);
  }

  input::placeholder {
    color: var(--text-faint);
  }

  .toggle {
    width: 24px;
    height: 22px;
    flex: none;
    border-radius: 4px;
    color: var(--text-dim);
    font: 600 11px var(--mono);
  }

  .toggle:hover {
    background: var(--surface-4);
    color: var(--text);
  }

  .toggle[aria-pressed='true'] {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent-soft);
  }

  .modes {
    display: flex;
    gap: 2px;
    margin-left: 6px;
    padding: 3px;
    border-radius: 6px;
    background: var(--bg-app);
  }

  .modes button {
    padding: 2px 9px;
    border-radius: 4px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .modes button[aria-selected='true'] {
    background: var(--menu-hover);
    color: var(--text-strong);
  }

  ul {
    flex: 0 1 auto;
    min-height: 0;
    margin: 0;
    padding: 6px;
    list-style: none;
    overflow: auto;
  }

  .text ul {
    max-height: 190px;
    flex: none;
  }

  li button {
    width: 100%;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-radius: 5px;
    font-size: 13px;
    color: var(--text-2);
    text-align: left;
    white-space: nowrap;
  }

  li.hit button {
    height: 28px;
    font-size: 12px;
  }

  li[aria-selected='true'] button {
    background: var(--menu-hover);
    color: var(--text-strong);
  }

  .none {
    padding: 8px 10px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .none:empty {
    display: none;
  }

  .square {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 2px;
  }

  .square.added {
    background: var(--ok);
  }

  .square.modified {
    background: #f0c987;
  }

  .square.deleted {
    background: var(--err);
  }

  .name {
    flex: none;
  }

  .hit .name {
    font-weight: 500;
    color: var(--text-2);
  }

  .dir {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-dim);
    font-size: 12px;
  }

  .on {
    color: var(--accent-soft);
    font-weight: 600;
  }

  .line {
    flex: none;
    color: var(--text-dim);
  }

  code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font: 12px var(--mono);
    color: #e3d7cf;
  }

  mark {
    background: rgba(240, 165, 122, 0.3);
    color: inherit;
    border-radius: 2px;
  }

  .who {
    flex: none;
    margin-left: auto;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-muted);
    font-size: 11.5px;
  }

  .where {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border-top: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text-dim);
    font-size: 11.5px;
  }

  .preview {
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
    padding: 4px 0;
    background: var(--bg-app);
    font: 12px/20px var(--mono);
    color: #e3d7cf;
  }

  .row {
    display: flex;
    white-space: pre;
  }

  .row i {
    width: 48px;
    flex: none;
    padding-right: 12px;
    text-align: right;
    font-style: normal;
    color: #918579;
  }

  .row span {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .row.current {
    background: rgba(237, 230, 225, 0.05);
  }

  .row.current i {
    color: var(--text);
  }

  .foot {
    flex: none;
    display: flex;
    gap: 16px;
    padding: 7px 14px;
    border-top: 1px solid var(--border-strong);
    font-size: 11.5px;
    color: var(--text-dim);
  }

  kbd {
    font: 11px var(--mono);
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }
</style>
