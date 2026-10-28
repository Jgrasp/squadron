<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { save } from '@tauri-apps/plugin-dialog'
  import { persist, send } from './agents.svelte'
  import { artifactUrl, docsOf, fileName, sharePrompt, type ArtifactInfo, type Doc } from './artifacts'
  import { refresh, shelf } from './artifacts.svelte'
  import { ask } from './asking.svelte'
  import { time } from './format'
  import { errorText, t } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let { agent, onpoint, onclose }: { agent: Agent; onpoint: (text: string) => void; onclose: () => void } = $props()

  let selected = $state<{ id: string; at: number } | null>(null)
  let copied = $state('')
  let asked = $state<string[]>([])
  let pointing = $state(false)
  let markdown = $state('')
  let frame = $state<HTMLIFrameElement>()

  const local = $derived(shelf.of[agent.id] ?? [])
  const docs = $derived(docsOf(agent, local.flatMap((a) => (a.link ? [a.link] : []))))
  const list = $derived<(ArtifactInfo | Doc)[]>([...local, ...docs])
  const latest = $derived(local.reduce<ArtifactInfo | undefined>((best, a) => (!best || a.modified > best.modified ? a : best), undefined))
  const open = $derived.by(() => {
    const picked = selected && list.find((a) => a.id === selected!.id)
    return picked && !(latest && latest.id !== picked.id && latest.modified > selected!.at) ? picked : (latest ?? docs.at(-1))
  })

  $effect(() => {
    if (open?.kind !== 'markdown') return
    const { id } = open
    void open.modified
    invoke<string>('read_artifact', { agent: agent.id, id })
      .then((text) => (markdown = text))
      .catch(() => (markdown = ''))
  })

  $effect(() => {
    void open?.id
    pointing = false
  })

  function aim(on: boolean) {
    pointing = on
    frame?.contentWindow?.postMessage({ squadron: 'pick', on }, '*')
  }

  function message(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow || !open || open.kind === 'doc') return
    if (e.data?.squadron === 'pick-off') pointing = false
    if (e.data?.squadron !== 'picked') return
    pointing = false
    const { selector, html, page } = e.data as { selector: string; html: string; page: string }
    onpoint(`${t('agent.artifactPointed', { title: open.title, page: page || open.entry, selector: String(selector) })}\n\n\`\`\`html\n${String(html)}\n\`\`\``)
  }

  function choose(id: string) {
    selected = { id, at: Date.now() }
  }

  function copy(what: string, text: string) {
    navigator.clipboard.writeText(text)
    copied = what
    setTimeout(() => (copied = ''), 1500)
  }

  async function source(artifact: ArtifactInfo) {
    copy('source', await invoke<string>('read_artifact', { agent: agent.id, id: artifact.id }))
  }

  async function keep(artifact: ArtifactInfo) {
    const path = await save({ defaultPath: fileName(artifact) })
    if (!path) return
    const text = await invoke<string>('read_artifact', { agent: agent.id, id: artifact.id })
    await invoke('save_text_file', { path, text }).catch((e) => ask(errorText(e), { kind: 'error' }))
  }

  async function share(artifact: ArtifactInfo) {
    asked = [...asked, artifact.id]
    if (!(await ask(t('agent.artifactShareAsk', { title: artifact.title, name: agent.name }), { title: t('agent.artifactShareTitle'), okLabel: t('agent.artifactShare') })))
      return void (asked = asked.filter((id) => id !== artifact.id))
    await send(agent.id, sharePrompt(artifact))
  }

  async function remove(item: ArtifactInfo | Doc) {
    if (item.kind === 'doc') {
      agent.hiddenDocs = [...(agent.hiddenDocs ?? []), item.link]
      persist(agent)
    } else {
      if (!(await ask(t('agent.artifactDeleteAsk', { title: item.title }), { title: t('agent.artifactDeleteTitle'), kind: 'warning', okLabel: t('agent.artifactDelete') }))) return
      await invoke('delete_artifact', { agent: agent.id, id: item.id }).catch((e) => ask(errorText(e), { kind: 'error' }))
      await refresh(agent.id)
    }
    if (list.length <= 1) onclose()
  }
</script>

<svelte:window onmessage={message} />

<SidePanel kind="artifacts" title={t('agent.artifactsTitle')} {onclose}>
  {#snippet sub()}
    {#if open?.kind === 'doc'}{open.title} · {t('agent.artifactOnClaude')}{:else if open}{open.title} · {t('agent.artifactModified', { time: time(open.modified) })}{:else}{t('agent.artifactsOf', { name: agent.name })}{/if}
  {/snippet}
  {#snippet actions()}
    {#if open?.kind === 'doc'}
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactHide')}</button>
    {:else if open}
      {#if open.kind === 'html'}
        <button class="btn btn-ghost" aria-pressed={pointing} title={t('agent.artifactPointTitle')} onclick={() => aim(!pointing)}>{pointing ? t('agent.artifactPointing') : t('agent.artifactPoint')}</button>
      {/if}
      <button class="btn btn-ghost" onclick={() => source(open)}>{copied === 'source' ? t('common.copied') : t('agent.artifactCopy')}</button>
      {#if open.kind === 'html'}
        <button class="btn btn-ghost" onclick={() => invoke('open_artifact_folder', { agent: agent.id, id: open.id }).catch((e) => ask(errorText(e), { kind: 'error' }))}>{t('agent.artifactFolder')}</button>
      {:else}
        <button class="btn btn-ghost" onclick={() => keep(open)}>{t('agent.artifactSave')}</button>
      {/if}
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactDelete')}</button>
    {/if}
  {/snippet}

  {#if list.length > 1}
    <div class="tabs" role="tablist" aria-label={t('agent.artifactList')}>
      {#each list as artifact (artifact.id)}
        <button role="tab" aria-selected={artifact === open} title={artifact.title} onclick={() => choose(artifact.id)}>
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

  <div class="view" class:pointing>
    {#if open?.kind === 'doc'}
      <div class="remote">
        <p>{t('agent.artifactRemote')}</p>
      </div>
    {:else if open?.kind === 'markdown'}
      <div class="markdown"><Markdown text={markdown} /></div>
    {:else if open}
      <iframe
        bind:this={frame}
        title={open.title}
        src={artifactUrl(agent.id, open)}
        sandbox="allow-scripts allow-forms allow-modals"
        referrerpolicy="no-referrer"
        onload={() => pointing && aim(true)}
      ></iframe>
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
    background: #fff;
  }

  .pointing iframe {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
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
