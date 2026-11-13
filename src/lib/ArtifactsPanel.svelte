<script lang="ts">
  import { copyText } from './clipboard'
  import { invoke } from '@tauri-apps/api/core'
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
  import { save } from '@tauri-apps/plugin-dialog'
  import { persist, send } from './agents.svelte'
  import { artifactUrl, docsOf, fileName, fillShare, sharePrompt, type ArtifactInfo, type Doc } from './artifacts'
  import { answer, bands, finish, refresh, setWork, shelf, workKey, type Work } from './artifacts.svelte'
  import { cleanupErrors, dueAt, extensionName, keepLabel, release, ruleOf } from './cleanup.svelte'
  import { ask, message as tell } from './asking.svelte'
  import { time } from './format'
  import { text, type Item, type Text } from './extensions'
  import { allowed, extensionItem, extensions, runItem, shareTool } from './extensions.svelte'
  import { errorText, locale, t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'
  import { detailOf, labelOf, shown } from './layout-slots.svelte'
  import Markdown from './Markdown.svelte'
  import Select from './Select.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let {
    agent,
    windowed = false,
    onpoint,
    onshare = (text, hint) => send(agent.id, text, [], [], '', hint),
    onanswer = () => answer(agent),
    onclose,
  }: { agent: Agent; windowed?: boolean; onpoint: (text: string) => void; onshare?: (text: string, hint?: string) => void; onanswer?: () => void; onclose: () => void } = $props()

  let selected = $state<{ id: string; at: number } | null>(null)
  let copied = $state('')
  let pointing = $state(false)
  let markdown = $state('')
  let frame = $state<HTMLIFrameElement>()
  let menu = $state(false)
  let more = $state(false)

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

  const openId = $derived(open?.id)

  $effect(() => {
    void openId
    pointing = false
    menu = false
    more = false
  })

  const destinations = $derived(shown('artifact.actions'))
  const link = $derived(open?.link ?? null)
  const local_ = $derived(open && open.kind !== 'doc' ? open : undefined)
  const band = $derived(local_ ? bands[workKey(agent.id, local_.id)] : undefined)
  const busy = $derived(band?.state === 'work' || band?.state === 'wait')
  const steps = $derived(band?.progress ? Array.from({ length: band.progress.total }, (_, i) => (i < band.progress!.done ? 'on' : i === band.progress!.done ? 'now' : '')) : band?.state === 'work' ? ['now'] : [])
  const bandLink = $derived(band && local_?.links?.[band.destination]?.link)
  const shared = $derived(!!link || !!Object.keys(local_?.links ?? {}).length)
  const rule = $derived(local_ && ruleOf(local_))
  const due = $derived(local_ ? dueAt(local_) : null)
  const left = $derived(due === null ? null : Math.max(0, Math.ceil((due - Date.now()) / 86_400_000)))
  const KEEPS: (number | null)[] = [null, 30, 365, 0]

  async function setKeep(keep: number | null) {
    if (!local_) return
    await invoke('set_artifact_keep', { agent: agent.id, id: local_.id, keep }).catch((e) => tell(errorText(e), { kind: 'error' }))
    await refresh(agent.id)
  }
  const shareOf = (item?: Item) => (item && 'artifact.share' in item.action ? item.action['artifact.share'] : undefined)
  const tr = (value?: Text) => (value ? text(value, locale()) : '')
  const madeBy = (extension: string) => {
    const name = extensions.list.find((m) => m.id === extension)?.manifest?.name
    return name ? text(name, locale()) : extension
  }

  function pick(id: string) {
    menu = false
    const item = extensionItem('artifact.actions', id)
    if (id === 'claude' && open && open.kind !== 'doc') share(open)
    else if (item && shareOf(item)) step(item, 'share')
    else if (item) runItem(item, { agent })
  }

  const valuesOf = (artifact: ArtifactInfo, destination: string, data: string, remove = false) => ({
    'artifact.id': artifact.id,
    'artifact.title': artifact.title,
    'artifact.dir': `${shelf.root}/${agent.id}/${artifact.id}`,
    'artifact.entry': artifact.entry,
    agent: agent.name,
    'agent.id': agent.id,
    destination,
    'extension.data': data,
    link: artifact.links?.[destination]?.link ?? '',
    delete: String(remove),
  })

  async function step(item: Item, verb: Work['verb']) {
    const artifact = local_
    const action = shareOf(item)
    const how = action?.[verb]
    if (!artifact || !action || !how) return
    menu = false
    if (!(await allowed(item.extension))) return
    const label = labelOf('artifact.actions', item.key)
    const data = await invoke<string>('extension_data_dir', { id: item.extension }).catch(() => '')
    let remove = false
    if (verb === 'share' && !(await ask(fillShare(tr(action.confirm), valuesOf(artifact, item.key, data)), { title: label, okLabel: label, lines: action.lines }))) return
    const check = action.delete ? { label: tr(action.delete), hint: tr(action.deleteHint) || undefined, onchange: (on: boolean) => (remove = on) } : undefined
    if (verb === 'unshare' && !(await ask(t('agent.artifactStopAsk'), { title: t('agent.artifactStopTitle'), kind: 'warning', okLabel: t('agent.artifactStop'), danger: true, check }))) return
    const key = workKey(agent.id, artifact.id)
    const values = valuesOf(artifact, item.key, data, remove)
    if (typeof how === 'string') {
      setWork(key, { destination: item.key, verb, by: agent.name, since: Date.now(), extension: item.extension })
      onshare(label, fillShare(how, values))
      return
    }
    setWork(key, { destination: item.key, verb, by: madeBy(item.extension), since: Date.now() })
    try {
      const input = { verb, destination: item.key, artifact: { id: artifact.id, title: artifact.title, dir: values['artifact.dir'], entry: artifact.entry }, link: values.link, delete: remove, data }
      const out = (await shareTool(item.extension, how.tool, input, agent)).trim()
      if (verb !== 'unshare' && !/^https:\/\/\S+$/.test(out)) throw new Error(t('agent.artifactNoLink', { tool: how.tool }))
      await invoke('set_artifact_share', { agent: agent.id, id: artifact.id, destination: item.key, link: verb === 'unshare' ? null : out })
      await refresh(agent.id)
      finish(key)
    } catch (e) {
      setWork(key, null)
      tell(errorText(e), { kind: 'error', title: label })
    }
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
    if (!(await ask(t('agent.artifactShareAsk', { title: artifact.title, name: agent.name }), { title: t('agent.artifactShareTitle'), okLabel: t('agent.artifactShare') }))) return
    setWork(workKey(agent.id, artifact.id), { destination: 'claude', verb: 'share', by: agent.name, since: Date.now() })
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
      const sharing = !!Object.keys(item.links ?? {}).length
      if (!(await ask(t(sharing ? 'agent.artifactDeleteShared' : 'agent.artifactDeleteAsk', { title: item.title }), { title: t('agent.artifactDeleteTitle'), kind: 'warning', okLabel: t('agent.artifactDelete'), danger: sharing }))) return
      try {
        await release(agent.id, item)
      } catch (e) {
        await refresh(agent.id)
        if (!(await ask(t('agent.artifactDeleteAnyway', { error: errorText(e) }), { title: t('agent.artifactDeleteTitle'), kind: 'error', okLabel: t('agent.artifactDelete'), danger: true }))) return
      }
      await invoke('delete_artifact', { agent: agent.id, id: item.id }).catch((e) => ask(errorText(e), { kind: 'error' }))
      await refresh(agent.id)
    }
    if (list.length <= 1) onclose()
  }
