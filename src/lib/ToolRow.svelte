<script lang="ts">
  import { counts, EDITS } from './diff'
  import { toolLabel } from './agents.svelte'
  import { t, type Key } from './i18n/index.svelte'
  import { argument, type Agent, type ToolCall } from './stream'

  let { agent, call, ondiff }: { agent: Agent; call: ToolCall; ondiff: (call: ToolCall) => void } = $props()

  const arg = $derived(argument(agent, call))
  const failed = $derived(!!call.result?.error)
  const edit = $derived(EDITS.includes(call.name) && !failed)
  const changes = $derived(counts(call))
  const output = $derived.by(() => {
    const data = call.result?.data
    if (call.name === 'Bash' && data && typeof data.stdout === 'string')
      return [data.stdout, data.stderr].filter(Boolean).join('\n').trim()
    return call.result?.text.trim() ?? ''
  })
  const code = $derived.by((): Key | '' => {
    if (call.name !== 'Bash' || !call.result) return ''
    if (call.result.data?.interrupted) return 'conversation.exitInterrupted'
    return failed ? 'conversation.exitFailed' : 'conversation.exitOk'
  })
</script>

{#if edit}
  <button class="row action" title={t('conversation.openInDiff')} onclick={() => ondiff(call)} disabled={!call.result}>
    <span class="name">{toolLabel(call.name)}</span>
    <span class="arg">{arg}</span>
    {#if call.result}
      <span class="mono"><span class="add">+{changes.add}</span>{#if changes.del}&nbsp;<span class="del">−{changes.del}</span>{/if}</span>
      <span class="go">{t('conversation.diffLink')}</span>
    {/if}
  </button>
{:else if output && (call.name === 'Bash' || failed)}
  <details class="row-box" class:failed open={failed}>
    <summary class="row">
      {#if failed}
        <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true"><path d="M5 5l14 14M19 5L5 19" /></svg>
      {/if}
      <span class="name">{toolLabel(call.name)}</span>
      <span class="arg">{arg}</span>
      {#if code}<span class="code" class:ok={code === 'conversation.exitOk'}>{t(code)}</span>{/if}
      <span class="go">{t('conversation.output')}</span>
    </summary>
    <pre>{output}</pre>
  </details>
{:else}
  <div class="row">
    <span class="name">{toolLabel(call.name)}</span>
    <span class="arg">{arg}</span>
    {#if !call.result}<span class="go">…</span>{/if}
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 30px;
    padding: 0 8px;
    margin: 0 -8px;
    border-radius: 4px;
    text-align: left;
    list-style: none;
    font-size: 12.5px;
  }

  .row::-webkit-details-marker {
    display: none;
  }

  button.row {
    width: calc(100% + 16px);
  }

  .action:not(:disabled):hover,
  summary.row:hover {
    background: var(--menu);
  }

  .name {
    min-width: 40px;
    flex: none;
    white-space: nowrap;
    color: var(--text-dim);
  }

  .arg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
  }

  .mono {
    font-family: var(--mono);
  }

  .add {
    color: var(--ok);
  }

  .del {
    color: var(--err);
  }

  .go {
    color: var(--text-dim);
  }

  .action:hover .go {
    color: var(--text);
  }

  .code {
    color: var(--text-dim);
  }

  .code.ok {
    color: var(--done-text);
  }

  .row-box {
    margin: 0 -8px;
    border-radius: 4px;
  }

  .row-box > .row {
    margin: 0;
  }

  .failed {
    background: var(--err-bg);
    box-shadow: inset 0 0 0 1px #6b3a33;
  }

  .failed .name,
  .failed .code,
  .failed svg {
    color: var(--err-text);
  }

  .failed .arg {
    color: #f8d5cc;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
  }

  pre {
    max-height: 320px;
    overflow: auto;
    margin: 0 8px 8px;
    padding: 8px 10px;
    border-radius: 3px;
    background: var(--bg-main);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-3);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .failed pre {
    background: var(--err-pre);
    color: var(--err-text);
  }
</style>
