<script lang="ts">
  import { preview, siteOf } from './preview'

  let { url, light = false, link = false }: { url: string; light?: boolean; link?: boolean } = $props()

  const site = $derived(siteOf(url))
  let icons = $state<string[]>([])
  let failed = $state(0)

  $effect(() => {
    const target = url
    icons = [siteOf(target).favicon].filter(Boolean)
    failed = 0
    preview(target).then((found) => found?.icon && target === url && !icons.includes(found.icon) && icons.push(found.icon))
  })
</script>

<svelte:element this={link ? 'a' : 'span'} href={link ? url : undefined} class="chip" class:light title={url}>
  <i aria-hidden="true">
    {#if icons[failed]}<img src={icons[failed]} alt="" onerror={() => failed++} />{:else}{site.host[0]}{/if}
  </i>
  <span class="host">{site.host}</span>{#if site.path}<span class="path">{site.path}</span>{/if}
</svelte:element>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    height: 20px;
    margin: 1px 0;
    padding: 0 7px 0 3px;
    border-radius: 4px;
    vertical-align: -1px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 12.5px var(--mono);
    color: var(--text-2);
    text-decoration: none;
    white-space: nowrap;
  }

  a.chip {
    cursor: pointer;
  }

  .chip i {
    flex: none;
    display: grid;
    place-items: center;
    width: 15px;
    height: 15px;
    margin-right: 5px;
    border-radius: 3px;
    font: normal 700 9px var(--sans);
    text-transform: uppercase;
  }

  .chip:not(.light) i:has(img) {
    background: var(--text);
  }

  .chip img {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    object-fit: contain;
  }

  .host {
    flex: none;
  }

  .path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-faint);
  }

  .light {
    background: color-mix(in srgb, var(--accent-ink) 13%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent-ink) 22%, transparent);
    color: var(--accent-ink);
  }

  a.light:hover {
    background: color-mix(in srgb, var(--accent-ink) 20%, transparent);
  }

  .light .path {
    color: color-mix(in srgb, var(--accent-ink) 58%, transparent);
  }
</style>
