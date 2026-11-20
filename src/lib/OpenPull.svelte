<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import Markdown from './Markdown.svelte'
  import { forgeName } from './forges.svelte'
  import { openPull } from './pulls.svelte'
  import { workdir, type Agent } from './stream'
  import { errorText, t } from './i18n/index.svelte'

  let { agent, branch, host, onclose }: { agent: Agent; branch: string; host: string; onclose: () => void } = $props()

  let title = $state('')
  let base = $state('')
  let description = $state('')
  let draft = $state(false)
  let typed = { title: false, description: false }
  let writing = $state(true)
  let tab = $state<'write' | 'preview'>('write')
  let opening = $state(false)
  let problem = $state('')

  onMount(async () => {
    try {
      const outline = await invoke<{ base: string; subjects: string[]; log: string }>('pull_outline', { dir: workdir(agent) })
      base ||= outline.base
      if (!typed.title) title = outline.subjects.length === 1 ? outline.subjects[0] : branch.replace(/^[^/]*\//, '').replace(/[-_]+/g, ' ')
      if (!typed.description && outline.subjects.length) description = ['## Changes', ...outline.subjects.map((s) => `- ${s}`)].join('\n')
      const suggested = await invoke<{ title: string; description: string }>('suggest_pull', { log: outline.log })
      if (!typed.title) title = suggested.title
      if (!typed.description) description = suggested.description
    } catch {
    } finally {
      writing = false
    }
  })

  async function submit(event: Event) {
    event.preventDefault()
    if (!title.trim() || !base.trim() || opening) return
    opening = true
    problem = ''
    try {
      await openPull(agent, { branch, base: base.trim(), title: title.trim(), description, draft })
      onclose()
    } catch (e) {
      problem = errorText(e)
    } finally {
      opening = false
    }
  }
</script>

<Dialog title={t('review.openPullTitle')} width={880} {onclose}>
  <form onsubmit={submit}>
    <p class="intro"><span class="mono">{branch}</span> → <input class="field mono base" aria-label={t('review.openPullBase')} bind:value={base} spellcheck="false" /> {t('review.openPullOn', { host: forgeName(host) })}</p>
    <div class="fields">
      <input class="field" aria-label={t('review.openPullTitleField')} placeholder={t('review.openPullTitleField')} bind:value={title} oninput={() => (typed.title = true)} />
      <div class="box">
        <div class="box-head">
          <span class="views">
            <button type="button" aria-pressed={tab === 'write'} onclick={() => (tab = 'write')}>{t('review.write')}</button>
            <button type="button" aria-pressed={tab === 'preview'} onclick={() => (tab = 'preview')}>{t('conversation.preview')}</button>
          </span>
          <span class="hint">{writing ? t('review.openPullWriting') : t('conversation.markdown')}</span>
        </div>
        {#if tab === 'write'}
          <textarea aria-label={t('common.description')} bind:value={description} oninput={() => (typed.description = true)} spellcheck="false"></textarea>
        {:else}
          <div class="preview"><Markdown text={description} /></div>
        {/if}
      </div>
      <label class="check"><input type="checkbox" bind:checked={draft} /> {t('review.openPullDraft')}</label>
    </div>
    <footer>
      {#if problem}<span class="bad" role="alert">{problem}</span>{/if}
      <button type="button" class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!title.trim() || !base.trim() || opening}>{opening ? t('review.openPullOpening') : t('review.openPullSubmit', { host: forgeName(host) })}</button>
    </footer>
  </form>
</Dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 160px);
  }

  .intro {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 0 22px 14px;
    font-size: 13px;
    color: var(--text-dim);
  }

  .mono {
    font: 12.5px var(--mono);
    color: var(--text-3);
  }

  .base {
    width: 160px;
    height: 28px;
    padding: 0 8px;
  }

  .fields {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 22px 18px;
  }

  .box {
    flex: 1;
    min-height: 320px;
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--border);
    overflow: hidden;
  }

  .box-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
  }

  .views {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .views button {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    font: 500 12px var(--sans);
    color: var(--text-dim);
  }

  .views button[aria-pressed='true'] {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .hint {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-faint);
  }

  textarea {
    flex: 1;
    min-height: 320px;
    padding: 12px 14px;
    border: none;
    resize: none;
    background: var(--bg-main);
    color: var(--text);
    font: 12.5px/1.6 var(--mono);
    outline: none;
  }

  .preview {
    flex: 1;
    min-height: 320px;
    overflow: auto;
    padding: 12px 16px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-2);
    cursor: pointer;
  }

  .check input {
    accent-color: var(--accent);
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }

  .bad {
    margin-right: auto;
    font-size: 12.5px;
    color: var(--err-text);
  }
</style>
