<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from './asking.svelte'
  import { untrack } from 'svelte'
  import { ofProject } from './agents.svelte'
  import CodeEditor, { forget, languageOf, type Goto } from './CodeEditor.svelte'
  import { changedFiles, type Changes } from './diff'
  import { openFile, remember, workspace, type OpenFile } from './editor.svelte'
  import FileTree, { type Change } from './FileTree.svelte'
  import Markdown from './Markdown.svelte'
  import { projects } from './projects.svelte'
  import OpenWith from './OpenWith.svelte'
  import QuickOpen from './QuickOpen.svelte'
  import SidePanel from './SidePanel.svelte'
  import { calls, relative } from './stream'
  import { errorText, list, t } from './i18n/index.svelte'

  let { project: folder, changes, onclose }: { project: string; changes: Changes | null; onclose: () => void } = $props()

  const project = untrack(() => folder)
  const ws = workspace(project)
  const root = $derived(projects.list.find((p) => p.path === project))
  const keyOf = (path: string) => `${project}\n${path}`
  const pathOf = (key: string) => key.slice(project.length + 1)

  let editor = $state<ReturnType<typeof CodeEditor> | null>(null)
  let cursor = $state({ line: 1, col: 1 })
  let notice = $state('')
  let quick = $state(false)
  let pending = $state<{ path: string; goto: Goto } | null>(null)

  const touched = $derived.by(() => {
    const map = new Map<string, string[]>()
    for (const a of ofProject(project))
      for (const f of changedFiles(calls(a))) {
        const path = relative(a, f.path)
        map.set(path, [...new Set([...(map.get(path) ?? []), a.name])])
      }
    return map
  })
  const status = $derived.by(() => {
    const map = new Map<string, Change>()
    if (changes) for (const f of changes.files) map.set(f.path, f.status === 'A' ? 'added' : f.status === 'D' ? 'deleted' : 'modified')
    else
      for (const a of ofProject(project))
        for (const f of changedFiles(calls(a))) map.set(relative(a, f.path), f.baseline === null ? 'added' : 'modified')
    return map
  })
  const folders = $derived.by(() => {
    const map = new Map<string, Set<Change>>()
    for (const [path, change] of status) {
      const parts = path.split('/')
      for (let i = 1; i < parts.length; i++) {
        const dir = parts.slice(0, i).join('/')
        if (!map.has(dir)) map.set(dir, new Set())
        map.get(dir)!.add(change)
      }
    }
    return map
  })
  const active = $derived(ws.files.find((f) => f.path === ws.active))
  const kind = $derived(active ? (languageOf(active.path.split('/').pop()!)?.name ?? t('shell.plainText')) : '')
  const authors = $derived(active ? (touched.get(active.path) ?? []) : [])
  const format = $derived(active ? (/\.(md|markdown|mdx)$/i.test(active.path) ? 'markdown' : /\.html?$/i.test(active.path) ? 'html' : '') : '')
  const previewing = $derived(!!format && !!active && ws.preview.includes(active.path))

  function flip() {
    if (!active) return
    ws.preview = previewing ? ws.preview.filter((p) => p !== active.path) : [...ws.preview, active.path]
    remember(project)
  }

  const open = (path: string, activate = true) => openFile(project, path, activate)

  function pick(path: string, goto: Goto | null) {
    quick = false
    pending = goto && { path, goto }
    open(path)
  }

  function keys(event: KeyboardEvent) {
    if (!event.ctrlKey || !event.shiftKey || event.altKey || event.key.toLowerCase() !== 'f' || document.querySelector('dialog[open]')) return
    event.preventDefault()
    quick = !quick
  }

  async function shut(file: OpenFile) {
    if (file.dirty && !(await ask(t('shell.discardAsk', { path: file.path }), { title: t('shell.unsavedChanges'), kind: 'warning', okLabel: t('shell.discard'), cancelLabel: t('shell.keepEditing') })))
      return
    const at = ws.files.indexOf(file)
    ws.files.splice(at, 1)
    forget(keyOf(file.path))
    if (ws.active === file.path) ws.active = ws.files[Math.min(at, ws.files.length - 1)]?.path ?? ''
    remember(project)
  }

  function toggle(path: string) {
    ws.expanded = ws.expanded.includes(path) ? ws.expanded.filter((p) => p !== path) : [...ws.expanded, path]
    remember(project)
  }

  function change(key: string, doc: string) {
    const file = ws.files.find((f) => f.path === pathOf(key))
    if (!file) return
    file.doc = doc
    file.dirty = doc !== file.saved
  }

  async function save(key: string, doc: string) {
    const file = ws.files.find((f) => f.path === pathOf(key))
    if (!file) return
    try {
      await invoke('write_project_file', { project, path: file.path, text: doc })
      file.saved = doc
      file.dirty = false
      notice = ''
    } catch (e) {
      notice = t('shell.couldNotSave', { error: errorText(e) })
    }
  }

  for (const path of ws.restore.splice(0)) open(path, false)

  $effect(() => {
    const from = changes?.from
    const file = active
    if (!from || !file || file.base !== undefined) return
    invoke<string | null>('git_file', { project, rev: from, path: file.path })
      .then((text) => (file.base = text))
      .catch(() => {})
  })

  $effect(() => {
    void changes?.now
    untrack(reload)
  })

  function reload() {
    for (const file of ws.files) {
      if (!file.ready || file.dirty || file.error) continue
      invoke<string | null>('read_project_file', { project, path: file.path })
        .then((text) => {
          if (text === null || text === file.saved || file.dirty) return
          file.saved = text
          file.doc = text
          if (file === active && editor) editor.replace(text)
          else forget(keyOf(file.path))
        })
        .catch(() => {})
    }
  }
