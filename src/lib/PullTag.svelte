<script lang="ts">
  import { progress, pulls, stateLabel } from './pulls.svelte'
  import { workdir, type Agent } from './stream'
  import { t } from './i18n/index.svelte'

  let { agent }: { agent: Agent } = $props()

  const pull = $derived(pulls[workdir(agent)])
  const review = (p: NonNullable<typeof pull>) => (p.changes_requested ? t('review.changesRequestedTag') : p.reviewers.length || p.approvals ? progress(p) : '')
</script>

{#if pull?.state === 'open'}
  <span class="pull" class:draft={pull.draft} title={pull.title}>
    {[t('review.pullTag', { number: String(pull.number) }), stateLabel(pull), review(pull)].filter(Boolean).join(' · ')}
  </span>
{/if}

<style>
  .pull {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 11.5px var(--sans);
    color: var(--ok-text);
  }

  .draft {
    color: var(--text-dim);
  }
</style>
