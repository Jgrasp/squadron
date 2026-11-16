<script lang="ts">
  import { colorOf } from './extension-pages.svelte'

  let { id, name, logo = null, size = 34, off = false }: { id: string; name: string; logo?: string | null; size?: number; off?: boolean } = $props()
</script>

{#if logo}
  <img class="image" class:off src={logo} alt="" width={size} height={size} draggable="false" />
{:else}
  <span class="logo" class:off style:background={colorOf(id)} style:width="{size}px" style:height="{size}px" style:font-size="{Math.max(10, Math.round(size * 0.4))}px" style:border-radius="{Math.max(4, Math.round(size / 5))}px" aria-hidden="true">{name.charAt(0).toUpperCase()}</span>
{/if}

<style>
  .logo {
    flex: none;
    display: grid;
    place-items: center;
    color: var(--ink);
    font-weight: 700;
    line-height: 1;
  }

  .image {
    flex: none;
    object-fit: contain;
  }

  .off {
    opacity: 0.5;
  }
</style>
