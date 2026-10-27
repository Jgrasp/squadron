<script lang="ts">
  import { fly } from 'svelte/transition'
  import { news, openChangelog } from './update.svelte'
  import { t } from './i18n/index.svelte'
</script>

{#if news.version}
  <div class="news" role="status" transition:fly={{ y: 12, duration: 180 }}>
    <span class="triangle" aria-hidden="true"></span>
    <button class="open" onclick={openChangelog}>
      <strong>{t('settings.newsTitle', { version: news.version })}</strong>
      <small>{t('settings.newsOpen')}</small>
    </button>
    <button class="close" aria-label={t('settings.newsDismiss')} onclick={() => (news.version = '')}>×</button>
  </div>
{/if}

<style>
  .news {
    position: fixed;
    right: 16px;
    bottom: 40px;
    z-index: 50;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 8px 10px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.35);
  }

  .triangle {
    width: 16px;
    height: 16px;
    background: var(--accent);
    clip-path: polygon(50% 0, 100% 100%, 0 100%);
  }

  .open {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text-strong);
    text-align: left;
    cursor: pointer;
  }

  .open small {
    color: var(--accent);
  }

  .open:hover small {
    text-decoration: underline;
  }

  .close {
    width: 24px;
    height: 24px;
    background: none;
    border: 0;
    border-radius: 6px;
    color: var(--text-dim);
    font-size: 16px;
    cursor: pointer;
  }

  .close:hover {
    background: var(--surface-3);
    color: var(--text);
  }
</style>
