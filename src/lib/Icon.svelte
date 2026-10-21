<script lang="ts">
  let { name, size = 14 }: { name: string; size?: number } = $props()
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
  {#if name === 'editor'}
    <path d="M8 6l-6 6 6 6M16 6l6 6-6 6" />
  {:else if name === 'terminal'}
    <path d="M4 17l6-6-6-6M12 19h8" />
  {:else if name === 'diff'}
    <circle cx="6" cy="6" r="2" /><circle cx="18" cy="18" r="2" /><path d="M6 8v8a2 2 0 0 0 2 2h8M18 16V8a2 2 0 0 0-2-2H8" />
  {:else if name === 'kanban'}
    <rect x="3" y="4" width="5" height="16" rx="1" /><rect x="10" y="4" width="5" height="10" rx="1" /><rect x="17" y="4" width="4" height="13" rx="1" />
  {:else if name === 'eye'}
    <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" /><circle cx="12" cy="12" r="3" />
  {:else if name === 'eye-off'}
    <path d="M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M9.9 5.1A9.8 9.8 0 0 1 12 5c6 0 10 7 10 7a17 17 0 0 1-3.2 3.9M6.1 6.1A17 17 0 0 0 2 12s4 7 10 7a9.6 9.6 0 0 0 4-.9" />
  {/if}
</svg>

<style>
  .icon {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
