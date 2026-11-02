<script lang="ts">
  import { clock } from './agents.svelte'
  import { duration, modelLabel, tokens } from './format'
  import { t } from './i18n/index.svelte'
  import Markdown from './Markdown.svelte'
  import type { Advice } from './stream'

  let { advice, model }: { advice: Advice; model?: string } = $props()

  const live = $derived(!advice.result)
  const failed = $derived(advice.result?.kind === 'error')
  const ms = $derived(advice.ms ?? (live && advice.at ? Math.max(0, clock.now - advice.at) : undefined))
  const state = $derived(live ? t('conversation.advisorReading') : failed ? t('conversation.advisorUnavailable') : t('conversation.advisorGave'))
  const meta = $derived(
    [
      advice.input !== undefined ? t('conversation.advisorTokens', { input: tokens(advice.input), output: tokens(advice.output ?? 0) }) : '',
      ms !== undefined ? duration(ms) : '',
      failed ? advice.result?.code : '',
    ]
      .filter(Boolean)
      .join(' · '),
  )
</script>

<div class="advice" class:live class:failed role="note">
  <div class="head">
    {#if live}<span class="pulse" aria-hidden="true"></span>{/if}
    <span class="origin">{t('conversation.advisor')}</span>
    {#if model && model !== 'none'}<span class="model">{modelLabel(model)}</span><span class="dot" aria-hidden="true">·</span>{/if}
    <span class="state">{state}</span>
    {#if meta}<span class="meta">{meta}</span>{/if}
  </div>
  {#if live}
    <div class="scan" aria-hidden="true"></div>
  {:else if advice.result?.kind === 'text' && advice.result.text}
    <div class="body"><Markdown text={advice.result.text} /></div>
  {:else if advice.result?.kind === 'redacted'}
    <p class="sub">
      <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>
      {t('conversation.advisorSealed')}
    </p>
  {:else if failed}
    <p class="sub">{t('conversation.advisorWithout')}</p>
  {/if}
</div>

<style>
  .advice {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 14px;
    border-left: 2px solid var(--advisor);
    border-radius: 0 6px 6px 0;
    background: var(--advisor-bg);
  }

  .advice.live {
    background: linear-gradient(90deg, rgba(195, 180, 240, 0.13), rgba(195, 180, 240, 0.03) 70%);
  }

  .advice.failed {
    border-left-color: var(--err);
    background: var(--err-bg);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 12.5px;
    color: var(--advisor-text);
  }

  .failed .head,
  .failed .origin {
    color: var(--err-text);
  }

  .origin {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--advisor);
  }

  .model {
    font-weight: 600;
  }

  .dot {
    color: var(--text-faint);
  }

  .meta {
    margin-left: auto;
    font: 11.5px var(--mono);
    white-space: nowrap;
    color: var(--text-dim);
  }

  .body {
    font-size: 13px;
    color: var(--text-2);
  }

  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .pulse {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--advisor);
    animation: ring 1.4s infinite;
  }

  .scan {
    height: 2px;
    border-radius: 2px;
    background: linear-gradient(90deg, transparent, var(--advisor), transparent) no-repeat;
    background-size: 40% 100%;
    opacity: 0.8;
    animation: scan 1.8s ease-in-out infinite;
  }

  @keyframes ring {
    0% {
      box-shadow: 0 0 0 0 rgba(195, 180, 240, 0.55);
    }
    80%,
    100% {
      box-shadow: 0 0 0 7px rgba(195, 180, 240, 0);
    }
  }

  @keyframes scan {
    0% {
      background-position: -40% 0;
    }
    100% {
      background-position: 140% 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulse,
    .scan {
      animation: none;
    }
  }
</style>
