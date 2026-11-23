<script lang="ts">
  import { displayOf, layout, resetDisplay, setDisplay, setFileTree, setSplit, type PanelSize } from './layout.svelte'
  import { MODES, modeLabel, modeShort, PANELS } from './panels'
  import { errorText, t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'
  import ModeIcon from './ModeIcon.svelte'
  import { detect } from './OpenWith.svelte'
  import Select from './Select.svelte'
  import { saveSettings, settings, type App } from './settings.svelte'
  import Trans from './Trans.svelte'

  const sizes = (custom: boolean) => [
    { value: 'small', label: t('settings.sizeSmall'), detail: t('settings.sizeSmallHint') },
    { value: 'medium', label: t('settings.sizeMedium'), detail: t('settings.sizeMediumHint') },
    { value: 'large', label: t('settings.sizeLarge'), detail: t('settings.sizeLargeHint') },
    ...(custom ? [{ value: 'custom', label: t('settings.sizeCustom'), detail: t('settings.sizeCustomHint'), disabled: true }] : []),
  ]

  let found = $state<App[]>([])
  detect().then((list) => (found = list))
  let own = $state<App[] | null>(settings.editors.length ? settings.editors.map((a) => ({ ...a })) : null)
  let editing = $state(-1)
  let error = $state('')
  const apps = $derived(own ?? found)

  function take() {
    own ??= found.map((a) => ({ ...a }))
    return own
  }

  async function store() {
    error = ''
    const editors = (own ?? []).filter((a) => a.name.trim() && a.command.trim())
    try {
      await saveSettings({ ...$state.snapshot(settings), editors: $state.snapshot(editors) })
    } catch (e) {
      error = errorText(e)
    }
  }

  function edit(i: number) {
    take()
    editing = editing === i ? -1 : i
    store()
  }

  function remove(i: number) {
    take().splice(i, 1)
    if (!own?.length) own = null
    editing = -1
    store()
  }

  function add() {
    editing = take().push({ name: '', command: '' }) - 1
  }

  function back() {
    own = null
    editing = -1
    store()
  }
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
<section>
  <div class="titled">
    <h2>{t('settings.editorsTitle')}</h2>
    {#if own}<button class="btn btn-ghost" onclick={back}>{t('settings.editorsBack')}</button>{/if}
  </div>
  <p class="hint">
    <Trans k="settings.editorsIntro">{#snippet project()}<code>{'{project}'}</code>{/snippet}{#snippet file()}<code>{'{file}'}</code>{/snippet}</Trans>
  </p>
  {#if apps.length}
    <div class="apps">
      {#each apps as app, i (i)}
        <div class="app">
          <div class="line">
            <span class="label">
              <strong>{app.name || t('settings.editorNew')}{#if !own}<span class="pill">{t('settings.editorFound')}</span>{/if}</strong>
              <code>{app.command || '—'}</code>
            </span>
            <button class="btn btn-ghost" onclick={() => edit(i)}>{editing === i ? t('settings.editorDone') : t('common.edit')}</button>
            <button class="x" title={t('common.remove')} aria-label={t('common.remove')} onclick={() => remove(i)}>×</button>
          </div>
          {#if editing === i}
            <div class="sub">
              <input class="field" bind:value={app.name} onchange={store} placeholder={t('common.name')} aria-label={t('common.name')} />
              <input class="field mono" bind:value={app.command} onchange={store} placeholder={'zed {project} {file}'} aria-label={t('settings.editorCommand')} spellcheck="false" />
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <p class="hint">{t('settings.editorsNone')}</p>
  {/if}
  <button class="btn btn-ghost add" onclick={add}><Icon name="plus" />{t('settings.addApp')}</button>
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
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

  .titled {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .btn-ghost {
    height: 30px;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .hint code {
    font: 12px var(--mono);
    color: var(--text-2);
  }

  .apps {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .app + .app {
    border-top: 1px solid var(--border);
  }

  .line {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .label {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .label strong {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 600;
  }

  .label code {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .pill {
    height: 18px;
    display: inline-flex;
    align-items: center;
    padding: 0 7px;
    border-radius: 3px;
    background: var(--surface-3);
    color: var(--text-dim);
    font-size: 11.5px;
    font-weight: 500;
  }

  .x {
    width: 26px;
    height: 26px;
    flex: none;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 16px;
  }

  .x:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .sub {
    display: grid;
    grid-template-columns: 180px 1fr;
    gap: 8px;
    padding: 0 14px 12px;
  }

  .sub .field {
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .sub .mono {
    font-family: var(--mono);
  }

  .add {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  .pair button[aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }
</style>
