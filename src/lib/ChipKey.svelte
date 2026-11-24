<script lang="ts">
  import { brand } from './extension-pages.svelte'
  import ExtensionLogo from './ExtensionLogo.svelte'
  import type { Chip } from './newAgentDraft'

  let { chip, logo = chip.logo !== false }: { chip: Chip; logo?: boolean } = $props()
</script>

{#if logo}<ExtensionLogo {...brand(chip.extension)} size={11} />{/if}{#if chip.tone}<span class="tone {chip.tone}" aria-hidden="true"></span>{/if}<span class="key">{chip.key}</span>

<style>
  .tone {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .tone.ok {
    background: var(--ok);
  }

  .tone.err {
    background: var(--err);
  }

  .tone.info {
    background: var(--remote-dot);
  }

  .key {
    white-space: nowrap;
  }
</style>
