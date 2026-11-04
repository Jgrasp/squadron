<script lang="ts">
  import { displayOf, setDisplay, type PanelKind } from './layout.svelte'
  import { MODES, modeHint, modeLabel, modeShort } from './panels'
  import { t } from './i18n/index.svelte'
  import ModeIcon from './ModeIcon.svelte'

  let { kind, windowed = false, onclose }: { kind: PanelKind; windowed?: boolean; onclose: () => void } = $props()

  const mode = $derived(displayOf(kind).mode)
  let open = $state(false)
  let root = $state<HTMLElement>()
  let trigger = $state<HTMLButtonElement>()

  function keys(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !open) return
    event.preventDefault()
    event.stopPropagation()
    open = false
    trigger?.focus()
  }
</script>

<svelte:window onclick={(e) => open && !root?.contains(e.target as Node) && (open = false)} />

{#if !windowed}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="place" bind:this={root} onkeydown={keys}>
    <button class="trigger" aria-label={t('shell.panelPlacement')} aria-expanded={open} title={t(modeLabel[mode])} bind:this={trigger} onclick={() => (open = !open)}>
      <ModeIcon {mode} />
      <span>{t(modeShort[mode])}</span>
      <svg class="glyph chevron" width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M6 9l6 6 6-6" /></svg>
    </button>
    {#if open}
      <div class="menu" role="group" aria-label={t('shell.panelPlacement')}>
        {#each MODES as each, i (each)}
          {#if i === 2}<div class="sep"></div>{/if}
          <button
            class="option"
            aria-pressed={mode === each}
            onclick={() => {
              setDisplay(kind, { mode: each })
              open = false
            }}
          >
            <ModeIcon mode={each} size={60} />
            <span class="text">
              <b>{t(modeLabel[each])}</b>
              <span>{t(modeHint[each])}</span>
            </span>
            {#if mode === each}<svg class="glyph check" width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>{/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}
<button class="icon" aria-label={t('shell.closePanel')} title={t('shell.closePanel')} onclick={onclose}>×</button>

<style>
  .place {
    position: relative;
    flex: none;
  }

  .trigger {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
    white-space: nowrap;
  }

  .trigger:hover,
  .trigger[aria-expanded='true'] {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .trigger :global(.mode) {
    color: var(--accent);
  }

  .glyph {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .chevron {
    opacity: 0.7;
  }

  .menu {
    position: absolute;
    z-index: 30;
    top: calc(100% + 6px);
    right: 0;
    width: 340px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 8px;
    border-radius: 4px;
    text-align: left;
  }

  .option:hover,
  .option:focus-visible {
    background: var(--menu-hover);
    outline: none;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .text b {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
  }

  .text span {
    font-size: 12px;
    color: var(--text-dim);
  }

  .option[aria-pressed='true'] b {
    color: var(--accent-soft);
  }

  .check {
    color: var(--accent-soft);
  }

  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }

  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }

  .icon:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }
</style>
