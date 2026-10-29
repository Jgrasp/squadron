<script lang="ts">
  import { namespace, shortName } from './agenttools'
  import Icon from './Icon.svelte'

  let { name, small = false, title, onclick }: { name: string; small?: boolean; title?: string; onclick?: (event: MouseEvent) => void } = $props()
</script>

{#snippet content()}<Icon name="persona" size={small ? 11 : 13} /><span><span class="ns">{namespace(name)}</span>{shortName(name)}</span>{/snippet}

{#if onclick}
  <button class="chip" class:small {title} type="button" {onclick}>{@render content()}</button>
{:else}
  <span class="chip" class:small {title}>{@render content()}</span>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
    height: 22px;
    padding: 0 8px 0 6px;
    border: 0;
    border-radius: 3px;
    background: var(--persona-bg);
    box-shadow: inset 0 0 0 1px var(--persona-line);
    color: var(--persona);
    font: 500 12px var(--mono);
    white-space: nowrap;
  }

  .chip :global(.icon) {
    margin-right: 1px;
  }

  button.chip {
    cursor: pointer;
  }

  button.chip:hover {
    background: rgba(227, 211, 180, 0.16);
  }

  .small {
    height: 18px;
    padding: 0 6px 0 4px;
    gap: 3px;
    font-size: 11px;
  }

  .ns {
    color: color-mix(in srgb, var(--persona) 55%, transparent);
    font-weight: 400;
  }
</style>
