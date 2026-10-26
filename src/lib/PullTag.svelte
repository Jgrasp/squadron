<script lang="ts">
  import { progress, pulls, stateLabel } from './pulls.svelte'
  import { workdir, type Agent } from './stream'
  import { t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'

  let { agent, gutter = false }: { agent: Agent; gutter?: boolean } = $props()

  const pull = $derived(pulls[workdir(agent)])
  const review = (p: NonNullable<typeof pull>) => (p.changes_requested ? t('review.changesRequestedTag') : p.reviewers.length || p.approvals ? progress(p) : '')
</script>

{#if pull?.state === 'open'}
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

  .draft {
    color: var(--text-dim);
  }
</style>
