<script lang="ts">
  import { locale, t } from './i18n/index.svelte'
  import { PAGE_THEME, text, themeOf, type Panel } from './extensions'
  import { extensions, onPanelPost, panelMessage, setOption } from './extensions.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let { panel, agent, onclose }: { panel: Panel; agent: Agent; onclose: () => void } = $props()

  let frame = $state<HTMLIFrameElement>()
  let loaded = $state(false)

  const owner = $derived(extensions.list.find((m) => m.id === panel.extension)?.manifest?.name)

  const base = navigator.userAgent.includes('Windows') ? 'http://extension.localhost' : 'extension://localhost'

  function theme() {
    const style = getComputedStyle(document.documentElement)
    frame?.contentWindow?.postMessage({ squadron: 'theme', vars: Object.fromEntries(PAGE_THEME.map((name) => [name, style.getPropertyValue(`--${name}`).trim()])) }, '*')
  }

  $effect(() => {
    void panel.key
    loaded = false
  })

  $effect(() => {
    themeOf(extensions.list)
    theme()
  })

  const options = () => frame?.contentWindow?.postMessage({ squadron: 'options', values: $state.snapshot(extensions.options[panel.extension] ?? {}) }, '*')

  $effect(() => {
    void extensions.options[panel.extension]
    options()
  })

  $effect(() => onPanelPost(panel.extension, panel.id, (message) => frame?.contentWindow?.postMessage({ squadron: 'extension', message }, '*')))

  function message(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow) return
    if (e.data?.squadron === 'extension') panelMessage(panel, agent, e.data.message)
    if (e.data?.squadron === 'option') setOption(panel.extension, String(e.data.id), e.data.value ?? null).catch((err) => console.warn(err))
  }
</script>

<svelte:window onmessage={message} />

<SidePanel kind={panel.key} title={text(panel.title, locale())} {onclose}>
  {#snippet sub()}{t('extensions.panelOf', { name: owner ? text(owner, locale()) : panel.extension })}{/snippet}
  {#key panel.key}
    <iframe bind:this={frame} title={text(panel.title, locale())} src="{base}/{panel.extension}/{panel.page}" sandbox="allow-scripts" referrerpolicy="no-referrer" class:loaded onload={() => (theme(), options(), (loaded = true))}></iframe>
  {/key}
</SidePanel>

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
