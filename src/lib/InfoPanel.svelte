<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { persist } from './agents.svelte'
  import Icon from './Icon.svelte'
  import { panelDocument, themeOf } from './infopanel'
  import type { Agent } from './stream'

  let { agent }: { agent: Agent } = $props()

  let host = $state<HTMLElement>()
  const panel = $derived(agent.infoPanel!)
  const doc = $derived(host ? panelDocument(panel.html, themeOf(host)) : '')

  function toggle() {
    panel.hidden = !panel.hidden
    persist(agent)
  }
</script>

<div class="info-panel" class:hidden={panel.hidden} bind:this={host}>
  {#if !panel.hidden}
    <iframe title={t('agent.panelOf', { name: agent.name })} sandbox="" referrerpolicy="no-referrer" srcdoc={doc} style:height="{panel.height}px" tabindex="-1"></iframe>
  {:else}
    <span class="label">{agent.name}’s panel is hidden</span>
  {/if}
  <button class="toggle" title={panel.hidden ? t('agent.panelShowTitle') : t('agent.panelHideTitle')} aria-label={panel.hidden ? t('agent.panelShow') : t('agent.panelHide')} onclick={toggle}>
    <Icon name={panel.hidden ? 'eye' : 'eye-off'} size={14} />
  </button>
</div>

<style>
  .info-panel {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
    background: var(--bg-main);
  }

  iframe {
    display: block;
    flex: 1;
    min-width: 0;
    border: 0;
    pointer-events: none;
  }

  .hidden {
    height: 28px;
    padding-left: 20px;
  }

  .label {
    flex: 1;
    font-size: 12px;
    color: var(--text-faint);
  }

  .toggle {
    position: absolute;
    top: 6px;
    right: 10px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    color: var(--text-faint);
  }

  .hidden .toggle {
    top: 2px;
  }

  .toggle:hover {
    background: var(--menu);
    color: var(--text);
  }
</style>
