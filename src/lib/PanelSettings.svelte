<script lang="ts">
  import { displayOf, layout, MAX_SHARE, MIN_PANEL, MIN_SHARE, resetDisplay, resizePanel, resizeShare, setDisplay, setFileTree, setSplit } from './layout.svelte'
  import { MODES, modeLabel, PANELS } from './panels'
  import { t } from './i18n/index.svelte'
  import ModeIcon from './ModeIcon.svelte'

  const widest = Math.max(MIN_PANEL + 200, window.innerWidth - layout.sidebar)
</script>

<div class="intro">
  <p>{t('settings.panelsIntro')}</p>
  <button class="btn btn-secondary" onclick={resetDisplay}>{t('settings.backToDefault')}</button>
</div>
{#each PANELS as [kind, title] (kind)}
  {@const shown = displayOf(kind)}
  <section>
    <h2>{t(title)}</h2>
    <div class="modes" role="group" aria-label={t('shell.panelPlacement')}>
      {#each MODES as mode (mode)}
        <button aria-pressed={shown.mode === mode} onclick={() => setDisplay(kind, { mode })}>
          <ModeIcon {mode} size={26} />
          {t(modeLabel[mode])}
        </button>
      {/each}
    </div>
    {#if shown.mode === 'slide'}
      <label class="size">
        <span>{t('settings.panelWidth')}</span>
        <input type="range" min={MIN_PANEL} max={Math.max(widest, shown.width)} step="4" value={shown.width} oninput={(e) => resizePanel(kind, e.currentTarget.valueAsNumber, widest)} />
        <output>{shown.width} px</output>
      </label>
    {:else if shown.mode === 'bottom'}
      <label class="size">
        <span>{t('settings.panelHeight')}</span>
        <input type="range" min={MIN_SHARE} max={MAX_SHARE} value={shown.height} oninput={(e) => resizeShare(kind, e.currentTarget.valueAsNumber)} />
        <output>{shown.height} %</output>
      </label>
    {:else}
      <p class="note">{t('settings.panelFullNote')}</p>
    {/if}
    {#if kind === 'diff'}
      <div class="option">
        <span>{t('settings.diffLayout')}</span>
        <div class="pair" role="group" aria-label={t('settings.diffLayout')}>
          <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>{t('review.unified')}</button>
          <button aria-pressed={layout.split} onclick={() => setSplit(true)}>{t('review.split')}</button>
        </div>
      </div>
      <label class="option">
        <span>{t('review.fileTree')}</span>
        <input type="checkbox" checked={layout.fileTree} onchange={(e) => setFileTree(e.currentTarget.checked)} />
      </label>
    {/if}
  </section>
{/each}

<style>
  .intro {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .intro p {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .intro .btn {
    flex: none;
    height: 34px;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }

  h2 {
    margin: 0;
    font-size: 13.5px;
    font-weight: 600;
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .modes button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-2);
    text-align: left;
  }

  .modes button :global(svg) {
    flex: none;
    color: var(--text-dim);
  }

  .modes button[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
    color: var(--text);
  }

  .modes button[aria-pressed='true'] :global(svg) {
    color: var(--accent);
  }

  .size,
  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .size span,
  .option > span {
    width: 110px;
    flex: none;
  }

  .size input {
    flex: 1;
    accent-color: var(--accent);
  }

  .size output {
    width: 64px;
    text-align: right;
    font: 12.5px var(--mono);
    color: var(--text-2);
  }

  .option input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }

  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .pair {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 6px;
    background: var(--surface-2);
  }

  .pair button {
    padding: 5px 12px;
    border-radius: 4px;
    color: var(--text-dim);
  }

  .pair button[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }
</style>
