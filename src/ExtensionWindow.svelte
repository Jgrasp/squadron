<script lang="ts">
  import { emitTo } from '@tauri-apps/api/event'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { onMount } from 'svelte'
  import SidePanel from './lib/SidePanel.svelte'
  import type { Gesture, WindowState } from './lib/extensionWindows.svelte'
  import { t } from './lib/i18n/index.svelte'

  const me = getCurrentWindow()
  const query = new URLSearchParams(location.search)
  const [extension, id] = [query.get('extension') ?? '', query.get('panel') ?? '']
  const base = navigator.userAgent.includes('Windows') ? 'http://extension.localhost' : 'extension://localhost'

  let shown = $state.raw<WindowState | null>(null)
  let frame = $state<HTMLIFrameElement>()
  let loaded = $state(false)
  let hovered = false
  let focused = true

  const gesture = (): Gesture => ({ focused: focused && document.hasFocus() && document.activeElement === frame, hovered, visible: document.visibilityState === 'visible' })
  const tell = (kind: string, more: Record<string, unknown> = {}) => emitTo('main', 'extension-window', { label: me.label, kind, gesture: gesture(), ...more }).catch(() => {})
  const later = () => setTimeout(() => tell('state'))
  const post = (data: unknown) => frame?.contentWindow?.postMessage(data, '*')

  function feed() {
    if (!shown) return
    for (const [name, value] of Object.entries(shown.theme)) document.documentElement.style.setProperty(`--${name}`, value)
    post({ squadron: 'theme', vars: shown.theme })
    post({ squadron: 'options', values: shown.options })
  }

  function message(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow) return
    if (e.data?.squadron === 'extension') tell('message', { message: e.data.message ?? null })
    if (e.data?.squadron === 'option') tell('option', { id: String(e.data.id), value: e.data.value ?? null })
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'F11') me.isFullscreen().then((on) => me.setFullscreen(!on))
  }

  onMount(() => {
    const off = [
      me.listen<WindowState>('extension-window-state', ({ payload }) => {
        if (payload.extension !== extension || payload.panel !== id) return
        shown = payload
        feed()
      }),
      me.listen<{ extension: string; panel: string; message: unknown }>('extension-window-post', ({ payload }) => {
        if (payload.extension === extension && payload.panel === id) post({ squadron: 'extension', message: payload.message })
      }),
      me.listen('extension-window-ask', () => tell('ready')),
      me.onFocusChanged(({ payload }) => {
        focused = payload
        later()
      }),
      me.onCloseRequested(async () => {
        await tell('closed')
      }),
    ]
    Promise.all(off).then(() => tell('ready'))
    return () => off.forEach((p) => p.then((stop) => stop()))
  })

  $effect(() => {
    if (!frame) return
    const element = frame
    const hover = (on: boolean) => () => {
      hovered = on
      tell('state')
    }
    const [enter, leave] = [hover(true), hover(false)]
    element.addEventListener('mouseenter', enter)
    element.addEventListener('mouseleave', leave)
    return () => {
      element.removeEventListener('mouseenter', enter)
      element.removeEventListener('mouseleave', leave)
    }
  })
</script>

<svelte:window onmessage={message} onkeydown={key} onblur={later} onfocus={later} />
<svelte:document onvisibilitychange={later} />

{#if shown}
  <SidePanel kind="extension:{extension}.{id}" title={shown.title} windowed onclose={() => me.close()}>
    {#snippet sub()}{t('extensions.panelOf', { name: shown!.owner })}{/snippet}
    <iframe bind:this={frame} title={shown.title} src="{base}/{extension}/{shown.page}" sandbox="allow-scripts" referrerpolicy="no-referrer" class:loaded onload={() => (feed(), (loaded = true))}></iframe>
  </SidePanel>
{/if}

<style>
  iframe {
    flex: 1;
    width: 100%;
    border: 0;
    background: var(--bg-main);
    visibility: hidden;
  }

  iframe.loaded {
    visibility: visible;
  }
</style>
