<script lang="ts">
  import { ui } from './actions.svelte'
  import { reset } from './layout-slots.svelte'
  import { t } from './i18n/index.svelte'
</script>

<div class="bar" role="status">
  <span class="dot" aria-hidden="true"></span>
  <span class="text">{t('settings.customizing')}</span>
  <button class="btn btn-secondary" onclick={() => reset()}>{t('settings.resetAll')}</button>
  <button class="btn btn-secondary" onclick={() => ((ui.customizing = false), (ui.settings = 'layout'))}>{t('settings.layoutSettings')}</button>
  <button class="btn btn-primary" onclick={() => (ui.customizing = false)}>{t('settings.done')}</button>
</div>

<style>
  .bar {
    position: fixed;
    left: 50%;
    top: 124px;
    z-index: 40;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: calc(100vw - 40px);
    padding: 8px 8px 8px 16px;
    border-radius: 8px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--wait-border),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 4px;
    background: var(--accent);
  }

  .text {
    font-size: 12.5px;
    color: var(--text-2);
  }

  .btn {
    flex: none;
    height: 32px;
    padding: 0 12px;
  }
</style>