</script>

<svelte:window onkeydown={keys} />

<SidePanel kind="editor" title={t('shell.editor')} {onclose}>
  {#snippet sub()}
    {changes ? t('shell.filesUncommitted') : t('shell.filesChangedByAgents')}
  {/snippet}
  {#snippet actions()}
    <button class="btn btn-secondary flip" title="Ctrl Shift F" onclick={() => (quick = true)}>⌕ {t('shell.quickOpen')}</button>
    {#if format}
      <button class="btn btn-secondary flip" aria-pressed={previewing} title={previewing ? undefined : t('shell.previewOf', { name: active!.path.split('/').pop()! })} onclick={flip}>
        {previewing ? t('shell.code') : t('shell.preview')}
      </button>
    {/if}
    <OpenWith {project} path={active?.path ?? ''} />
  {/snippet}
  <div class="body" style:--project={root?.color}>
    <nav aria-label={t('shell.projectFiles')}>
      <ul>
        <li class="root">▾ {root?.name}</li>
        <FileTree
          {project}
          version={changes?.now ?? ''}
          expanded={ws.expanded}
          active={ws.active}
          {status}
          {folders}
          authors={touched}
          onopen={open}
          ontoggle={toggle}
        />
      </ul>
    </nav>

    <div class="main">
      {#if ws.files.length}
        <div class="tabs" role="tablist" aria-label={t('shell.openFiles')}>
          {#each ws.files as file (file.path)}
            <span class="tab" class:selected={file.path === ws.active}>
              <button role="tab" aria-selected={file.path === ws.active} title={file.path} onclick={() => (ws.active = file.path)}>
                {file.path.split('/').pop()}
                {#if file.dirty}<span class="unsaved" aria-label={t('shell.unsaved')}></span>{/if}
              </button>
              <button class="shut" aria-label={t('shell.closeNamed', { name: file.path })} onclick={() => shut(file)}>×</button>
            </span>
          {/each}
        </div>
      {/if}

      {#if active && authors.length}
        <div class="notice">
          <span class="dot" aria-hidden="true"></span>
          <span>{t('shell.changedBy', { names: list(authors) })}</span>
        </div>
      {/if}

      {#if !active}
        <p class="empty">{t('shell.pickFile')}</p>
      {:else if !active.ready}
        <p class="empty">{t('shell.readingFile')}</p>
      {:else if active.error}
        <p class="empty">{active.error}</p>
      {:else if previewing && format === 'markdown'}
        <div class="preview"><Markdown text={active.doc ?? active.saved} /></div>
      {:else if previewing}
        <iframe class="page" title={active.path} sandbox="" srcdoc={active.doc ?? active.saved}></iframe>
      {:else}
        <CodeEditor
          bind:this={editor}
          key={keyOf(active.path)}
          name={active.path.split('/').pop()!}
          text={active.saved}
          base={active.base}
          onchange={change}
          onsave={save}
          oncursor={(line, col) => (cursor = { line, col })}
          goto={pending?.path === active.path ? pending.goto : null}
          ongoto={() => (pending = null)}
        />
      {/if}

      {#if active?.ready && !active.error}
        <div class="status">
          <span>{kind}</span>
          <span>UTF-8</span>
          {#if !previewing}<span>{t('shell.cursor', { line: String(cursor.line), col: String(cursor.col) })}</span>{/if}
          <span class="grow">{notice}</span>
          {#if active.base !== undefined}
            <span class="legend" title={t('shell.linesAdded')}><span class="added"></span>{t('shell.added')}</span>
            <span class="legend" title={t('shell.linesRemoved')}><span class="removed"></span>{t('shell.removed')}</span>
          {/if}
          <span>{t('shell.saveHint')}</span>
        </div>
      {/if}
    </div>

    {#if quick}
      <QuickOpen {project} recent={ws.files.map((f) => f.path)} {status} authors={touched} onpick={pick} onclose={() => (quick = false)} />
    {/if}
  </div>
</SidePanel>

<style>
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    background: var(--bg-app);
  }

  nav {
    width: 220px;
    flex: none;
    overflow: auto;
    padding: 10px 6px;
    border-right: 1px solid var(--border);
    background: var(--bg-sidebar);
    font-size: 12.5px;
  }

  nav > ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .root {
    height: 26px;
    display: flex;
    align-items: center;
    padding: 0 8px;
    color: var(--text-muted);
    font-weight: 600;
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .tabs {
    height: 36px;
    flex: none;
    display: flex;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    background: var(--bg-sidebar);
    font-size: 12.5px;
  }

  .tab {
    position: relative;
    display: flex;
    flex: none;
    border-right: 1px solid var(--border);
  }

  .tab [role='tab'] {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 30px 0 14px;
    color: var(--text-muted);
  }

  .tab.selected {
    background: var(--bg-app);
    box-shadow: inset 0 2px 0 var(--project, var(--accent));
  }

  .tab.selected [role='tab'] {
    color: var(--text-strong);
  }

  .unsaved {
    width: 7px;
    height: 7px;
    border-radius: 4px;
    background: var(--text);
  }

  .shut {
    position: absolute;
    top: 8px;
    right: 6px;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    color: var(--text-dim);
    opacity: 0;
  }

  .tab:hover .shut,
  .tab.selected .shut,
  .shut:focus-visible {
    opacity: 1;
  }

  .shut:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .notice {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-bottom: 1px solid #5a3322;
    background: #2a1f14;
    font-size: 12.5px;
    color: var(--wait-text-2);
  }

  .notice .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 4px;
    background: var(--accent);
  }

  .empty {
    flex: 1;
    margin: 0;
    padding: 24px 18px;
    color: var(--text-dim);
  }

  .flip {
    height: 32px;
    padding: 0 12px;
    font-size: 13px;
  }

  .preview {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 18px 28px;
    font-size: 14px;
    color: var(--text);
  }

  .page {
    flex: 1;
    min-height: 0;
    border: none;
    background: #fff;
  }

  .status {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    background: var(--bg-sidebar);
    font-size: 12px;
    color: var(--text-dim);
    white-space: nowrap;
  }

  .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--err-text);
  }

  .legend {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .added {
    width: 8px;
    height: 8px;
    background: #9ad6cb;
  }

  .removed {
    width: 8px;
    height: 3px;
    background: #f59e8c;
  }
</style>
