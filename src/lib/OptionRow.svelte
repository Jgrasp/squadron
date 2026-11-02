<script lang="ts">
  import type { Snippet } from 'svelte'

  type Choice = { value: string; label: string; detail?: string; off?: boolean; tone?: string }

  let {
    label,
    icon,
    value,
    tone = '',
    choices,
    selected,
    onpick,
  }: { label: string; icon: Snippet; value: string; tone?: string; choices: Choice[]; selected: string; onpick: (value: string) => void } = $props()

  let open = $state(false)
  let root = $state<HTMLElement>()
</script>

<svelte:window onpointerdown={(e) => open && !root?.contains(e.target as Node) && (open = false)} />

<div class="option-row" bind:this={root}>
  <button type="button" class="option-trigger" aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="option-icon" aria-hidden="true">{@render icon()}</span>
    <span class="option-key">{label}</span>
    <span class="option-value {tone}">{value}</span>
    <svg class="option-updown" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 9l4-4 4 4M16 15l-4 4-4-4" /></svg>
  </button>
  {#if open}
    <div class="option-menu" role="listbox" aria-label={label}>
      {#each choices as choice (choice.value)}
        <button
          type="button"
          role="option"
          aria-selected={choice.value === selected}
          disabled={choice.off}
          onclick={() => ((open = false), onpick(choice.value))}
        >
          <strong class={choice.tone ?? ''}>{choice.label}</strong>
          {#if choice.detail}<span>{choice.detail}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
