<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { CHAPTERS, STOPS, chapterName, firstUnseen, minutes, restartTour, startTour, titleOf, tour } from './tour.svelte'

  let { onstart }: { onstart: () => void } = $props()

  const seen = $derived(STOPS.filter((s) => tour.seen.includes(s.id)).length)
  const check = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--done)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>'

  function play(at: number, fresh = false) {
    onstart()
    fresh ? restartTour() : startTour(at)
  }
</script>

<p class="intro">{t('tour.qsLead')}</p>

<h3>{t('tour.qsTour')}</h3>
<div class="options">
  <div class="option">
    <span class="stat">{#if seen === STOPS.length}{@html check}{:else}<span class="m">{seen}</span>{/if}</span>
    <span class="label">
      <strong>{t('tour.fullTour')}</strong>
      <small>{t('tour.fullMeta', { stops: STOPS.length, min: minutes() })} · {seen ? t('tour.seenSome', { n: seen }) : t('tour.notTaken')}</small>
    </span>
    {#if seen}
      <button class="btn btn-ghost" onclick={() => play(0, true)}>{t('tour.startOver')}</button>
      <button class="btn btn-secondary" onclick={() => play(firstUnseen())}>{t('tour.resume')}</button>
    {:else}
      <button class="btn btn-secondary" onclick={() => play(0, true)}>{t('tour.takeTour')}</button>
    {/if}
  </div>
  {#each CHAPTERS as ch (ch)}
    {@const list = STOPS.filter((s) => s.ch === ch)}
    {@const k = list.filter((s) => tour.seen.includes(s.id)).length}
    <div class="option sub">
      <span class="stat">{#if k === list.length}{@html check}{:else if k}<span class="m">{k}/{list.length}</span>{:else}<span class="dot"></span>{/if}</span>
      <span class="label"><strong>{chapterName(ch)}</strong><small>{list.map(titleOf).join(' · ')}</small></span>
      <button class="btn btn-ghost" onclick={() => play(STOPS.findIndex((s) => s.ch === ch))}>{t('tour.replay')}</button>
    </div>
  {/each}
</div>

<style>
  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  h3 {
    margin: 6px 0 -8px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .options {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .option + .option {
    border-top: 1px solid var(--border);
  }

  .option.sub {
    min-height: 44px;
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .label strong {
    font-size: 13px;
    font-weight: 600;
  }

  .label small {
    overflow: hidden;
    color: var(--text-dim);
    font-size: 12px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .stat {
    width: 26px;
    flex: none;
    display: grid;
    place-items: center;
  }

  .stat .m {
    font: 600 11px var(--mono);
    color: var(--text-dim);
  }

  .stat .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-faint);
  }

  .btn {
    height: 32px;
    padding: 0 12px;
    font-size: 12.5px;
  }
</style>
