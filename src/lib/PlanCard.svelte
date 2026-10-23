<script lang="ts">
  import { plan } from './agents.svelte'
  import Markdown from './Markdown.svelte'
  import type { Agent, Ask } from './stream'

  let { agent, ask }: { agent: Agent; ask: Ask } = $props()
</script>

<section class="card" aria-label="Plan approval">
  <div class="head">
    <span class="icon" aria-hidden="true">
      <svg width="16" height="16" viewBox="0 0 24 24"><path d="M9 6h11M9 12h11M9 18h11M4 6h.01M4 12h.01M4 18h.01" /></svg>
    </span>
    <span class="what">
      <strong>Approve the plan?</strong>
      <span>{agent.name} has not changed anything yet. It waits for your go-ahead.</span>
    </span>
  </div>
  <div class="plan"><Markdown text={String(ask.input.plan ?? '')} /></div>
  <div class="actions">
    <button class="btn go" onclick={() => plan(agent, ask, 'edits')}>Start · accept edits</button>
    <button class="btn ask" onclick={() => plan(agent, ask, 'ask')}>Start · ask for each edit</button>
    <button class="btn more" onclick={() => plan(agent, ask, 'continue')}>Keep planning</button>
  </div>
  <span class="hint">To adjust the plan, write your feedback below.</span>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    width: 30px;
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
  }

  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }

  .what {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .what strong {
    font-weight: 600;
    color: var(--wait-text);
  }

  .what span {
    font-size: 12.5px;
    color: var(--wait-text-2);
  }

  .plan {
    max-height: 420px;
    overflow: auto;
    padding: 14px 16px;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px #4a2e29;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .go {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .ask {
    background: var(--wait-btn);
    color: var(--wait-btn-text);
  }

  .more {
    background: var(--surface-3);
    color: #e3d7cf;
  }

  .hint {
    font-size: 12.5px;
    color: var(--wait-text-2);
  }
</style>
