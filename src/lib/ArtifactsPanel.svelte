<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { save } from '@tauri-apps/plugin-dialog'
  import { persist, send } from './agents.svelte'
  import { docsOf, fileName, sharePrompt, type Doc } from './artifacts'
  import { ask } from './asking.svelte'
  import { time } from './format'
  import { errorText, t } from './i18n/index.svelte'
  import { panelDocument, themeOf } from './infopanel'
  import Markdown from './Markdown.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent, Artifact } from './stream'

  let { agent, onclose }: { agent: Agent; onclose: () => void } = $props()

  let selected = $state<string | null>(null)
  let copied = $state('')
  let asked = $state<string[]>([])
  let host = $state<HTMLElement>()

  const local = $derived(agent.artifacts ?? [])
  const docs = $derived(docsOf(agent))
  const list = $derived<(Artifact | Doc)[]>([...local, ...docs])
  const latest = $derived(local.reduce<Artifact | undefined>((best, a) => (!best || a.at > best.at ? a : best), undefined) ?? docs.at(-1))
  const open = $derived(list.find((a) => a.id === selected) ?? latest)
  const doc = $derived(host && open && open.kind !== 'markdown' && open.kind !== 'doc' ? panelDocument(open.content, themeOf(host), true) : '')

  function copy(what: string, text: string) {
    navigator.clipboard.writeText(text)
    copied = what
    setTimeout(() => (copied = ''), 1500)
  }

  async function keep(artifact: Artifact) {
    const path = await save({ defaultPath: fileName(artifact) })
    if (path) await invoke('save_text_file', { path, text: artifact.content }).catch((e) => ask(errorText(e), { kind: 'error' }))
  }

  async function share(artifact: Artifact) {
    asked = [...asked, artifact.id]
    if (!(await ask(t('agent.artifactShareAsk', { title: artifact.title, name: agent.name }), { title: t('agent.artifactShareTitle'), okLabel: t('agent.artifactShare') })))
      return void (asked = asked.filter((id) => id !== artifact.id))
    await send(agent.id, sharePrompt(artifact))
  }

  async function remove(item: Artifact | Doc) {
    if (item.kind === 'doc') {
      agent.hiddenDocs = [...(agent.hiddenDocs ?? []), item.link]
    } else {
      if (!(await ask(t('agent.artifactDeleteAsk', { title: item.title }), { title: t('agent.artifactDeleteTitle'), kind: 'warning', okLabel: t('agent.artifactDelete') }))) return
      agent.artifacts = local.filter((a) => a.id !== item.id)
    }
    persist(agent)
    if (list.length <= 1) onclose()
  }
</script>

<SidePanel kind="artifacts" title={t('agent.artifactsTitle')} {onclose}>
  {#snippet sub()}
    {#if open?.kind === 'doc'}{open.title} · {t('agent.artifactOnClaude')}{:else if open}{open.title} · {t('agent.artifactVersion', { version: open.version, time: time(open.at) })}{:else}{t('agent.artifactsOf', { name: agent.name })}{/if}
  {/snippet}
  {#snippet actions()}
    {#if open?.kind === 'doc'}
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactHide')}</button>
    {:else if open}
      <button class="btn btn-ghost" onclick={() => copy('source', open.content)}>{copied === 'source' ? t('common.copied') : t('agent.artifactCopy')}</button>
      <button class="btn btn-ghost" onclick={() => keep(open)}>{t('agent.artifactSave')}</button>
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactDelete')}</button>
    {/if}
  {/snippet}

  {#if list.length > 1}
    <div class="tabs" role="tablist" aria-label={t('agent.artifactList')}>
      {#each list as artifact (artifact.id)}
        <button role="tab" aria-selected={artifact === open} title={artifact.title} onclick={() => (selected = artifact.id)}>
          <span class="name">{artifact.title}</span>
          <span class="kind">{artifact.kind === 'doc' ? 'claude.ai' : artifact.kind}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if open && (open.kind === 'doc' || open.link || open.kind === 'markdown')}
    <div class="share">
      {#if open.kind === 'doc' || open.link}
        {@const link = open.link!}
        <span class="where">{t('agent.artifactShared')} <span class="mono">{link.replace('https://', '')}</span></span>
        <button class="btn btn-ghost" onclick={() => copy('link', link)}>{copied === 'link' ? t('common.copied') : t('agent.artifactCopyLink')}</button>
        <button class="btn btn-primary" onclick={() => invoke('open_link', { url: link })}>{t('agent.artifactOpenClaude')}</button>
      {:else if asked.includes(open.id)}
        <span class="where">{t('agent.artifactSharing', { name: agent.name })}</span>
      {:else}
        <span class="where">{t('agent.artifactShareHint')}</span>
        <button class="btn btn-ghost" onclick={() => share(open)}>{t('agent.artifactShare')}</button>
      {/if}
    </div>
  {/if}

  <div class="view" bind:this={host}>
    {#if open?.kind === 'doc'}
      <div class="remote">
        <p>{t('agent.artifactRemote')}</p>
      </div>
    {:else if open?.kind === 'markdown'}
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

  .share {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px 8px 20px;
    border-bottom: 1px solid var(--border);
    font-size: 12.5px;
  }

  .where {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .where .mono {
    color: var(--text);
  }

  .remote {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 32px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .danger:hover {
    color: var(--err);
  }
</style>