</script>

<svelte:window
  onmessage={message}
  onclick={(e) => {
    const at = e.target as Element
    if (menu && !at.closest?.('[data-share]')) menu = false
    if (more && !at.closest?.('[data-more]')) more = false
  }}
  onkeydown={(e) => e.key === 'Escape' && ((menu = false), (more = false))}
/>

{#snippet shareMenu()}
  {#if open && destinations.length}
    <div class="anchor" data-share>
      <button
        class="btn btn-ghost share"
        class:open={menu}
        aria-haspopup="menu"
        aria-expanded={menu}
        title={t('agent.artifactShareMenuOf', { title: open.title })}
        onclick={() => ((menu = !menu), (more = false))}
      >
        {#if shared}<span class="dot" aria-hidden="true"></span>{/if}{t('agent.artifactShareMenu')}
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
            {:else if item && shareOf(item) && local_?.links?.[id]}
              {@const action = shareOf(item)!}
              {@const on = local_.links[id]}
              {@const stale = local_.modified > on.at}
              <div class="entry">
                <div class="option">
                  <span class="ico"><Icon name={item.icon ?? 'plugins'} /></span>
                  <span class="text">
                    <b><span class="dot" aria-hidden="true"></span>{tr(action.shared) || labelOf('artifact.actions', id)}</b>
                    <span>{stale ? t('agent.artifactStale', { time: time(on.at) }) : t('agent.artifactSince', { time: time(on.at) })}</span>
                    <span class="by">{madeBy(item.extension)}</span>
                  </span>
                </div>
                <div class="acts">
                  {#if stale && action.update}<button class="btn btn-primary" role="menuitem" disabled={!!busy} onclick={() => step(item, 'update')}>{t('agent.artifactUpdate')}</button>{/if}
                  <button class={stale && action.update ? 'btn btn-ghost' : 'btn btn-primary'} role="menuitem" onclick={() => ((menu = false), invoke('open_link', { url: on.link }))}>{t('agent.artifactOpen')}</button>
                  <button class="btn btn-ghost" role="menuitem" onclick={() => copy(id, on.link)}>{copied === id ? t('common.copied') : t('agent.artifactCopyLink')}</button>
                  {#if action.unshare}<span class="gap"></span><button class="btn btn-ghost danger" role="menuitem" disabled={!!busy} onclick={() => step(item, 'unshare')}>{t('agent.artifactStop')}</button>{/if}
                </div>
              </div>
            {:else if id === 'claude' || item}
              <button class="option" role="menuitem" disabled={(id === 'claude' && open.kind !== 'markdown') || (!!busy && (id === 'claude' || !!shareOf(item)))} onclick={() => pick(id)}>
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
      {@render shareMenu()}
      <div class="anchor" data-more>
        <button
          class="btn btn-ghost square"
          class:open={more}
          aria-label={t('agent.artifactMore')}
          title={t('agent.artifactMore')}
          aria-haspopup="menu"
          aria-expanded={more}
          onclick={() => ((more = !more), (menu = false))}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor"><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg>
        </button>
        {#if more}
          <div class="menu narrow" role="menu" aria-label={t('agent.artifactMore')}>
            <button class="option" role="menuitem" onclick={() => source(open)}>
              <span class="ico"><Icon name="copy" /></span>
              <span class="text"><b>{copied === 'source' ? t('common.copied') : t('agent.artifactCopy')}</b><span>{t('agent.artifactCopyHint')}</span></span>
            </button>
            {#if open.kind === 'html'}
              <button class="option" role="menuitem" onclick={() => ((more = false), invoke('open_artifact_folder', { agent: agent.id, id: open.id }).catch((e) => ask(errorText(e), { kind: 'error' })))}>
                <span class="ico"><Icon name="folder" /></span>
                <span class="text"><b>{t('agent.artifactFolder')}</b><span>{t('agent.artifactFolderHint')}</span></span>
              </button>
            {:else}
              <button class="option" role="menuitem" onclick={() => ((more = false), keep(open))}>
                <span class="ico"><Icon name="update" /></span>
                <span class="text"><b>{t('agent.artifactSave')}</b><span>{t('agent.artifactSaveHint')}</span></span>
              </button>
            {/if}
            <span class="sep" aria-hidden="true"></span>
            {#if rule}
              {@const error = cleanupErrors[workKey(agent.id, open.id)]}
              <div class="entry">
                <div class="option">
                  <span class="ico"><Icon name="clock" /></span>
                  <span class="text">
                    <b>{left === null ? t('agent.artifactKeptForever') : left === 0 ? t('agent.artifactDeletedSoon') : t('agent.artifactDeletedIn', { count: left })}</b>
                    <span>{rule.from === 'artifact' ? t('agent.artifactRuleOwn') : rule.from === 'extension' ? t('agent.artifactRuleExtension', { name: extensionName(rule.extension ?? '') }) : t('agent.artifactRuleSquadron')}</span>
                    {#if error}<span class="failed">{error}</span>{/if}
                  </span>
                </div>
                <div class="segments" role="group" aria-label={t('agent.artifactKeepRule')}>
                  {#each KEEPS as keep (keep)}
                    <button aria-pressed={(open.keep ?? null) === keep} onclick={() => setKeep(keep)}>{keep === null ? t('agent.artifactKeepRule') : keepLabel(keep)}</button>
                  {/each}
                </div>
              </div>
              <span class="sep" aria-hidden="true"></span>
            {/if}
            <button class="option delete" role="menuitem" onclick={() => ((more = false), remove(open))}>
              <span class="ico"><Icon name="trash" /></span>
              <span class="text"><b>{t('agent.artifactDelete')}</b><span>{Object.keys(open.links ?? {}).length ? t('agent.artifactDeleteSharedHint') : t('agent.artifactDeleteHint')}</span></span>
            </button>
          </div>
        {/if}
      </div>
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

  {#if band}
    {@const title = labelOf('artifact.actions', band.destination)}
    {@const p = band.progress}
    {@const count = p ? `${Math.min(p.done + 1, p.total)}/${p.total}` : ''}
    {@const label =
      band.state === 'wait'
        ? t('agent.artifactAsksHint')
        : band.state === 'done'
          ? (bandLink ?? '')
          : band.state === 'err'
            ? band.reason?.split('\n').find((l) => l.trim()) || t(band.extension && !band.reason ? 'agent.artifactNotStarted' : 'agent.artifactStoppedHint')
            : p?.message || (band.queued ? t('agent.artifactQueued') : band.destination === 'claude' ? t('agent.artifactSharing', { name: agent.name }) : t(band.verb === 'share' ? 'agent.artifactBusyShare' : band.verb === 'update' ? 'agent.artifactBusyUpdate' : 'agent.artifactBusyUnshare', { who: band.by }))}
    <div class="band {band.state}" role="status">
      {#if band.state === 'done'}
        <span class="badge" aria-hidden="true"><svg width="12" height="12" viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7" /></svg></span>
      {:else if band.state === 'err'}
        <span class="badge" aria-hidden="true"><svg width="11" height="11" viewBox="0 0 24 24"><path d="M5 5l14 14M19 5 5 19" /></svg></span>
      {:else}
        <span class="orb" aria-hidden="true"></span>
      {/if}
      <span class="words">
        <span>
          {#if band.state === 'wait'}<b>{t('agent.artifactAsks', { name: band.asker ?? title })}</b>
          {:else if band.state === 'done'}<b>{(band.verb === 'share' && tr(shareOf(extensionItem('artifact.actions', band.destination))?.shared)) || t(band.verb === 'share' ? 'agent.artifactDoneShare' : band.verb === 'update' ? 'agent.artifactDoneUpdate' : 'agent.artifactDoneUnshare', { what: title })}</b>
          {:else if band.state === 'err'}<b>{title}</b> {count ? t('agent.artifactStoppedAt', { step: count }) : t('agent.artifactStopped')}
          {:else}<b>{title}</b>{/if}
          {#if count && band.state !== 'err' && band.state !== 'done'} · <span class="count">{count}</span>{/if}
        </span>
        {#key label}<span class="label" class:mono={band.state === 'done'}><span>{label}</span></span>{/key}
      </span>
      {#if band.state === 'wait'}
        <button class="btn answer" onclick={onanswer}>{t('agent.artifactAnswer')}</button>
      {:else if band.state === 'done' && bandLink}
        <button class="btn btn-ghost" onclick={() => invoke('open_link', { url: bandLink })}>{t('agent.artifactOpen')}</button>
        <button class="btn btn-ghost" onclick={() => copy('band', bandLink)}>{copied === 'band' ? t('common.copied') : t('agent.artifactCopyLink')}</button>
      {:else if band.state === 'err'}
        <button class="btn close" onclick={() => setWork(workKey(agent.id, local_!.id), null)}>{t('common.close')}</button>
      {/if}
      {#if steps.length || band.state === 'done'}
        <span class="segs" aria-hidden="true">{#each band.state === 'done' ? ['on'] : steps as cell, c (c)}<i class={cell}></i>{/each}</span>
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
    --tone: var(--background);
    position: relative;
    flex: none;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 52px;
    padding: 0 12px 4px 18px;
    border-bottom: 1px solid var(--background-line);
    background: var(--background-bg);
    font-size: 12.5px;
    color: var(--text-2);
    transition:
      background 0.4s,
      border-color 0.4s;
  }

  .band::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(100deg, transparent 30%, color-mix(in srgb, var(--tone) 14%, transparent) 50%, transparent 70%) 0 0 / 220% 100%;
    animation: sweep 2.6s linear infinite;
    pointer-events: none;
  }

  .band.wait {
    --tone: var(--accent);
    border-color: var(--wait-border);
    background: var(--wait-bg);
    color: var(--wait-text-2);
  }

  .band.wait::before {
    animation-duration: 3.6s;
  }

  .band.done {
    --tone: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 35%, transparent);
    background: color-mix(in srgb, var(--ok) 9%, var(--bg-sidebar));
  }

  .band.done::before {
    animation: sweep 1.2s ease-out 1 forwards;
  }

  .band.err {
    --tone: var(--err);
    border-color: var(--err-border);
    background: var(--err-bg);
    color: var(--err-muted);
  }

  .band.err::before {
    display: none;
  }

  .orb {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
  }

  .orb::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: conic-gradient(from 0deg, transparent 0 55%, var(--tone));
    mask: radial-gradient(circle, transparent 7.5px, black 8px);
    animation: spin 1.1s linear infinite;
  }

  .orb::after {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--tone);
    box-shadow: 0 0 10px 1px var(--tone);
  }

  .wait .orb::before {
    opacity: 0.5;
    background: var(--tone);
    mask: none;
    animation: ping 1.6s ease-out infinite;
  }

  .wait .orb::after {
    width: 9px;
    height: 9px;
  }

  .badge {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--tone);
    color: var(--ink);
    animation: pop 0.5s cubic-bezier(0.3, 1.6, 0.5, 1) both;
  }

  .err .badge {
    color: var(--err-ink);
    animation: none;
  }

  .badge svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 3.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .words {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .wait .words b {
    color: var(--wait-text);
  }

  .label {
    position: relative;
    display: block;
    height: 18px;
    overflow: hidden;
  }

  .label span {
    position: absolute;
    inset: 0 0 auto;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    animation: rise 0.45s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }

  .label.mono {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .count {
    font: 600 11.5px var(--mono);
    color: var(--tone);
  }

  .wait .count {
    color: var(--accent-soft);
  }

  .segs {
    position: absolute;
    inset: auto 0 0;
    display: flex;
    gap: 3px;
    height: 3px;
  }

  .segs i {
    position: relative;
    flex: 1;
    overflow: hidden;
    background: color-mix(in srgb, var(--tone) 16%, transparent);
    transition: background 0.4s;
  }

  .segs i.on {
    background: var(--tone);
  }

  .segs i.now {
    background: color-mix(in srgb, var(--tone) 30%, transparent);
    box-shadow: 0 0 8px color-mix(in srgb, var(--tone) 70%, transparent);
  }

  .segs i.now::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, var(--tone), color-mix(in srgb, var(--tone) 40%, white), var(--tone), transparent) 0 0 / 50% 100% no-repeat;
    animation: run 1.3s ease-in-out infinite;
  }

  .wait .segs i.now::after {
    background: var(--tone);
    animation: pulse 1.6s ease-in-out infinite;
  }

  .band .answer {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
    animation: ring 2.4s ease-in-out infinite;
  }

  .band .close {
    background: #3a2420;
    color: #f8d5cc;
    font-weight: 700;
  }

  @keyframes sweep {
    from {
      background-position: 160% 0;
    }
    to {
      background-position: -60% 0;
    }
  }

  @keyframes run {
    from {
      background-position: -100% 0;
    }
    to {
      background-position: 200% 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      filter: blur(2px);
      transform: translateY(70%);
    }
    to {
      opacity: 1;
      filter: none;
      transform: none;
    }
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.3);
    }
    to {
      opacity: 1;
      transform: none;
    }
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

  .square {
    width: 32px;
    padding: 0;
  }

  .square.open {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .menu.narrow {
    width: 300px;
  }

  .option.delete .text b {
    color: var(--err-text);
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

  .acts .gap {
    flex: 1;
  }

  .band .btn {
    position: relative;
    flex: none;
    height: 28px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .band b {
    font-weight: 600;
    color: var(--text-strong);
  }

  .acts .btn {
    height: 28px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .segments {
    display: flex;
    align-self: flex-start;
    gap: 2px;
    margin-left: 42px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--text-2);
    font-size: 12px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .text .failed {
    color: var(--err-text);
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
