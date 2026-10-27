<script lang="ts" module>
  import { invoke } from '@tauri-apps/api/core'

  const previews = new Map<string, Promise<string>>()

  export const isImage = (path: string) => /\.(png|jpe?g|gif|webp)$/i.test(path)

  const KINDS: [RegExp, Kind][] = [
    [/^(xlsx?|xlsm|ods|csv|tsv|numbers)$/, 'sheet'],
    [/^pdf$/, 'pdf'],
    [/^(docx?|odt|rtf|md|txt|pages)$/, 'doc'],
    [/^(zip|tar|gz|tgz|rar|7z)$/, 'archive'],
    [/^(ts|js|svelte|vue|php|py|rs|go|java|rb|json|ya?ml|sql|html?|css|scss|sh|twig|xml)$/, 'code'],
  ]

  type Kind = 'sheet' | 'pdf' | 'doc' | 'archive' | 'code' | 'file'

  export const kindOf = (path: string): Kind => {
    const ext = path.split('.').pop()?.toLowerCase() ?? ''
    return KINDS.find(([re]) => re.test(ext))?.[1] ?? 'file'
  }

  const sizes = new Map<string, Promise<number>>()

  function sizeOf(path: string) {
    if (!sizes.has(path)) sizes.set(path, invoke<number>('attachment_size', { path }))
    return sizes.get(path)!
  }

  export function preview(path: string) {
    if (!previews.has(path)) previews.set(path, invoke<string>('attachment_preview', { path }))
    return previews.get(path)!
  }
</script>

<script lang="ts">
  import { errorText, number, t, type Key } from './i18n/index.svelte'

  let {
    path,
    onremove,
    large = false,
    look = 'chip',
  }: { path: string; onremove?: () => void; large?: boolean; look?: 'chip' | 'card' } = $props()

  const name = $derived(path.split('/').pop() ?? path)
  const ext = $derived(name.includes('.') ? name.split('.').pop()!.toUpperCase().slice(0, 4) : '')
  const kind = $derived(kindOf(path))
  const label = $derived(t(`agent.kind${kind[0].toUpperCase()}${kind.slice(1)}` as Key))
  let src = $state('')
  let error = $state('')
  let bytes = $state<number | null>(null)

  $effect(() => {
    bytes = null
    if (look !== 'chip') sizeOf(path).then((n) => (bytes = n)).catch(() => {})
  })

  const size = $derived(
    bytes === null
      ? ''
      : bytes < 1024
        ? number(bytes, { style: 'unit', unit: 'byte', unitDisplay: 'short' })
        : bytes < 1024 ** 2
          ? number(Math.round(bytes / 1024), { style: 'unit', unit: 'kilobyte', unitDisplay: 'short' })
          : number(bytes / 1024 ** 2, { style: 'unit', unit: 'megabyte', unitDisplay: 'short', maximumFractionDigits: 1 }),
  )

  $effect(() => {
    src = ''
    if (isImage(path)) preview(path).then((data) => (src = data)).catch(() => {})
  })

  function open() {
    error = ''
    invoke('open_attachment', { path }).catch((e) => (error = errorText(e)))
  }
</script>

<span class="attachment {look} {kind}" class:large class:picture={!!src} title={error || path}>
  <button type="button" class="open" aria-label={t('agent.openFile', { name })} onclick={open}>
    {#if src}
      <img {src} alt={name} />
    {:else if look !== 'chip'}
      <span class="tile" aria-hidden="true">{ext || '·'}</span>
      <span class="text">
        <span class="name">{name}</span>
        <span class="meta">{label}{size ? ` · ${size}` : ''}</span>
      </span>
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

  .sheet {
    --kind: #4cbf85;
  }

  .pdf {
    --kind: #e5655a;
  }

  .doc {
    --kind: #6ea8f0;
  }

  .archive {
    --kind: #e0b65b;
  }

  .code {
    --kind: #b79cf0;
  }

  .file {
    --kind: var(--text-dim);
  }

  .card {
    max-width: 300px;
  }

  .card .open {
    gap: 10px;
    min-width: 220px;
    height: auto;
    padding: 9px 14px 9px 10px;
    border-radius: 8px;
    background: var(--surface-2);
    text-align: left;
    font: 13px var(--sans);
  }

  .tile {
    flex: none;
    width: 34px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    background: color-mix(in srgb, var(--kind) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--kind) 35%, transparent);
    color: var(--kind);
    font: 700 9.5px/1 var(--sans);
    letter-spacing: 0.04em;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card .name {
    font-weight: 600;
    color: var(--text-strong);
  }

  .meta {
    font-size: 11.5px;
    color: var(--text-dim);
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
