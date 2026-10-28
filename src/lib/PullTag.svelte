<script lang="ts">
  import { progress, pullOf, stateLabel } from './pulls.svelte'
  import type { Agent } from './stream'
  import { t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'

  let { agent, gutter = false }: { agent: Agent; gutter?: boolean } = $props()

  const pull = $derived(pullOf(agent))
  const review = (p: NonNullable<typeof pull>) => (p.changes_requested ? t('review.changesRequestedTag') : p.reviewers.length || p.approvals ? progress(p) : '')
</script>

{#if agent.review}
  <span class="pull reviewing" class:gutter title={t('review.reviewingTitle', { number: String(agent.review.number) })}>
    <Icon name="pull" size={11} />
    <span class="text">{[t('review.reviewingTag', { number: String(agent.review.number) }), agent.review.title].filter(Boolean).join(' · ')}</span>
  </span>
{:else if pull?.state === 'open'}
  <span class="pull" class:draft={pull.draft} class:gutter title={pull.title}>
    {#if gutter}<Icon name="pull" size={11} />{/if}
    <span class="text">{[t('review.pullTag', { number: String(pull.number) }), stateLabel(pull), review(pull)].filter(Boolean).join(' · ')}</span>
  </span>
{/if}

<style>
  .pull {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font: 11.5px var(--sans);
    color: var(--ok-text);
  }

  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .gutter :global(svg) {
    flex: none;
  }

  .reviewing {
    align-self: flex-start;
    max-width: 100%;
    padding: 1px 7px 1px 5px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--remote-dot) 14%, transparent);
    color: var(--remote-text);
    font-weight: 600;
  }

  .reviewing :global(svg) {
    flex: none;
  }

  .draft {
    color: var(--text-dim);
  }
</style>
