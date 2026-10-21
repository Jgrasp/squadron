<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { ask } from '@tauri-apps/plugin-dialog'
  import { untrack } from 'svelte'
  import { ofProject } from './agents.svelte'
  import CodeEditor, { forget, languageOf } from './CodeEditor.svelte'
  import { changedFiles, type Changes } from './diff'
  import { remember, workspace, type OpenFile } from './editor.svelte'
  import FileTree, { type Change } from './FileTree.svelte'
  import { projects } from './projects.svelte'
  import OpenWith from './OpenWith.svelte'
  import SidePanel from './SidePanel.svelte'
  import { calls, relative, type Agent } from './stream'

  let { agent, changes, onclose }: { agent: Agent; changes: Changes | null; onclose: () => void } = $props()

  const project = untrack(() => agent.project)
  const ws = workspace(project)
  const root = $derived(projects.list.find((p) => p.path === project))
  const keyOf = (path: string) => `${project}\n${path}`
  const pathOf = (key: string) => key.slice(project.length + 1)

  let editor = $state<ReturnType<typeof CodeEditor> | null>(null)
  let cursor = $state({ line: 1, col: 1 })
  let notice = $state('')

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
  const kind = $derived(active ? (languageOf(active.path.split('/').pop()!)?.name ?? 'Plain text') : '')
  const authors = $derived(active ? (touched.get(active.path) ?? []) : [])

  async function load(file: OpenFile) {
    try {
      file.saved = (await invoke<string | null>('read_project_file', { project, path: file.path })) ?? ''
    } catch (e) {
      file.error = String(e)
    }
    file.ready = true
  }

  async function open(path: string, activate = true) {
    if (!ws.files.some((f) => f.path === path)) {
      ws.files.push({ path, saved: '', dirty: false, error: '', base: undefined, ready: false })
      load(ws.files.at(-1)!)
    }
    if (activate) ws.active = path
    remember(project)
  }

  async function shut(file: OpenFile) {
    if (file.dirty && !(await ask(`Discard your changes to ${file.path}?`, { title: 'Unsaved changes', kind: 'warning', okLabel: 'Discard', cancelLabel: 'Keep editing' })))
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
    if (file) file.dirty = doc !== file.saved
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
      notice = `Could not save: ${e}`
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
          if (file === active) editor?.replace(text)
          else forget(keyOf(file.path))
        })
        .catch(() => {})
    }
  }
</script>

<SidePanel kind="editor" title="Editor" {onclose}>
  {#snippet sub()}
    {changes ? 'Project files · colors show what is not committed yet' : 'Project files · colors show what the agents changed'}
  {/snippet}
  {#snippet actions()}
    <OpenWith {project} path={active?.path ?? ''} />
  {/snippet}
  <div class="body" style:--project={root?.color}>
    <nav aria-label="Project files">
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
        <div class="tabs" role="tablist" aria-label="Open files">
          {#each ws.files as file (file.path)}
            <span class="tab" class:selected={file.path === ws.active}>
              <button role="tab" aria-selected={file.path === ws.active} title={file.path} onclick={() => (ws.active = file.path)}>
                {file.path.split('/').pop()}
                {#if file.dirty}<span class="unsaved" aria-label="unsaved"></span>{/if}
              </button>
              <button class="shut" aria-label="Close {file.path}" onclick={() => shut(file)}>×</button>
            </span>
          {/each}
        </div>
      {/if}

      {#if active && authors.length}
        <div class="notice">
          <span class="dot" aria-hidden="true"></span>
          <span>
            Changed by {authors.join(', ')}. Your edits are saved to the file on disk: the agent works from your version the next time it
            reads it.
          </span>
        </div>
      {/if}

      {#if !active}
        <p class="empty">Pick a file in the tree.</p>
      {:else if !active.ready}
        <p class="empty">Reading the file…</p>
      {:else if active.error}
        <p class="empty">{active.error}</p>
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
        />
      {/if}

      {#if active?.ready && !active.error}
        <div class="status">
          <span>{kind}</span>
          <span>UTF-8</span>
          <span>L{cursor.line}, C{cursor.col}</span>
          <span class="grow">{notice}</span>
          {#if active.base !== undefined}
            <span class="legend" title="Lines added or changed since the last commit"><span class="added"></span>added</span>
            <span class="legend" title="Lines removed since the last commit"><span class="removed"></span>removed</span>
          {/if}
          <span>Ctrl S to save</span>
        </div>
      {/if}
    </div>
  </div>
</SidePanel>

<style>
  .body {
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
