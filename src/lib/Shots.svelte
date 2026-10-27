<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity'

  const unfolded = new SvelteSet<string>()
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { preview } from './Attachment.svelte'
  import { t } from './i18n/index.svelte'

  let { paths }: { paths: string[] } = $props()

  let seen = $state<Record<string, string>>({})
  let gone = $state<string[]>([])
  const all = $derived(unfolded.has(paths[0]))
  const shown = $derived(paths.filter((p) => !gone.includes(p)))

  $effect(() => {
    for (const path of paths)
      if (!(path in seen))
        preview(path)
          .then((data) => (seen[path] = data))
          .catch(() => (gone = [...gone, path]))
  })

  function toggle(event: MouseEvent) {
    event.preventDefault()
    event.stopPropagation()
    if (all) unfolded.delete(paths[0])
    else unfolded.add(paths[0])
  }

  function open(event: MouseEvent, path: string) {
    event.preventDefault()
    event.stopPropagation()
    invoke('open_attachment', { path }).catch(() => (gone = [...gone, path]))
  }
</script>

{#if shown.length}
  <span class="shots" class:all>
    {#each all ? shown : shown.slice(0, 4) as path (path)}
      {#if seen[path]}
        <button type="button" title={t('conversation.openCapture')} onclick={(e) => open(e, path)}><img src={seen[path]} alt="" /></button>
      {/if}
    {/each}
    {#if shown.length > 4}
      <button type="button" class="more" aria-expanded={all} title={all ? t('conversation.fewerCaptures') : t('conversation.allCaptures', { count: shown.length })} onclick={toggle}
        >{all ? '−' : `+${shown.length - 4}`}</button
      >
    {/if}
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

  .shots.all {
    flex-basis: 100%;
    flex-wrap: wrap;
    margin-left: 0;
    padding: 6px 0 4px 22px;
  }

  .more {
    width: 44px;
    height: 44px;
    background: var(--surface-2);
    font-size: 12.5px;
    color: var(--text-2);
    cursor: pointer;
  }

  .more:hover {
    color: var(--text-strong);
  }
</style>
