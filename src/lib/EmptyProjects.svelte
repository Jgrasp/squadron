<script lang="ts">
  import { t } from './i18n/index.svelte'

  let { onadd }: { onadd: () => void } = $props()
</script>

<main>
  <div class="box">
    <svg width="64" height="56" viewBox="0 0 64 56" aria-hidden="true">
      <path d="M32 4L60 52H4L32 4Z" fill="none" stroke="var(--accent)" stroke-width="3" stroke-linejoin="round" />
      <path d="M32 24L44 46H20L32 24Z" fill="var(--accent)" opacity="0.35" />
    </svg>
    <h1>{t('board.noProjects')}</h1>
    <p>{t('board.noProjectsText')}</p>
    <button class="btn btn-primary" onclick={onadd}>{t('board.addProject')}</button>
    <div class="drop">
      <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"
        ><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" /></svg
      >
      {t('board.dropFolder')}
    </div>
  </div>
</main>

<style>
  main {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px;
    background: var(--bg-main);
  }

  .box {
    width: 460px;
    max-width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    text-align: center;
  }

  h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.6;
    text-wrap: pretty;
  }

  .btn {
    height: 44px;
    padding: 0 22px;
  }

  .drop {
    width: 100%;
    height: 88px;
    margin-top: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    border: 1.5px dashed var(--border-dashed);
    border-radius: 6px;
    color: var(--text-dim);
  }

  .drop svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
</style>
