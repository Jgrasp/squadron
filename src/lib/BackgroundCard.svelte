<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { clock } from './agents.svelte'
  import { duration } from './format'
  import { t } from './i18n/index.svelte'
  import { calls, type Agent } from './stream'

  let { agent, onterminal }: { agent: Agent; onterminal: (command: string) => void } = $props()

  const started = $derived(Math.min(...agent.background.map((b) => b.started)))
  const callOf = (id: string) => calls(agent).find((c) => c.id === id)
  const outputOf = (id: string) =>
    /^\s*sleep\s+\S+\s*$/.test(callOf(id)?.input?.command ?? '')
      ? undefined
      : /Output is being written to: (\S+?)\.?(\s|$)/.exec(callOf(id)?.result?.text ?? '')?.[1]
</script>

<section class="background" aria-label={t('conversation.inBackground')}>
  <div class="head">
    <span class="icon" aria-hidden="true">
      <svg class="slow" width="16" height="16" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" stroke-dasharray="4 5" /></svg>
    </span>
    <span class="what">
      <strong>{t('conversation.inBackground')}</strong>
      <small>{t('conversation.resumesAlone')}</small>
    </span>
    <span class="mono">{duration(clock.now - started)}</span>
  </div>
  {#each agent.background as task (task.id)}
    {@const output = outputOf(task.toolUseId)}
    <div class="task">
      <span class="name">
        <span>{task.name}</span>
        <span class="command">{callOf(task.toolUseId)?.input?.command ?? ''}</span>
      </span>
      <span class="mono">{duration(clock.now - task.started)}</span>
      {#if output}<button class="link" onclick={() => onterminal(`tail -f '${output}'\n`)}>{t('conversation.viewOutput')}</button>{/if}
      <button class="btn btn-secondary" onclick={() => invoke('stop_task', { id: agent.id, taskId: task.id })}>{t('conversation.stopTask')}</button>
    </div>
  {/each}
</section>

<style>
  .background {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--background-line);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    flex: none;
    width: 30px;
    height: 30px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--background-bg);
  }

  .icon svg {
    fill: none;
    stroke: var(--background);
    stroke-width: 3.5;
  }

  .slow {
    animation: spin 6s linear infinite;
  }

  .what,
  .name {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  small {
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .mono {
    flex: none;
    font: 12px var(--mono);
    color: var(--text-muted);
  }

  .task {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .command {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .link {
    flex: none;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-link);
  }

  .link:hover {
    color: var(--accent-soft);
  }

  .btn {
    height: 28px;
    padding: 0 12px;
  }
</style>
