<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { kindOf } from './Attachment.svelte'
  import { languageOf } from './CodeEditor.svelte'
  import { openFile, workspace } from './editor.svelte'
  import { fenced, windowLabel } from './filelink'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import { setPanel } from './layout.svelte'
  import Markdown from './Markdown.svelte'

  let { path, project }: { path: string; project: string } = $props()

  const TONES = { sheet: '#4cbf85', pdf: '#e5655a', doc: '#6ea8f0', archive: '#e0b65b', code: '#b79cf0', file: 'var(--text-dim)' }

  const name = $derived(path.split('/').pop() ?? path)
  const folder = $derived(path.slice(0, -name.length))
  const ext = $derived(name.includes('.') ? name.split('.').pop()!.toUpperCase().slice(0, 4) : '')
  const kind = $derived(kindOf(path))
  const label = $derived(t(`agent.kind${kind[0].toUpperCase()}${kind.slice(1)}` as Key))

  let open = $state(false)
  let text = $state<string | null>()
  let error = $state('')


  function go() {
    const ws = workspace(project)
    const dirs = path.split('/').slice(0, -1).map((_, i, parts) => parts.slice(0, i + 1).join('/'))
    ws.expanded = [...new Set([...ws.expanded, ...dirs])]
    openFile(project, path)
    setPanel(project, 'editor')
  }

  async function float() {
    const label = windowLabel(project, path)
    const shown = await WebviewWindow.getByLabel(label)
    if (shown) return shown.setFocus()
    new WebviewWindow(label, { url: `index.html?${new URLSearchParams({ project, path })}`, title: `${name} · Squadron`, width: 760, height: 820, minWidth: 420, minHeight: 300 })
  }

  function toggle() {
    open = !open
    if (!open || text !== undefined || error) return
    invoke<string | null>('read_project_file', { project, path })
      .then((found) => (text = found))
      .catch((e) => (error = errorText(e)))
  }
</script>

<div class="card" class:open style:--kind={TONES[kind]}>
  <div class="head" title={path}>
    <span class="tile" aria-hidden="true">{ext || '·'}</span>
    <span class="text">
      <span class="name">{name}</span>
      <span class="meta">{folder ? `${folder} · ` : ''}{label}</span>
    </span>
  </div>
  <div class="actions">
    <button type="button" class="btn btn-primary" onclick={go}>
      {t('conversation.goToFile')}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17L17 7M8 7h9v9" /></svg>
    </button>
    <button type="button" class="btn btn-ghost" aria-expanded={open} onclick={toggle}>
      {open ? t('conversation.hideContent') : t('conversation.showContent')}
      <svg class="chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </button>
    <button type="button" class="btn btn-ghost float" onclick={float}>
      {t('conversation.openInWindow')}
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="7" width="14" height="14" rx="2" /><path d="M10 3h11v11" /></svg>
    </button>
  </div>
  {#if open}
    <div class="body">
      {#if error}
        <p class="note">{error}</p>
      {:else if text === null}
        <p class="note">{t('conversation.fileMissing')}</p>
      {:else if text === undefined}
        <p class="note">{t('common.loading')}</p>
      {:else}
        <Markdown text={fenced(text, languageOf(name)?.name.toLowerCase())} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    position: relative;
    max-width: 560px;
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow: 0 0 0 1px var(--border-strong);
    overflow: hidden;
    line-height: 1.4;
  }

  .card::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    z-index: 1;
    width: 3px;
    background: var(--kind);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 15px;
  }

  .tile {
    flex: none;
    width: 30px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    background: color-mix(in srgb, var(--kind) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--kind) 35%, transparent);
    color: var(--kind);
    font: 700 9.5px/1 var(--sans);
    letter-spacing: 0.04em;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-strong);
  }

  .meta {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .actions {
    display: flex;
    gap: 6px;
    padding: 8px 14px 10px 15px;
    border-top: 1px solid var(--border);
    background: var(--surface-1);
  }

  .actions .btn {
    height: 28px;
    gap: 6px;
    padding: 0 11px;
    font-size: 12.5px;
  }

  .btn svg {
    width: 12px;
    height: 12px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .float {
    margin-left: auto;
  }

  .btn[aria-expanded='true'] {
    background: var(--surface-5);
    box-shadow: none;
    color: var(--text-strong);
  }

  .btn[aria-expanded='true'] .chevron {
    transform: rotate(180deg);
  }

  .body {
    border-top: 1px solid var(--border);
  }

  .body :global(.code) {
    margin: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
  }

  .note {
    margin: 0;
    padding: 12px 15px;
    font-size: 12.5px;
    color: var(--text-dim);
  }
</style>
