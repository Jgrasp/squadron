<script lang="ts">
  import { brand } from './extension-pages.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import { LOGO } from './extensions'
  import Icon from './Icon.svelte'

  let { icon, extension, fallback }: { icon?: string; extension: string; fallback?: string } = $props()

  const name = $derived(icon ?? fallback)
</script>

{#if name === LOGO}<ExtensionLogo {...brand(extension)} size={14} />{:else if name}<Icon {name} />{/if}
