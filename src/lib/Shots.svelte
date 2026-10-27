<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { preview } from './Attachment.svelte'
  import { t } from './i18n/index.svelte'

  let { paths }: { paths: string[] } = $props()

  let seen = $state<Record<string, string>>({})
  let gone = $state<string[]>([])
  const shown = $derived(paths.filter((p) => !gone.includes(p)))

  $effect(() => {
    for (const path of paths)
      if (!(path in seen))
        preview(path)
          .then((data) => (seen[path] = data))
          .catch(() => (gone = [...gone, path]))
  })

  function open(event: MouseEvent, path: string) {
    event.preventDefault()
    event.stopPropagation()
    invoke('open_attachment', { path }).catch(() => (gone = [...gone, path]))
  }
</script>

{#if shown.length}
  <span class="shots">
    {#each shown.slice(0, 4) as path (path)}
      {#if seen[path]}
        <button type="button" title={t('conversation.openCapture')} onclick={(e) => open(e, path)}><img src={seen[path]} alt="" /></button>
      {/if}
    {/each}
    {#if shown.length > 4}<span class="more">+{shown.length - 4}</span>{/if}
  </span>
{/if}

<style>
  .shots {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    padding-left: 12px;
  }

  button {
    display: block;
    padding: 0;
    border-radius: 4px;
    box-shadow: 0 0 0 1px var(--border-strong);
    overflow: hidden;
    cursor: zoom-in;
  }

  button:hover {
    box-shadow: 0 0 0 1px var(--text-muted);
  }

  img {
    display: block;
    width: 72px;
    height: 44px;
    object-fit: cover;
    object-position: top left;
  }

  .more {
    font-size: 12px;
    color: var(--text-dim);
  }
</style>
