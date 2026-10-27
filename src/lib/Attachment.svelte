<script lang="ts" module>
  import { invoke } from '@tauri-apps/api/core'

  const previews = new Map<string, Promise<string>>()

  export const isImage = (path: string) => /\.(png|jpe?g|gif|webp)$/i.test(path)

  export function preview(path: string) {
    if (!previews.has(path)) previews.set(path, invoke<string>('attachment_preview', { path }))
    return previews.get(path)!
  }
</script>

<script lang="ts">
  import { errorText, t } from './i18n/index.svelte'

  let { path, onremove, large = false }: { path: string; onremove?: () => void; large?: boolean } = $props()

  const name = $derived(path.split('/').pop() ?? path)
  let src = $state('')
  let error = $state('')

  $effect(() => {
    src = ''
    if (isImage(path)) preview(path).then((data) => (src = data)).catch(() => {})
  })

  function open() {
    error = ''
    invoke('open_attachment', { path }).catch((e) => (error = errorText(e)))
  }
</script>

<span class="attachment" class:large class:picture={!!src} title={error || path}>
  <button type="button" class="open" aria-label={t('agent.openFile', { name })} onclick={open}>
    {#if src}
      <img {src} alt={name} />
    {:else}
      <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5" /></svg>
      <span class="name">{name}</span>
    {/if}
  </button>
  {#if onremove}
    <button type="button" class="remove" aria-label={t('agent.removeFile', { name })} title={t('common.remove')} onclick={onremove}>×</button>
  {/if}
</span>

<style>
  .attachment {
    position: relative;
    display: inline-flex;
    max-width: 260px;
  }

  .open {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 32px;
    padding: 0 10px;
    border-radius: 6px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    font: 12.5px var(--mono);
  }

  .open:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .open svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .picture .open {
    height: 64px;
    padding: 0;
    overflow: hidden;
  }

  .picture.large .open {
    height: auto;
    max-height: 220px;
  }

  img {
    display: block;
    height: 100%;
    max-width: 240px;
    object-fit: cover;
  }

  .large img {
    max-height: 220px;
    max-width: 360px;
    object-fit: contain;
  }

  .remove {
    position: absolute;
    top: -7px;
    right: -7px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-5);
    box-shadow: 0 0 0 2px var(--bg-main);
    color: var(--text-strong);
    font-size: 13px;
    line-height: 20px;
  }

  .remove:hover {
    background: var(--err);
    color: var(--err-ink);
  }
</style>
