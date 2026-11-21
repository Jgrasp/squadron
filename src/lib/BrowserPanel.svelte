<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { untrack } from 'svelte'
  import { browser, type BrowserTab } from './browser.svelte'
  import { ask } from './asking.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import BrowserNotes, { type Note } from './BrowserNotes.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent, Comment } from './stream'

  let {
    agent,
    onnote,
    onclose,
  }: { agent: Agent; onnote: (comments: Comment[], files: string[]) => void; onclose: () => void } = $props()

  const tabs = $derived(browser.tabs.filter((x) => x.agent === agent.id))
  let chosen = $state('')
  const tab = $derived<BrowserTab | undefined>(tabs.find((x) => x.id === chosen) ?? tabs.at(-1))

  const seen = new Set<string>()
  $effect(() => {
    const fresh = tabs.filter((x) => !seen.has(x.id))
    for (const x of fresh) seen.add(x.id)
    if (fresh.length) chosen = fresh.at(-1)!.id
  })

  let address = $state('')
  let editing = $state(false)
  let field = $state<HTMLInputElement>()
  $effect(() => {
    const url = tab?.url ?? ''
    if (!untrack(() => editing)) address = url === 'about:blank' ? '' : url
  })

  let screen = $state<HTMLElement>()
  let canvas = $state<HTMLCanvasElement>()
  let frozen = $state(false)
  let annotating = $state(false)
  let notes = $state<Note[]>([])
  const where = $derived(`${tab?.id} ${tab?.url}`)
  $effect(() => {
    void where
    annotating = false
    notes = []
  })

  const COVERS = 'dialog[open], [role=menu], [role=dialog], [role=listbox], [role=alertdialog], [role=tooltip]'

  function covered(b: DOMRect) {
    for (const e of document.querySelectorAll(COVERS)) {
      const r = e.getBoundingClientRect()
      if (r.width && r.height && r.left < b.right && r.right > b.left && r.top < b.bottom && r.bottom > b.top) return true
    }
    return false
  }

  async function freeze(id: string) {
    const data = await invoke<ArrayBuffer | number[]>('browser_picture', { tab: id })
    const bytes = data instanceof ArrayBuffer ? data : new Uint8Array(data)
    const image = await createImageBitmap(new Blob([bytes], { type: 'image/jpeg' }))
    if (canvas) {
      ;[canvas.width, canvas.height] = [image.width, image.height]
      canvas.getContext('2d')?.drawImage(image, 0, 0)
    }
    image.close()
  }

  const viewed = $derived(tab?.id)
  const painted = () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)))

  $effect(() => {
    const id = viewed
    if (!id || !screen) return
    let frame = 0
    let placed = ''
    let busy = false
    const tick = async () => {
      frame = requestAnimationFrame(tick)
      if (busy || !screen) return
      const b = screen.getBoundingClientRect()
      const hide = annotating || !b.width || !b.height || covered(b)
      const key = hide ? 'hidden' : [b.x, b.y, b.width, b.height].map(Math.round).join()
      if (key === placed) return
      busy = true
      placed = key
      if (hide && !frozen) {
        frozen = await freeze(id).then(
          () => true,
          (e) => (fail(e), (annotating = false), false),
        )
        await painted()
      }
      await invoke('browser_place', { tab: id, rect: hide ? null : [b.x, b.y, b.width, b.height] }).catch(fail)
      if (!hide && frozen) {
        await painted()
        frozen = false
      }
      busy = false
    }
    frame = requestAnimationFrame(tick)
    return () => {
      cancelAnimationFrame(frame)
      frozen = false
      invoke('browser_place', { tab: id, rect: null }).catch(() => {})
    }
  })

  function fail(e: unknown) {
    ask(errorText(e), { kind: 'error' })
  }

  async function go(event: SubmitEvent) {
    event.preventDefault()
    const url = address.trim()
    if (!url) return
    editing = false
    field?.blur()
    if (tab) await invoke('browser_navigate', { tab: tab.id, url }).catch(fail)
    else chosen = await invoke<string>('browser_open', { agent: agent.id, url }).catch((e) => (fail(e), ''))
  }

  async function blank() {
    chosen = await invoke<string>('browser_open', { agent: agent.id, url: 'about:blank' }).catch((e) => (fail(e), ''))
    field?.focus()
  }

  let answer = $state('')
  $effect(() => {
    answer = tab?.dialog?.defaultPrompt ?? ''
  })

  function reply(accept: boolean) {
    if (tab) invoke('browser_dialog', { tab: tab.id, accept, text: tab.dialog?.type === 'prompt' ? answer : null }).catch(fail)
  }

  const heading = (x: BrowserTab) => x.title || (x.url === 'about:blank' ? t('browser.newTab') : x.url)
</script>

