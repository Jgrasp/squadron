<script lang="ts">
  import { preview, siteOf, type Preview } from './preview'

  let { url }: { url: string } = $props()

  const site = $derived(siteOf(url))
  let found = $state<Preview | null>(null)
  let noImage = $state(false)
  let noIcon = $state(false)

  $effect(() => {
    const target = url
    found = null
    noImage = noIcon = false
    preview(target).then((p) => target === url && (found = p))
  })
</script>

{#if found}
  <a class="card" href={url} title={url}>
    {#if found.image && !noImage}
      <img class="image" src={found.image} alt="" onerror={() => (noImage = true)} />
    {:else}
      <span class="tile" aria-hidden="true">{#if found.icon && !noIcon}<img src={found.icon} alt="" onerror={() => (noIcon = true)} />{:else}{site.host[0]}{/if}</span>
    {/if}
    <span class="text">
      <span class="meta">{found.site && found.site !== site.host ? `${found.site} · ${site.host}` : site.host}</span>
      <span class="name">{found.title}</span>
      {#if found.description}<span class="description">{found.description}</span>{/if}
    </span>
  </a>
{/if}

<style>
  .card {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 15px;
    border-radius: 8px;
    background: var(--bg-main);
    box-shadow: 0 0 0 1px var(--border-strong);
    overflow: hidden;
    line-height: 1.4;
    color: inherit;
    text-decoration: none;
  }

  .card:hover {
    box-shadow: 0 0 0 1px var(--border-focus);
  }

  .card::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: var(--text-faint);
  }

  .image {
    flex: none;
    width: 104px;
    height: 54px;
    border-radius: 5px;
    object-fit: cover;
    background: var(--surface-3);
  }

  .tile {
    flex: none;
    width: 30px;
    height: 36px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    background: color-mix(in srgb, var(--text-dim) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text-dim) 35%, transparent);
    color: var(--text-dim);
    font: 700 13px var(--sans);
    text-transform: uppercase;
  }

  .tile:has(img) {
    background: var(--text);
    box-shadow: none;
  }

  .tile img {
    width: 16px;
    height: 16px;
    object-fit: contain;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta,
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .description {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 12.5px;
    color: var(--text-muted);
  }
</style>
