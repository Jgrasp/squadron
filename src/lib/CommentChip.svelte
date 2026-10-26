<script lang="ts">
  import type { Comment } from './stream'
  import { t } from './i18n/index.svelte'

  let { comment, light = false, onopen, onremove }: { comment: Comment; light?: boolean; onopen?: () => void; onremove?: () => void } =
    $props()

  const ref = $derived(comment.path ? `${comment.path.split('/').pop()}:${comment.line}` : t('review.pullRequest'))
  const title = $derived(comment.path ? `${comment.path}:${comment.line}${comment.removed ? ` · ${t('review.removedLine')}` : ''}\n${comment.code.trim()}\n\n${comment.note}` : comment.note)
</script>

{#snippet body()}
  <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z" /></svg>
  <span class="ref">{ref}</span>
  <span class="note">{comment.note}</span>
{/snippet}

<span class="chip" class:light {title}>
  {#if onopen}
    <button type="button" class="open" onclick={onopen} aria-label={t('review.editCommentOnLineOf', { path: comment.path, line: String(comment.line) })}>{@render body()}</button>
  {:else}
    {@render body()}
  {/if}
  {#if onremove}
    <button type="button" class="remove" aria-label={t('review.removeCommentOn', { ref })} onclick={onremove}>×</button>
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

  .open {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 100%;
  }

  .open:hover .ref {
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
    stroke: var(--accent-ink);
  }

  .light .ref {
    color: var(--accent-ink);
  }

  .light .note {
    color: rgba(36, 18, 10, 0.72);
  }
</style>
