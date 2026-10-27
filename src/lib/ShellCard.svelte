<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { FitAddon } from '@xterm/addon-fit'
  import { Terminal } from '@xterm/xterm'
  import '@xterm/xterm/css/xterm.css'
  import { clock } from './agents.svelte'
  import { duration } from './format'
  import { t } from './i18n/index.svelte'
  import { attach, detach, stop, type Shell } from './shells.svelte'

  let { entry, onreplay, onterminal }: { entry: Shell; onreplay: (command: string) => void; onterminal: (command: string) => void } = $props()

  const running = $derived(entry.state === 'running')
  const failed = $derived(entry.state === 'stopped' || !!entry.code)
  let open = $state(false)
  let host = $state<HTMLDivElement>()
  const ROWS = 14

  $effect(() => {
    open = running || failed
  })

  $effect(() => {
    const id = entry.pty
    if (!host || !id || !open) return
    const term = new Terminal({
      fontFamily: "'DM Mono', monospace",
      fontSize: 12,
      lineHeight: 1.3,
      rows: 3,
      theme: { background: '#110e0d', foreground: '#d8cdc6', cursor: '#ede6e1', cursorAccent: '#110e0d', selectionBackground: 'rgba(224, 138, 91, 0.35)' },
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(host)
    const resize = () => {
      const used = term.buffer.active.baseY + term.buffer.active.cursorY + 1
      const rows = Math.min(ROWS, Math.max(term.rows, used))
      const cols = fit.proposeDimensions()?.cols ?? term.cols
      if (rows === term.rows && cols === term.cols) return
      term.resize(cols, rows)
      invoke('term_resize', { id, cols, rows }).catch(() => {})
    }
    resize()
    term.write(attach(id, (data) => term.write(data, resize)), resize)
    term.onData((data) => invoke('term_write', { id, data }).catch(() => {}))
    const observer = new ResizeObserver(resize)
    observer.observe(host)
    return () => {
      observer.disconnect()
      detach(id)
      term.dispose()
    }
  })

  const elapsed = $derived(entry.ms ?? Math.max(0, clock.now - entry.at))
</script>

<div class="shell" class:running class:failed class:ok={!running && !failed}>
  <div class="head">
    <button type="button" class="toggle" aria-expanded={open} onclick={() => (open = !open)}>
      <span class="chevron" aria-hidden="true">›</span>
      <span class="command">{entry.command}</span>
    </button>
    <span class="time">{duration(elapsed)}</span>
    {#if running}
      <span class="spin" role="status" aria-label={t('shell.commandRunning')}></span>
      <button type="button" class="icon stop" aria-label={t('shell.stopCommand')} title={t('shell.stopCommand')} onclick={() => stop(entry)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" stroke="none" /></svg>
      </button>
    {:else}
      <button type="button" class="icon" aria-label={t('shell.replayCommand')} title={t('shell.replayCommand')} onclick={() => onreplay(entry.command)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M20 12a8 8 0 1 1-2.3-5.7" /><path d="M20 4v5h-5" /></svg>
      </button>
      <button type="button" class="icon" aria-label={t('shell.openInTerminal')} title={t('shell.openInTerminal')} onclick={() => onterminal(entry.command)}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M7 9l3 3-3 3M12.5 15H17" /></svg>
      </button>
    {/if}
  </div>
  {#if open}
    {#if entry.pty}
      <div class="screen"><div class="host" bind:this={host}></div></div>
    {:else if entry.output}
      <pre class="out">{entry.output}</pre>
    {/if}
  {/if}
</div>

<style>
  .shell {
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface-1);
    overflow: hidden;
  }

  .shell.ok {
    border-color: color-mix(in srgb, var(--ok) 55%, transparent);
  }

  .shell.failed {
    border-color: color-mix(in srgb, var(--err) 60%, transparent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--err) 10%, transparent);
  }

  .shell.running {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 6px 4px;
    font-size: 12.5px;
  }

  .toggle {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 6px;
    text-align: left;
  }

  .chevron {
    color: var(--text-faint);
    transition: transform 0.15s;
  }

  [aria-expanded='true'] .chevron {
    transform: rotate(90deg);
  }

  .command {
    font-family: var(--mono);
    color: var(--text-strong);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .command::before {
    content: '! ';
    color: var(--accent);
    font-weight: 700;
  }

  .time {
    color: var(--text-faint);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }

  .spin {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .icon {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-faint);
  }

  .icon:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .icon svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon.stop {
    color: var(--err);
  }

  .screen {
    padding: 8px 6px 6px 12px;
    background: #110e0d;
    border-top: 1px solid var(--border);
  }

  .host :global(.xterm-viewport) {
    background: #110e0d !important;
  }

  .out {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px 14px;
    background: #110e0d;
    border-top: 1px solid var(--border);
    color: #d8cdc6;
    font: 12px/1.5 var(--mono);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
