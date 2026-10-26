<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { agents, importSession, knownSessions, type Elsewhere } from './agents.svelte'
  import Dialog from './Dialog.svelte'
  import { date, errorText, number, t } from './i18n/index.svelte'
  import type { Project } from './projects.svelte'

  let { project, onclose }: { project: Project; onclose: () => void } = $props()

  let found = $state<Elsewhere[] | null>(null)
  let search = $state('')
  let hideArchived = $state(true)
  let chosen = $state<string[]>([])
  let importing = $state<number | null>(null)
  let errors = $state<string[]>([])

  async function load() {
    const [all, known] = await Promise.all([invoke<Elsewhere[]>('project_sessions', { project: project.path }), knownSessions()])
    found = all.filter((f) => !known.has(f.id))
  }
  load()

  const shown = $derived(
    (found ?? []).filter(
      (f) => (!hideArchived || !f.archived) && `${f.title} ${f.id}`.toLowerCase().includes(search.trim().toLowerCase()),
    ),
  )
  const archivedCount = $derived((found ?? []).filter((f) => f.archived).length)
  const when = (at: number) => date(at, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' })
  const size = (bytes: number) =>
    bytes > 1024 * 1024
      ? t('board.megabytes', { n: number(bytes / 1024 / 1024, { minimumFractionDigits: 1, maximumFractionDigits: 1 }) })
      : t('board.kilobytes', { n: Math.max(1, Math.round(bytes / 1024)) })
  const place = (f: Elsewhere) =>
    f.cwd && f.cwd !== project.path ? (f.here ? t('board.inFolder', { folder: f.cwd.split('/').pop() ?? '' }) : t('board.folderGone')) : ''
  const toggle = (id: string) => (chosen = chosen.includes(id) ? chosen.filter((c) => c !== id) : [...chosen, id])
  const allShown = $derived(shown.length > 0 && shown.every((f) => chosen.includes(f.id)))

  async function run() {
    const picked = (found ?? []).filter((f) => chosen.includes(f.id))
    errors = []
    let last = ''
    for (const [i, f] of picked.entries()) {
      importing = i + 1
      try {
        last = (await importSession(project.path, f)).id
      } catch (e) {
        errors.push(t('board.importError', { title: f.title, error: errorText(e) }))
      }
    }
    importing = null
    if (last) agents.current[project.path] = last
    if (errors.length) {
      chosen = []
      load()
    } else onclose()
  }
</script>

<Dialog title={t('board.importTitle', { project: project.name })} full {onclose}>
  <div class="import">
    <p class="intro">{t('board.importIntro', { project: project.name })}</p>
    <div class="bar">
      <input class="field" type="search" placeholder={found ? t('board.filterConversations', { count: found.length }) : t('board.filterConversationsLoading')} bind:value={search} />
      {#if archivedCount}
        <label class="check"><input type="checkbox" bind:checked={hideArchived} /> {t('board.hideArchived', { count: archivedCount })}</label>
      {/if}
      <button class="btn btn-secondary" disabled={!shown.length} onclick={() => (chosen = allShown ? [] : [...new Set([...chosen, ...shown.map((f) => f.id)])])}>
        {allShown ? t('board.unselectAll') : t('board.selectAll')}
      </button>
    </div>
    <ul>
      {#each shown as f (f.id)}
        <li>
          <label class:on={chosen.includes(f.id)}>
            <input type="checkbox" checked={chosen.includes(f.id)} onchange={() => toggle(f.id)} />
            <span class="what">
              <strong>{f.title}</strong>
              <small>
                {when(f.updated)} · {size(f.size)} · {f.app ? t('board.claudeApp') : 'Claude Code'}{#if place(f)}&nbsp;· {place(f)}{/if}{#if f.archived}&nbsp;· {t('board.archivedInApp')}{/if}
              </small>
            </span>
          </label>
        </li>
      {:else}
        <p class="empty">{found ? t('board.noOtherConversation') : t('board.lookingForConversations')}</p>
      {/each}
    </ul>
    {#if errors.length}
      <div class="bad" role="alert">{#each errors as e (e)}<p>{e}</p>{/each}</div>
    {/if}
    <footer>
      <span class="count">{chosen.length ? t('board.selected', { count: chosen.length }) : t('board.selectToImport')}</span>
      <button class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!chosen.length || importing !== null} onclick={run}>
        {importing !== null ? t('board.importing', { n: importing, total: chosen.length }) : chosen.length ? t('board.importCount', { count: chosen.length }) : t('board.import')}
      </button>
    </footer>
  </div>
</Dialog>

<style>
  .import {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 14px;
    padding: 16px 24px 0;
    border-top: 1px solid var(--border);
  }

  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--text-dim);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .bar .field {
    flex: 1;
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
    white-space: nowrap;
  }

  ul {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li label {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 6px;
    cursor: pointer;
  }

  li label:hover {
    background: var(--surface-1);
  }

  li label.on {
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .what {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .what strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .what small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .empty {
    margin: 0;
    padding: 12px;
    color: var(--text-dim);
  }

  .bad {
    font-size: 12.5px;
    color: var(--err-text);
  }

  .bad p {
    margin: 0;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 0 18px;
    border-top: 1px solid var(--border);
  }

  .count {
    flex: 1;
    font-size: 13px;
    color: var(--text-dim);
  }
</style>
