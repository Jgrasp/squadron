<script lang="ts">
  import type { Comment } from './stream'

  let { comment, light = false, onopen, onremove }: { comment: Comment; light?: boolean; onopen?: () => void; onremove?: () => void } =
    $props()

  const ref = $derived(`${comment.path.split('/').pop()}:${comment.line}`)
  const title = $derived(`${comment.path}:${comment.line}${comment.removed ? ' · removed line' : ''}\n${comment.code.trim()}\n\n${comment.note}`)
</script>

<span class="chip" class:light {title}>
  <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>
  {#if onopen}
    <button type="button" class="ref" onclick={onopen} aria-label="Open {comment.path} line {comment.line} in the diff">{ref}</button>
  {:else}
    <span class="ref">{ref}</span>
  {/if}
  <span class="note">{comment.note}</span>
  {#if onremove}
    <button type="button" class="remove" aria-label="Remove the comment on {ref}" onclick={onremove}>×</button>
  {/if}
</span>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 100%;
    height: 26px;
    padding: 0 8px;
    border-radius: 4px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font-size: 12.5px;
    line-height: 1;
    white-space: nowrap;
  }

  .chip:has(.remove) {
    padding-right: 2px;
  }

  svg {
    flex: none;
    fill: none;
    stroke: var(--accent-soft);
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .ref {
    flex: none;
    font: 12px var(--mono);
    color: var(--text-2);
  }

  button.ref:hover {
    color: var(--text-strong);
    text-decoration: underline;
  }

  .note {
    min-width: 0;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-muted);
  }

  .remove {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 3px;
    color: var(--text-dim);
    font-size: 15px;
  }

  .remove:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .light {
    background: rgba(26, 21, 19, 0.07);
    box-shadow: inset 0 0 0 1px rgba(26, 21, 19, 0.16);
  }

  .light svg {
    stroke: #9a5530;
  }

  .light .ref {
    color: var(--ink);
  }

  .light .note {
    color: #5c514b;
  }
</style>
