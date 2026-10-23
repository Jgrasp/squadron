<script lang="ts">
  import type { Status } from './stream'

  let { status }: { status: Status } = $props()
</script>

{#if status === 'working'}
  <svg class="spin" width="10" height="10" viewBox="0 0 24 24" role="img" aria-label="Working">
    <circle cx="12" cy="12" r="9" fill="none" stroke="var(--ok)" stroke-width="4" stroke-dasharray="40 20" />
  </svg>
{:else if status === 'waiting'}
  <span class="dot" role="img" aria-label="Waiting"></span>
{:else if status === 'done'}
  <svg width="10" height="10" viewBox="0 0 24 24" role="img" aria-label="Done">
    <path d="M4 12l5 5L20 6" fill="none" stroke="var(--done)" stroke-width="3.5" stroke-linecap="round" stroke-linejoin="round" />
  </svg>
{:else if status === 'interrupted'}
  <svg width="10" height="10" viewBox="0 0 24 24" role="img" aria-label="Interrupted">
    <rect x="5" y="5" width="14" height="14" rx="2" fill="var(--text-2)" />
  </svg>
{:else}
  <svg width="10" height="10" viewBox="0 0 24 24" role="img" aria-label="Error">
    <path d="M5 5l14 14M19 5L5 19" fill="none" stroke="var(--err)" stroke-width="4" stroke-linecap="round" />
  </svg>
{/if}

<style>
  svg,
  .dot {
    flex: none;
  }

  .spin {
    animation: spin 1.4s linear infinite;
  }

  .dot {
    width: 8px;
    height: 8px;
    margin: 0 1px;
    border-radius: 4px;
    background: var(--accent);
    box-shadow: 0 0 0 4px rgba(224, 138, 91, 0.25);
  }
</style>
