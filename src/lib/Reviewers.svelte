<script lang="ts">
  import Avatar from './Avatar.svelte'
  import type { Reviewer } from './pulls.svelte'

  let { reviewers, size = 24 }: { reviewers: Reviewer[]; size?: number } = $props()

  const SAID: Record<string, string> = {
    approved: 'approved',
    changes_requested: 'asked for changes',
    commented: 'commented',
    waiting: 'has not reviewed yet',
  }
</script>

<span class="reviewers" style:--size="{size}px">
  {#each reviewers as r (r.name)}
    <span class="who {r.verdict}" title="{r.name} · {SAID[r.verdict] ?? r.verdict}" role="img" aria-label="{r.name} {SAID[r.verdict] ?? r.verdict}">
      <span class="face"><Avatar name={r.name} src={r.avatar} {size} /></span>
      {#if r.verdict !== 'waiting'}
        <span class="mark" aria-hidden="true">
          {#if r.verdict === 'approved'}
            <svg viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
          {:else if r.verdict === 'changes_requested'}
            <svg viewBox="0 0 24 24"><path d="M7 7l10 10M17 7L7 17" /></svg>
          {:else}
            <svg viewBox="0 0 24 24"><path d="M6 12h12" /></svg>
          {/if}
        </span>
      {/if}
    </span>
  {/each}
</span>

<style>
  .reviewers {
    display: inline-flex;
    align-items: center;
    gap: 9px;
  }

  .who {
    position: relative;
    width: var(--size);
    height: var(--size);
  }

  .face {
    display: flex;
    border-radius: 50%;
    box-shadow: 0 0 0 2px var(--bg-main);
  }

  .waiting .face {
    opacity: 0.55;
  }

  .approved .face {
    box-shadow:
      0 0 0 2px var(--bg-main),
      0 0 0 3.5px var(--ok);
  }

  .changes_requested .face {
    box-shadow:
      0 0 0 2px var(--bg-main),
      0 0 0 3.5px var(--err);
  }

  .mark {
    position: absolute;
    right: -3px;
    bottom: -3px;
    width: 13px;
    height: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--text-faint);
    box-shadow: 0 0 0 2px var(--bg-main);
  }

  .approved .mark {
    background: var(--ok);
  }

  .changes_requested .mark {
    background: var(--err);
  }

  .mark svg {
    width: 9px;
    height: 9px;
    fill: none;
    stroke: var(--ink);
    stroke-width: 3.5;
    stroke-linecap: round;
  }
</style>
