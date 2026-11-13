<script lang="ts">
  import { PAGE_THEME, themeOf } from './extensions'
  import { extensions, onPanelPost, SETTINGS_PAGE, setOption, settingsMessage } from './extensions.svelte'

  let { extension, page, title }: { extension: string; page: string; title: string } = $props()

  let frame = $state<HTMLIFrameElement>()
  let loaded = $state(false)
  let height = $state(120)

  const base = navigator.userAgent.includes('Windows') ? 'http://extension.localhost' : 'extension://localhost'

  function theme() {
    const style = getComputedStyle(document.documentElement)
    frame?.contentWindow?.postMessage({ squadron: 'theme', vars: Object.fromEntries(PAGE_THEME.map((name) => [name, style.getPropertyValue(`--${name}`).trim()])) }, '*')
  }

  const options = () => frame?.contentWindow?.postMessage({ squadron: 'options', values: $state.snapshot(extensions.options[extension] ?? {}) }, '*')

  $effect(() => {
    themeOf(extensions.list)
    theme()
  })

  $effect(() => {
    void extensions.options[extension]
    options()
  })

  $effect(() => onPanelPost(extension, SETTINGS_PAGE, (message) => frame?.contentWindow?.postMessage({ squadron: 'extension', message }, '*')))

  function message(e: MessageEvent) {
    if (!frame || e.source !== frame.contentWindow) return
    const data = e.data ?? {}
    if (data.squadron === 'extension') settingsMessage(extension, data.message)
    if (data.squadron === 'option') setOption(extension, String(data.id), data.value ?? null).catch((err) => console.warn(err))
    if (data.squadron === 'height') height = Math.min(2000, Math.max(40, Number(data.height) || 0))
  }
</script>

<svelte:window onmessage={message} />

{#key `${extension}/${page}`}
  <iframe bind:this={frame} {title} src="{base}/{extension}/{page}" sandbox="allow-scripts" referrerpolicy="no-referrer" class:loaded style:height="{height + 2}px" onload={() => (theme(), options(), (loaded = true))}></iframe>
{/key}

<style>
  iframe {
    display: block;
    width: 100%;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-main);
    visibility: hidden;
  }

  iframe.loaded {
    visibility: visible;
  }
</style>
