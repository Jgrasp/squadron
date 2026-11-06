<script lang="ts">
  import { locale, t } from './i18n/index.svelte'
  import { text, themeOf, type Panel } from './mods'
  import { mods, onPanelPost, panelMessage } from './mods.svelte'
  import SidePanel from './SidePanel.svelte'
  import type { Agent } from './stream'

  let { panel, agent, onclose }: { panel: Panel; agent: Agent; onclose: () => void } = $props()

  let frame = $state<HTMLIFrameElement>()
  let loaded = $state(false)

  const THEME = ['bg-main', 'surface-1', 'surface-2', 'surface-3', 'border', 'border-focus', 'text', 'text-strong', 'text-muted', 'text-dim', 'ink', 'accent', 'accent-link', 'ok-text', 'err-text', 'sans', 'mono']

  const owner = $derived(mods.list.find((m) => m.id === panel.mod)?.manifest?.name)

  const base = navigator.userAgent.includes('Windows') ? 'http://mod.localhost' : 'mod://localhost'

  function theme() {
    const style = getComputedStyle(document.documentElement)
    frame?.contentWindow?.postMessage({ squadron: 'theme', vars: Object.fromEntries(THEME.map((name) => [name, style.getPropertyValue(`--${name}`).trim()])) }, '*')
  }

  $effect(() => {
    void panel.key
    loaded = false
  })

  $effect(() => {
    themeOf(mods.list)
    theme()
  })

  $effect(() => onPanelPost(panel.mod, panel.id, (message) => frame?.contentWindow?.postMessage({ squadron: 'mod', message }, '*')))

  function message(e: MessageEvent) {
    if (frame && e.source === frame.contentWindow && e.data?.squadron === 'mod') panelMessage(panel, agent, e.data.message)
  }
</script>

<svelte:window onmessage={message} />

<SidePanel kind={panel.key} title={text(panel.title, locale())} {onclose}>
  {#snippet sub()}{t('mods.panelOf', { name: owner ? text(owner, locale()) : panel.mod })}{/snippet}
  {#key panel.key}
    <iframe bind:this={frame} title={text(panel.title, locale())} src="{base}/{panel.mod}/{panel.page}" sandbox="allow-scripts" referrerpolicy="no-referrer" class:loaded onload={() => (theme(), (loaded = true))}></iframe>
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
