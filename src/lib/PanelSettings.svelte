<script lang="ts">
  import { displayOf, layout, resetDisplay, setDisplay, setFileTree, setSplit, type PanelSize } from './layout.svelte'
  import { MODES, modeLabel, modeShort, PANELS } from './panels'
  import { t } from './i18n/index.svelte'
  import ModeIcon from './ModeIcon.svelte'
  import Select from './Select.svelte'

  const sizes = (custom: boolean) => [
    { value: 'small', label: t('settings.sizeSmall'), detail: t('settings.sizeSmallHint') },
    { value: 'medium', label: t('settings.sizeMedium'), detail: t('settings.sizeMediumHint') },
    { value: 'large', label: t('settings.sizeLarge'), detail: t('settings.sizeLargeHint') },
    ...(custom ? [{ value: 'custom', label: t('settings.sizeCustom'), detail: t('settings.sizeCustomHint'), disabled: true }] : []),
  ]
</script>

<div class="intro">
  <p>{t('settings.panelsIntro')}</p>
  <button class="btn btn-secondary" onclick={resetDisplay}>{t('settings.backToDefault')}</button>
</div>
<div class="table">
  <div class="row head">
    <span></span>
    {#each MODES as mode (mode)}
      <span class="col"><ModeIcon {mode} size={64} />{t(modeShort[mode])}</span>
    {/each}
    <span class="col size">{t('settings.panelSize')}</span>
  </div>
  {#each PANELS as [kind, title] (kind)}
    {@const shown = displayOf(kind)}
    <div class="row" role="radiogroup" aria-label={t(title)}>
      <span class="name">{t(title)}</span>
      {#each MODES as mode (mode)}
        <button class="cell" role="radio" aria-checked={shown.mode === mode} aria-label={t(modeLabel[mode])} title={t(modeLabel[mode])} onclick={() => setDisplay(kind, { mode })}>
          <span class="dot"></span>
        </button>
      {/each}
      {#if shown.mode === 'full'}
        <span class="whole">{t('settings.sizeWhole')}</span>
      {:else}
        <Select
          value={shown.size ?? 'custom'}
          options={sizes(!shown.size)}
          label={`${t('settings.panelSize')} · ${t(title)}`}
          onpick={(size) => setDisplay(kind, { size: size as PanelSize })}
        />
      {/if}
    </div>
  {/each}
</div>
<section>
  <h2>{t('shell.diff')}</h2>
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
</section>

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

  .table {
    padding: 6px;
    border-radius: 8px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .row {
    display: grid;
    grid-template-columns: minmax(110px, 1fr) repeat(4, 76px) 168px;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 6px;
  }

  .row:not(.head):hover {
    background: var(--surface-2);
  }

  .head {
    margin-bottom: 4px;
    padding: 10px 8px 12px;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
  }

  .col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    color: var(--text-2);
    text-align: center;
  }

  .col.size {
    align-self: end;
    align-items: flex-start;
    padding-left: 2px;
    color: var(--text-dim);
  }

  .name {
    padding-left: 4px;
    font-size: 13px;
    font-weight: 600;
  }

  .cell {
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
  }

  .cell:hover {
    background: var(--surface-3);
  }

  .dot {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--border-strong);
  }

  .cell[aria-checked='true'] {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .cell[aria-checked='true'] .dot {
    box-shadow: inset 0 0 0 4.5px var(--accent);
  }

  .row :global(.trigger) {
    height: 32px;
  }

  .whole {
    padding-left: 12px;
    font-size: 12.5px;
    color: var(--text-dim);
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

  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .option > span {
    width: 110px;
    flex: none;
  }

  .option input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
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
