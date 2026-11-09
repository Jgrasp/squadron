<script lang="ts">
  import { KIND_ICONS, kindOf } from './Attachment.svelte'

  let { path, count = 0, light = false }: { path: string; count?: number; light?: boolean } = $props()

  const trimmed = $derived(path.replace(/\/$/, ''))
  const name = $derived(trimmed.split('/').pop()!)
  const folder = $derived(path.endsWith('/'))
  const kind = $derived(kindOf(path))
</script>

<span class="chip {folder ? 'folder' : kind}" class:light title={path}>
  {#if folder}
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /><path d="M3 10h18" fill="none" /></svg>
    <span><span class="parent">{trimmed.slice(0, -name.length)}</span>{name}</span><span class="count">{count}</span>
  {:else}
    <i aria-hidden="true"><svg viewBox="0 0 24 24">{@html KIND_ICONS[kind]}</svg></i>{name}
  {/if}
</span>

<style>
  .chip {
    --kind: var(--text-dim);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    margin: 1px 0;
    padding: 0 7px 0 3px;
    border-radius: 4px;
    vertical-align: -1px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 12.5px var(--mono);
    color: var(--text-2);
    white-space: nowrap;
  }

  .chip i {
    display: grid;
    place-items: center;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--kind) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--kind) 35%, transparent);
    color: var(--kind);
  }

  .chip svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .chip.folder {
    gap: 4px;
    padding-left: 6px;
  }

  .chip.folder svg {
    width: 14px;
    height: 14px;
    margin-right: 1px;
    color: var(--text-muted);
    fill: currentColor;
    fill-opacity: 0.28;
    stroke-width: 1.8;
  }

  .chip .parent {
    color: var(--text-faint);
  }

  .chip .count {
    margin-left: 3px;
    color: var(--text-dim);
    font-size: 11px;
  }

  .chip.sheet {
    --kind: #4cbf85;
  }

  .chip.pdf {
    --kind: #e5655a;
  }

  .chip.doc {
    --kind: #6ea8f0;
  }

  .chip.archive {
    --kind: #e0b65b;
  }

  .chip.code {
    --kind: #b79cf0;
  }

  .light {
    background: var(--accent-ink);
    box-shadow: none;
    color: var(--accent-soft);
  }

  .light :is(.parent, .count) {
    color: color-mix(in srgb, var(--accent-soft) 55%, transparent);
  }

  .light.folder svg {
    color: var(--accent-soft);
  }
</style>
