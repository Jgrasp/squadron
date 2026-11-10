<script lang="ts">
  import { copyText } from './clipboard'
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { save } from '@tauri-apps/plugin-dialog'
  import { persist, send } from './agents.svelte'
  import { artifactUrl, docsOf, fileName, sharePrompt, type ArtifactInfo, type Doc } from './artifacts'
  import { refresh, shelf } from './artifacts.svelte'
  import { ask } from './asking.svelte'
  import { time } from './format'
  import { text } from './extensions'
  import { extensionItem, extensions, runItem } from './extensions.svelte'
  import { errorText, locale, t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'
  import { detailOf, labelOf, shown } from './layout-slots.svelte'
  import Markdown from './Markdown.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import Select from './Select.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let {
    agent,
    windowed = false,
    onpoint,
    onshare = (text) => send(agent.id, text),
    onclose,
  }: { agent: Agent; windowed?: boolean; onpoint: (text: string) => void; onshare?: (text: string) => void; onclose: () => void } = $props()

  let selected = $state<{ id: string; at: number } | null>(null)
  let copied = $state('')
  let asked = $state<string[]>([])
  let pointing = $state(false)
  let markdown = $state('')
  let frame = $state<HTMLIFrameElement>()
  let menu = $state(false)

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
    menu = false
  })

  const destinations = $derived(shown('artifact.actions'))
  const link = $derived(open?.link ?? null)
  const publishing = $derived(!!open && open.kind !== 'doc' && !open.link && asked.includes(open.id))
  const madeBy = (extension: string) => {
    const name = extensions.list.find((m) => m.id === extension)?.manifest?.name
    return name ? text(name, locale()) : extension
  }

  function pick(id: string) {
    menu = false
    const item = extensionItem('artifact.actions', id)
    if (id === 'claude' && open && open.kind !== 'doc') share(open)
    else if (item) runItem(item, { agent })
  }

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
    if (windowed) copy('point', '')
    onpoint(`${t('agent.artifactPointed', { title: open.title, page: page || open.entry, selector: String(selector) })}\n\n\`\`\`html\n${String(html)}\n\`\`\``)
  }

  function choose(id: string) {
    selected = { id, at: Date.now() }
  }

  function copy(what: string, text: string) {
    if (text) copyText(text)
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
    onshare(sharePrompt(artifact))
  }

  async function popOut() {
    const label = `artifacts-${agent.id}`
    const shown = await WebviewWindow.getByLabel(label)
    if (shown) shown.setFocus()
    else
      new WebviewWindow(label, {
        url: `index.html?${new URLSearchParams({ agent: agent.id, name: agent.name })}`,
        title: `${t('agent.artifactsTitle')} · ${agent.name}`,
        width: 1000,
        height: 820,
        minWidth: 480,
        minHeight: 360,
      })
    onclose()
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

<svelte:window onmessage={message} onclick={(e) => menu && !(e.target as Element).closest?.('[data-share]') && (menu = false)} onkeydown={(e) => e.key === 'Escape' && menu && (menu = false)} />

{#snippet shareMenu()}
  {#if open && destinations.length}
    <div class="anchor" data-share>
      <button
        class="btn btn-ghost share"
        class:open={menu}
        aria-haspopup="menu"
        aria-expanded={menu}
        title={t('agent.artifactShareMenuOf', { title: open.title })}
        onclick={() => (menu = !menu)}
      >
        {#if link}<span class="dot" aria-hidden="true"></span>{/if}{t('agent.artifactShareMenu')}
        <svg class="chevron" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
      </button>
      {#if menu}
        <div class="menu" role="menu" aria-label={t('agent.artifactShareMenu')}>
          {#each destinations as id, i (id)}
            {@const item = extensionItem('artifact.actions', id)}
            {#if i > 0 && (id === 'claude') !== (destinations[i - 1] === 'claude')}<span class="sep" aria-hidden="true"></span>{/if}
            {#if id === 'claude' && link}
              <div class="entry">
                <div class="option">
                  <span class="ico"><Icon name="artifact" /></span>
                  <span class="text"><b><span class="dot" aria-hidden="true"></span>{t('agent.artifactSharedOn')}</b><span>{t('agent.artifactOnClaude')}</span></span>
                </div>
                <div class="acts">
                  <button class="btn btn-primary" role="menuitem" onclick={() => ((menu = false), invoke('open_link', { url: link }))}>{t('agent.artifactOpenClaude')}</button>
                  <button class="btn btn-ghost" role="menuitem" onclick={() => copy('link', link)}>{copied === 'link' ? t('common.copied') : t('agent.artifactCopyLink')}</button>
                </div>
              </div>
            {:else if id === 'claude' || item}
              <button class="option" role="menuitem" disabled={id === 'claude' && (open.kind !== 'markdown' || publishing)} onclick={() => pick(id)}>
                <span class="ico"><Icon name={id === 'claude' ? 'artifact' : (item?.icon ?? 'plugins')} /></span>
                <span class="text">
                  <b>{labelOf('artifact.actions', id)}</b>
                  <span>{id === 'claude' && open.kind !== 'markdown' ? t('agent.artifactMarkdownOnly') : detailOf('artifact.actions', id)}</span>
                  {#if item}<span class="by">{madeBy(item.extension)}</span>{/if}
                </span>
              </button>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/if}
{/snippet}

<SidePanel kind="artifacts" title={t('agent.artifactsTitle')} {windowed} {onclose}>
  {#snippet sub()}
    {#if open?.kind === 'doc'}{open.title} · {t('agent.artifactOnClaude')}{:else if open}{open.title} · {t('agent.artifactModified', { time: time(open.modified) })}{:else}{t('agent.artifactsOf', { name: agent.name })}{/if}
  {/snippet}
  {#snippet actions()}
    {#if open?.kind === 'doc'}
      {@render shareMenu()}
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactHide')}</button>
    {:else if open}
      {#if open.kind === 'html'}
        <button class="btn btn-ghost" aria-pressed={pointing} title={t('agent.artifactPointTitle')} onclick={() => aim(!pointing)}>{pointing ? t('agent.artifactPointing') : copied === 'point' ? t('agent.artifactPointSent', { name: agent.name }) : t('agent.artifactPoint')}</button>
      {/if}
      <button class="btn btn-ghost" onclick={() => source(open)}>{copied === 'source' ? t('common.copied') : t('agent.artifactCopy')}</button>
      {@render shareMenu()}
      {#if open.kind === 'html'}
        <button class="btn btn-ghost" onclick={() => invoke('open_artifact_folder', { agent: agent.id, id: open.id }).catch((e) => ask(errorText(e), { kind: 'error' }))}>{t('agent.artifactFolder')}</button>
      {:else}
        <button class="btn btn-ghost" onclick={() => keep(open)}>{t('agent.artifactSave')}</button>
      {/if}
      <button class="btn btn-ghost danger" onclick={() => remove(open)}>{t('agent.artifactDelete')}</button>
    {/if}
    {#if !windowed}
      <button class="btn btn-ghost window" aria-label={t('conversation.openInWindow')} title={t('conversation.openInWindow')} onclick={popOut}>
        <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="7" width="14" height="14" rx="2" /><path d="M10 3h11v11" /></svg>
      </button>
    {/if}
  {/snippet}

  {#if list.length > 1}
    <div class="pick">
      <Select
        label={t('agent.artifactList')}
        value={open?.id ?? ''}
        options={list.map((a) => ({ value: a.id, label: a.title, detail: a.kind === 'doc' ? 'claude.ai' : `${a.kind} · ${t('agent.artifactModified', { time: time(a.modified) })}` }))}
        onpick={choose}
      />
    </div>
  {/if}

  {#if publishing}
    <div class="band" role="status">
      <StatusIcon status="background" />
      <span>{t('agent.artifactSharing', { name: agent.name })}</span>
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
  .pick {
    flex: none;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .pick :global(.select) {
    max-width: 420px;
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

  .band {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 8px 20px;
    border-bottom: 1px solid var(--background-line);
    background: var(--background-bg);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .anchor {
    position: relative;
    flex: none;
  }

  .share {
    gap: 7px;
  }

  .share.open {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .chevron {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.7;
  }

  .dot {
    display: inline-block;
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }

  .menu {
    position: absolute;
    z-index: 30;
    top: calc(100% + 6px);
    right: 0;
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 8px;
    border-radius: 4px;
    text-align: left;
  }

  button.option:hover:not(:disabled),
  button.option:focus-visible {
    background: var(--menu-hover);
    outline: none;
  }

  button.option:disabled {
    opacity: 0.5;
  }

  .ico {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 4px;
    background: var(--surface-3);
    color: var(--text-2);
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .text b {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .text b .dot {
    margin: 0 6px 1px 0;
    vertical-align: middle;
  }

  .text span {
    font-size: 12px;
    color: var(--text-dim);
  }

  .text .by {
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 8px 10px;
  }

  .entry .option {
    padding: 0;
  }

  .acts {
    display: flex;
    gap: 6px;
    margin-left: 42px;
  }

  .acts .btn {
    height: 28px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
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

  .window {
    width: 32px;
    padding: 0;
  }

  .window svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .danger:hover {
    color: var(--err);
  }
</style>
