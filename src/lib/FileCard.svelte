<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { tick } from 'svelte'
  import { slide } from 'svelte/transition'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { KIND_ICONS, kindOf } from './Attachment.svelte'
  import { languageOf } from './CodeEditor.svelte'
  import { revealFile } from './editor.svelte'
  import { fenced, windowLabel } from './filelink'
  import { errorText, t, type Key } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'

  let { path, project }: { path: string; project: string } = $props()

  const PREVIEW = 300
  const TONES = { sheet: '#4cbf85', pdf: '#e5655a', doc: '#6ea8f0', archive: '#e0b65b', code: '#b79cf0', file: 'var(--text-dim)' }

  const name = $derived(path.split('/').pop() ?? path)
  const folder = $derived(path.slice(0, -name.length))
  const kind = $derived(kindOf(path))
  const label = $derived(t(`agent.kind${kind[0].toUpperCase()}${kind.slice(1)}` as Key))

  let open = $state(false)
  let text = $state<string | null>()
  const lines = $derived(text?.split('\n') ?? [])
  let limit = $state(PREVIEW)
  let error = $state('')
  let card: HTMLElement
  let scroller: HTMLElement | null = null
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches

  async function float() {
    const label = windowLabel(project, path)
    const shown = await WebviewWindow.getByLabel(label)
    if (shown) return shown.setFocus()
    new WebviewWindow(label, { url: `index.html?${new URLSearchParams({ project, path })}`, title: `${name} · Squadron`, width: 760, height: 820, minWidth: 420, minHeight: 300 })
  }

  async function toggle() {
    if (!open && text === undefined && !error)
      await invoke<string | null>('read_project_file', { project, path })
        .then((found) => (text = found))
        .catch((e) => (error = errorText(e)))
    open = !open
    if (!open) return
    scroller = card.parentElement
    while (scroller && !/auto|scroll/.test(getComputedStyle(scroller).overflowY)) scroller = scroller.parentElement
    card.dispatchEvent(new Event('unfold', { bubbles: true }))
    scroller?.style.setProperty('overflow-anchor', 'none')
  }

  function more(event: Event) {
    const pre = event.target as HTMLElement
    const body = event.currentTarget as HTMLElement
    if (limit >= lines.length || pre.tagName !== 'PRE' || pre.scrollTop + pre.clientHeight < pre.scrollHeight - 200) return
    const top = pre.scrollTop
    limit += PREVIEW
    tick().then(() => body.querySelector('pre')?.scrollTo({ top }))
  }

  function reveal() {
    if (!scroller) return
    const shown = scroller
    shown.addEventListener('scrollend', () => shown.style.removeProperty('overflow-anchor'), { once: true })
    const view = scroller.getBoundingClientRect()
    const box = card.getBoundingClientRect()
    const by = Math.min(box.bottom + 12 - view.bottom, box.top - view.top - 12)
    if (by > 0) scroller.scrollBy({ top: by, behavior: still ? 'instant' : 'smooth' })
  }
</script>

<div class="card" class:open bind:this={card} style:--kind={TONES[kind]}>
  <div class="head" title={path}>
    <span class="tile" aria-hidden="true"><svg viewBox="0 0 24 24">{@html KIND_ICONS[kind]}</svg></span>
    <span class="text">
      <span class="name">{name}</span>
      <span class="meta">{folder ? `${folder} · ` : ''}{label}</span>
    </span>
  </div>
  <div class="actions">
    <button type="button" class="btn btn-primary" onclick={() => revealFile(project, path)}>
      {t('conversation.goToFile')}
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17L17 7M8 7h9v9" /></svg>
    </button>
    <button type="button" class="btn btn-ghost" aria-expanded={open} onclick={toggle}>
      {open ? t('conversation.hideContent') : t('conversation.showContent')}
      <svg class="chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </button>
    <button type="button" class="btn btn-ghost float" title={t('conversation.openInWindow')} aria-label={t('conversation.openInWindow')} onclick={float}>
      <span class="label">{t('conversation.openInWindow')}</span>
      <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="7" width="14" height="14" rx="2" /><path d="M10 3h11v11" /></svg>
    </button>
  </div>
  {#if open}
    <div class="body" onscrollcapture={more} transition:slide={{ duration: still ? 0 : 260 }} onintroend={reveal}>
      {#if error}
        <p class="note">{error}</p>
      {:else if text === null}
        <p class="note">{t('conversation.fileMissing')}</p>
      {:else}
        <Markdown text={fenced(lines.slice(0, limit).join('\n'), languageOf(name)?.name.toLowerCase())} />
        {#if lines.length > limit}
          <p class="note more">
            {t('conversation.previewLines', { shown: String(limit), total: String(lines.length) })}
            <button type="button" onclick={float}>{t('conversation.openInWindow')}</button>
          </p>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    position: relative;
    container-type: inline-size;
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
  }

  .tile svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
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

  @container (max-width: 440px) {
    .float .label {
      display: none;
    }
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

  .more {
    border-top: 1px solid var(--border);
  }

  .more button {
    color: var(--accent-link);
  }

  .more button:hover {
    text-decoration: underline;
  }
</style>
