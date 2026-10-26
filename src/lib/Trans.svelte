<script lang="ts">
  import type { Snippet } from 'svelte'
  import { parts, type Key } from './i18n/index.svelte'

  let { k, ...values }: { k: Key } & Record<string, string | number | Snippet> = $props()
</script>

{#each parts(k, values) as part, i (i)}{#if typeof part === 'string'}{part}{:else}{@render part()}{/if}{/each}
