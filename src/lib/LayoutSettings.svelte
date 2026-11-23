<script lang="ts">
  import { ui } from './actions.svelte'
  import { displayOf, layout, resetDisplay, setDisplay, setFileTree, setSplit, type PanelSize } from './layout.svelte'
  import { reset } from './layout-slots.svelte'
  import { MODES, modeLabel, modeShort, PANELS } from './panels'
  import { errorText, t } from './i18n/index.svelte'
  import Icon from './Icon.svelte'
  import ModeIcon from './ModeIcon.svelte'
  import { detect } from './OpenWith.svelte'
  import Select from './Select.svelte'
  import { saveSettings, settings, type App } from './settings.svelte'
  import SlotList from './SlotList.svelte'
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

<p class="hint">{t('settings.layoutIntro')}</p>
<h3>
  <span>{t('settings.layoutHeader')}</span>
  <button class="btn btn-ghost" onclick={() => ((ui.settings = null), (ui.customizing = true))}>{t('settings.customizeInPlace')}</button>
  <button class="btn btn-ghost" onclick={() => (reset('agent.info'), reset('agent.tools'))}>{t('settings.backToDefault')}</button>
</h3>
<div class="two">
  <SlotList slot="agent.info" />
  <SlotList slot="agent.tools" />
</div>
<h3>
  <span>{t('settings.layoutMenus')}</span>
  <button class="btn btn-ghost" onclick={() => (reset('agent.menu'), reset('menu.squadron'), reset('artifact.actions'))}>{t('settings.backToDefault')}</button>
</h3>
<div class="two">
  <SlotList slot="agent.menu" />
  <div class="stack">
    <SlotList slot="menu.squadron" />
    <SlotList slot="artifact.actions" />
  </div>
</div>
<h3>
  <span>{t('settings.tabPanels')}</span>
  <button class="btn btn-ghost" onclick={resetDisplay}>{t('settings.backToDefault')}</button>
</h3>
<p class="hint">{t('settings.panelsIntro')}</p>
<div class="options">
  {#each PANELS as [kind, title] (kind)}
    {@const shown = displayOf(kind)}
    <div class="option">
      <strong class="name">{t(title)}</strong>
      <div class="segs" role="radiogroup" aria-label={t(title)}>
        {#each MODES as mode (mode)}
          <button role="radio" aria-checked={shown.mode === mode} title={t(modeLabel[mode])} onclick={() => setDisplay(kind, { mode })}>
            <ModeIcon {mode} />{t(modeShort[mode])}
          </button>
        {/each}
      </div>
      <div class="size">
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
    </div>
  {/each}
</div>
<h3><span>{t('shell.diff')}</span></h3>
<div class="options">
  <div class="option">
    <span class="label"><strong>{t('settings.diffLayout')}</strong><small>{t('settings.diffLayoutHint')}</small></span>
    <div class="segs" role="group" aria-label={t('settings.diffLayout')}>
      <button aria-pressed={!layout.split} onclick={() => setSplit(false)}>{t('review.unified')}</button>
      <button aria-pressed={layout.split} onclick={() => setSplit(true)}>{t('review.split')}</button>
    </div>
  </div>
  <label class="option">
    <span class="label"><strong>{t('review.fileTree')}</strong><small>{t('settings.fileTreeHint')}</small></span>
    <input type="checkbox" class="switch" checked={layout.fileTree} onchange={(e) => setFileTree(e.currentTarget.checked)} />
  </label>
</div>
<h3>
  <span>{t('settings.editorsTitle')}</span>
  {#if own}<button class="btn btn-ghost" onclick={back}>{t('settings.editorsBack')}</button>{/if}
</h3>
<section>
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
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    margin: 10px 0 -6px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  h3 span {
    flex: 1;
  }

  h3 .btn {
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
  }

  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    align-items: start;
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 10px;
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

  .option .name,
  .option .label {
    flex: 1;
    min-width: 0;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
  }

  .label small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .segs {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 6px;
    background: var(--surface-2);
  }

  .segs button {
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 12.5px;
  }

  .segs button:hover {
    color: var(--text);
  }

  .segs [aria-checked='true'],
  .segs [aria-pressed='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .size {
    width: 150px;
    flex: none;
  }

  .size :global(.trigger) {
    height: 32px;
  }

  .whole {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .switch {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .switch::before {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .switch:checked {
    background: var(--accent);
  }

  .switch:checked::before {
    transform: translateX(14px);
    background: var(--ink);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
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

</style>
