<script lang="ts">
  import { clock } from './agents.svelte'
  import { duration } from './format'
  import { t } from './i18n/index.svelte'
  import { inBackground, type Agent } from './stream'

  let { agent, label }: { agent: Agent; label: string } = $props()

  let at = $state({ x: 0, y: 0 })
  let open = $state(false)

  function show(e: PointerEvent) {
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect()
    at = { x: box.left - 8, y: box.bottom + 6 }
    open = true
  }
</script>

<span class="label" role="note" onpointerenter={show} onpointerleave={() => (open = false)}>{label}</span>
<span class="tip" class:open role="tooltip" style:left="{at.x}px" style:top="{at.y}px">
  <span class="title">{t('conversation.inBackground')}</span>
  {#each inBackground(agent) as task (task.id)}
    <span class="task"><span class="dot" aria-hidden="true"></span><span class="name">{task.name}</span><span class="time">{duration(clock.now - task.started)}</span></span>
  {/each}
</span>

<style>
  .label {
    color: var(--background);
    text-decoration: underline dotted var(--background-line);
    text-underline-offset: 3px;
  }

  .tip {
    position: fixed;
    z-index: 50;
    min-width: 220px;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--surface-3);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 8px 24px rgba(0, 0, 0, 0.45);
    pointer-events: none;
    white-space: normal;
    opacity: 0;
    visibility: hidden;
    transform: translateY(-4px) scale(0.98);
    transform-origin: top left;
    transition:
      opacity 0.16s ease,
      transform 0.16s ease,
      visibility 0s linear 0.16s;
  }

  .tip.open {
    opacity: 1;
    visibility: visible;
    transform: none;
    transition:
      opacity 0.16s ease 0.12s,
      transform 0.16s ease 0.12s,
      visibility 0s 0.12s;
  }

  .title {
    font: 500 10.5px var(--mono);
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .task {
    display: flex;
    align-items: center;
    gap: 6px;
    font: 12px var(--sans);
    color: var(--text-2);
  }

  .dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--background);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .time {
    margin-left: auto;
    padding-left: 12px;
    font: 11px var(--mono);
    color: var(--text-faint);
  }
</style>
