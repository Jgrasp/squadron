<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { FitAddon } from '@xterm/addon-fit'
  import { Terminal } from '@xterm/xterm'
  import '@xterm/xterm/css/xterm.css'
  import { home } from './projects.svelte'
  import SidePanel from './SidePanel.svelte'
  import { attach, close, detach, ofProject, sessions, start } from './terminals.svelte'
  import { t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'

  let { dir, to, onsend, onclose }: { dir: string; to?: string; onsend?: (text: string) => void; onclose: () => void } = $props()

  const THEME = {
    background: '#110e0d',
    foreground: '#d8cdc6',
    cursor: '#ede6e1',
    cursorAccent: '#110e0d',
    selectionBackground: 'rgba(224, 138, 91, 0.35)',
    black: '#2e2724',
    red: '#f59e8c',
    green: '#9ad6cb',
    yellow: '#f0c987',
    blue: '#8db4e8',
    magenta: '#c9a6ff',
    cyan: '#7fd1d9',
    white: '#d8cdc6',
    brightBlack: '#6e625b',
    brightRed: '#f8bdb1',
    brightGreen: '#bfe7df',
    brightYellow: '#f7dccb',
    brightBlue: '#b3cdf0',
    brightMagenta: '#dcc5ff',
    brightCyan: '#a8e3e8',
    brightWhite: '#ffffff',
  }

  const list = $derived(ofProject(dir))
  const current = $derived(sessions.current[dir])

  let host: HTMLDivElement
  let term = $state<Terminal | null>(null)
  let fit: FitAddon
  let shown = 0
  let starting = false
  let selection = $state(false)

  const resize = () => shown && invoke('term_resize', { id: shown, cols: term!.cols, rows: term!.rows }).catch(() => {})

  $effect(() => {
    let disposed = false
    let created: Terminal | null = null
    const observer = new ResizeObserver(() => {
      if (!created) return
      fit.fit()
      resize()
    })
    document.fonts.load("12.5px 'DM Mono'").finally(() => {
      if (disposed) return
      created = new Terminal({
        fontFamily: "'DM Mono', monospace",
        fontSize: 12.5,
        lineHeight: 1.3,
        cursorBlink: true,
        theme: THEME,
        linkHandler: { activate: (_, url) => /^https?:\/\//.test(url) && invoke('open_link', { url }) },
      })
      fit = new FitAddon()
      created.loadAddon(fit)
      created.open(host)
      fit.fit()
      created.onData((data) => shown && invoke('term_write', { id: shown, data }).catch(() => {}))
      created.onSelectionChange(() => (selection = created!.hasSelection()))
      created.onTitleChange((title) => {
        const session = sessions.list.find((s) => s.id === shown)
        if (session && title.trim()) session.title = title.trim()
      })
      observer.observe(host)
      term = created
    })
    return () => {
      disposed = true
      observer.disconnect()
      if (shown) detach(shown)
      created?.dispose()
    }
  })

  $effect(() => {
    if (!term) return
    const id = current
    if (id && id !== shown) {
      if (shown) detach(shown)
      shown = id
      term.reset()
      term.write(attach(id, (data) => term?.write(data)))
      resize()
      term.focus()
    } else if (!id && !starting) {
      starting = true
      start(dir, term.cols, term.rows).finally(() => (starting = false))
    }
  })

  function quit(id: number) {
    if (list.length === 1) onclose()
    close(id)
  }
</script>

<SidePanel kind="terminal" title={t('shell.terminal')} {onclose}>
  {#snippet sub()}<Trans k="shell.yourShell">{#snippet path()}<span class="mono">{home(dir)}</span>{/snippet}</Trans>{/snippet}
  {#snippet actions()}
    <div class="sessions" role="tablist" aria-label={t('shell.sessions')}>
      {#each list as session (session.id)}
        <span class="session" class:ended={session.ended}>
          <button role="tab" aria-selected={session.id === current} title={session.title} onclick={() => (sessions.current[dir] = session.id)}>
            {session.title}
          </button>
          <button class="quit" aria-label={t('shell.closeNamed', { name: session.title })} title={t('shell.closeSession')} onclick={() => quit(session.id)}>×</button>
        </span>
      {/each}
      <button class="new" aria-label={t('shell.newSession')} title={t('shell.newSession')} onclick={() => term && start(dir, term.cols, term.rows)}>+</button>
    </div>
  {/snippet}
  <div class="screen"><div class="host" bind:this={host}></div></div>
  <div class="foot">
    <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9" /><path d="M12 11v5M12 8h.01" /></svg>
    <span>{t('shell.terminalNote')}</span>
    {#if to && onsend}
      <button class="btn btn-secondary" disabled={!selection} onclick={() => term && onsend(term.getSelection())}>
        {t('shell.sendSelection', { name: to })}
      </button>
    {/if}
  </div>
</SidePanel>

<style>
  .mono {
    font-family: var(--mono);
  }

  .sessions {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }

  .session {
    position: relative;
    display: flex;
    min-width: 0;
  }

  .session [role='tab'] {
    max-width: 160px;
    height: 30px;
    padding: 0 22px 0 10px;
    overflow: hidden;
    border-radius: 4px;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--text-muted);
    font: 12px var(--mono);
  }

  .session [role='tab'][aria-selected='true'] {
    background: var(--menu);
    box-shadow: inset 0 0 0 1px #4e443e;
    color: var(--text-strong);
  }

  .session.ended [role='tab'] {
    color: var(--text-faint);
    text-decoration: line-through;
  }

  .quit {
    position: absolute;
    top: 5px;
    right: 3px;
    width: 18px;
    height: 20px;
    border-radius: 3px;
    color: var(--text-dim);
    opacity: 0;
  }

  .session:hover .quit,
  .quit:focus-visible,
  [aria-selected='true'] + .quit {
    opacity: 1;
  }

  .quit:hover {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .new {
    width: 30px;
    height: 30px;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 16px;
  }

  .new:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }

  .screen {
    flex: 1;
    min-height: 0;
    padding: 14px 6px 6px 18px;
    background: #110e0d;
  }

  .host {
    width: 100%;
    height: 100%;
  }

  .host :global(.xterm-viewport) {
    background: #110e0d !important;
  }

  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-sidebar);
    font-size: 12.5px;
  }

  .foot svg {
    flex: none;
    fill: none;
    stroke: var(--text-dim);
    stroke-width: 2;
    stroke-linecap: round;
  }

  .foot span {
    flex: 1;
    color: var(--text-dim);
  }

  .foot .btn {
    height: 30px;
    padding: 0 12px;
    font-size: 12.5px;
  }
</style>