<SidePanel kind="browser" title={t('browser.title')} {onclose}>
  {#snippet sub()}
    {tab ? heading(tab) : t('browser.of', { name: agent.name })}
  {/snippet}
  {#snippet actions()}
    <span class="alpha" title={t('browser.alphaHint')}>{t('browser.alpha')}</span>
  {/snippet}

  <div class="tabs" role="tablist" aria-label={t('browser.tabs')}>
    {#each tabs as x (x.id)}
      <span class="tab" class:on={x === tab}>
        <button role="tab" aria-selected={x === tab} title={x.url} onclick={() => (chosen = x.id)}>
          {#if x.loading}<span class="spin" aria-hidden="true"></span>{/if}
          <span class="name">{heading(x)}</span>
        </button>
        <button class="x" aria-label={t('browser.closeTab', { title: heading(x) })} onclick={() => invoke('browser_close', { tab: x.id }).catch(fail)}>×</button>
      </span>
    {/each}
    <button class="new" aria-label={t('browser.newTab')} title={t('browser.newTab')} onclick={blank}>+</button>
  </div>

  <form class="bar" onsubmit={go}>
    <button type="button" class="nav" aria-label={t('browser.back')} title={t('browser.back')} disabled={!tab} onclick={() => tab && invoke('browser_history', { tab: tab.id, delta: -1 }).catch(fail)}>
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M15 18l-6-6 6-6" /></svg>
    </button>
    <button type="button" class="nav" aria-label={t('browser.forward')} title={t('browser.forward')} disabled={!tab} onclick={() => tab && invoke('browser_history', { tab: tab.id, delta: 1 }).catch(fail)}>
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 18l6-6-6-6" /></svg>
    </button>
    <button type="button" class="nav" aria-label={t('browser.reload')} title={t('browser.reload')} disabled={!tab} onclick={() => tab && invoke('browser_reload', { tab: tab.id }).catch(fail)}>
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 11a8 8 0 1 0-2.3 5.7M20 4v7h-7" /></svg>
    </button>
    <button type="button" class="nav" aria-label={t('browser.annotate')} aria-pressed={annotating} title={t('browser.annotateTitle', { name: agent.name })} disabled={!tab} onclick={() => (annotating = !annotating)}>
      <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20h9M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" /></svg>
    </button>
    <input
      class="field mono"
      bind:this={field}
      bind:value={address}
      placeholder={t('browser.address')}
      aria-label={t('browser.address')}
      spellcheck="false"
      autocomplete="off"
      onfocus={() => ((editing = true), field?.select())}
      onblur={() => (editing = false)}
    />
  </form>
  <div class="progress" class:on={tab?.loading} aria-hidden="true"></div>

  {#if tab?.dialog}
    <div class="dialog" role="alertdialog" aria-label={tab.dialog.message}>
      <span class="message">
        <strong>{tab.dialog.type === 'beforeunload' ? t('browser.dialogLeave') : t(tab.dialog.type === 'alert' ? 'browser.dialogAlert' : 'browser.dialogConfirm')}</strong>
        {tab.dialog.message}
      </span>
      {#if tab.dialog.type === 'prompt'}
        <input class="field" bind:value={answer} onkeydown={(e) => e.key === 'Enter' && reply(true)} />
      {/if}
      {#if tab.dialog.type !== 'alert'}
        <button class="btn btn-ghost" onclick={() => reply(false)}>{t('common.cancel')}</button>
      {/if}
      <button class="btn btn-primary" onclick={() => reply(true)}>{t('common.ok')}</button>
    </div>
  {/if}

  <div class="screen" bind:this={screen}>
    {#if tab}
      <canvas bind:this={canvas} class:frozen aria-hidden="true"></canvas>
      {#if annotating && frozen && canvas}
        <BrowserNotes
          {tab}
          {canvas}
          bind:notes
          onfail={fail}
          onclose={() => (annotating = false)}
          onsend={(comments, image) => (onnote(comments, image ? [image] : []), (annotating = false))}
        />
      {/if}
    {:else}
      <p class="empty">{t('browser.empty', { name: agent.name })}</p>
    {/if}
  </div>
</SidePanel>

<style>
  .alpha {
    padding: 2px 7px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--wait-border);
    color: var(--accent-soft);
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: help;
  }

  .tabs {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    flex: none;
    max-width: 220px;
    display: flex;
    align-items: center;
    border-radius: 4px;
    color: var(--text-2);
  }

  .tab:hover {
    background: var(--surface-2);
  }

  .tab.on {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .tab [role='tab'] {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 4px 5px 10px;
    font-size: 12.5px;
  }

  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .x,
  .new {
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: 3px;
    color: var(--text-dim);
  }

  .x:hover,
  .new:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .spin {
    width: 10px;
    height: 10px;
    flex: none;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 12px;
  }

  .nav {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-2);
  }

  .nav:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text-strong);
  }

  .nav[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--accent);
  }

  .nav:disabled {
    opacity: 0.4;
  }

  .nav svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .bar .field {
    flex: 1;
    min-width: 0;
    margin-left: 4px;
    font-size: 12.5px;
  }

  .progress {
    flex: none;
    height: 2px;
  }

  .progress.on {
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    background-size: 50% 100%;
    background-repeat: no-repeat;
    animation: sweep 1.1s ease-in-out infinite;
  }

  @keyframes sweep {
    from {
      background-position: -50% 0;
    }
    to {
      background-position: 150% 0;
    }
  }

  .dialog {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px 10px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
    font-size: 13px;
  }

  .message {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .message strong {
    margin-right: 6px;
  }

  .dialog .field {
    width: 200px;
  }

  .screen {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-main);
  }

  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    visibility: hidden;
    transition: translate 0.18s ease;
  }

  canvas.frozen {
    visibility: visible;
  }

  .empty {
    max-width: 360px;
    margin: 64px auto;
    padding: 0 24px;
    text-align: center;
    font-size: 13px;
    color: var(--text-muted);
  }

  @media (prefers-reduced-motion: reduce) {
    canvas {
      transition: none;
    }
  }
</style>
