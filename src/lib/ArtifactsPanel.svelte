<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { save } from '@tauri-apps/plugin-dialog'
  import { persist } from './agents.svelte'
  import { fileName } from './artifacts'
  import { ask } from './asking.svelte'
  import { time } from './format'
  import { errorText, t } from './i18n/index.svelte'
  import { panelDocument, themeOf } from './infopanel'
  import Markdown from './Markdown.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let { agent, onclose }: { agent: Agent; onclose: () => void } = $props()

  let selected = $state<string | null>(null)
  let copied = $state(false)
  let host = $state<HTMLElement>()

  const list = $derived(agent.artifacts ?? [])
  const latest = $derived(list.reduce<(typeof list)[number] | undefined>((best, a) => (!best || a.at > best.at ? a : best), undefined))
  const open = $derived(list.find((a) => a.id === selected) ?? latest)
  const doc = $derived(host && open && open.kind !== 'markdown' ? panelDocument(open.content, themeOf(host), true) : '')

  function copy() {
    if (!open) return
    navigator.clipboard.writeText(open.content)
    copied = true
    setTimeout(() => (copied = false), 1500)
  }

  async function keep() {
    if (!open) return
    const artifact = open
    const path = await save({ defaultPath: fileName(artifact) })
    if (path) await invoke('save_text_file', { path, text: artifact.content }).catch((e) => ask(errorText(e), { kind: 'error' }))
  }

  async function remove() {
    if (!open) return
    const artifact = open
    if (!(await ask(t('agent.artifactDeleteAsk', { title: artifact.title }), { title: t('agent.artifactDeleteTitle'), kind: 'warning', okLabel: t('agent.artifactDelete') }))) return
    agent.artifacts = list.filter((a) => a.id !== artifact.id)
    persist(agent)
    if (!agent.artifacts.length) onclose()
  }
</script>

<SidePanel kind="artifacts" title={t('agent.artifactsTitle')} {onclose}>
  {#snippet sub()}
    {open ? `${open.title} · ${t('agent.artifactVersion', { version: open.version, time: time(open.at) })}` : t('agent.artifactsOf', { name: agent.name })}
  {/snippet}
  {#snippet actions()}
    {#if open}
      <button class="btn btn-ghost" onclick={copy}>{copied ? t('common.copied') : t('agent.artifactCopy')}</button>
      <button class="btn btn-ghost" onclick={keep}>{t('agent.artifactSave')}</button>
      <button class="btn btn-ghost danger" onclick={remove}>{t('agent.artifactDelete')}</button>
    {/if}
  {/snippet}

  {#if list.length > 1}
    <div class="tabs" role="tablist" aria-label={t('agent.artifactList')}>
      {#each list as artifact (artifact.id)}
        <button role="tab" aria-selected={artifact === open} title={artifact.title} onclick={() => (selected = artifact.id)}>
          <span class="name">{artifact.title}</span>
          <span class="kind">{artifact.kind}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="view" bind:this={host}>
    {#if open?.kind === 'markdown'}
      <div class="markdown"><Markdown text={open.content} /></div>
    {:else if open}
      <iframe title={open.title} sandbox="" referrerpolicy="no-referrer" srcdoc={doc}></iframe>
    {/if}
  </div>
</SidePanel>

<style>
  .tabs {
    flex: none;
    display: flex;
    gap: 4px;
    padding: 8px 12px;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    flex: none;
    max-width: 240px;
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 5px 10px;
    border-radius: 4px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .tabs button:hover {
    background: var(--surface-2);
  }

  .tabs button[aria-selected='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .kind {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--text-faint);
  }

  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-main);
  }

  .markdown {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
  }

  iframe {
    flex: 1;
    width: 100%;
    border: 0;
  }

  .danger:hover {
    color: var(--err);
  }
</style>
