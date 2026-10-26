<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { untrack } from 'svelte'
  import { unshelve } from './agents.svelte'
  import Dialog from './Dialog.svelte'
  import ColorPick from './ColorPick.svelte'
  import { errorText, t } from './i18n/index.svelte'
  import Trans from './Trans.svelte'
  import { add, home, inspect, projects, type Inspection } from './projects.svelte'

  let { path: initial, onclose }: { path: string; onclose: () => void } = $props()

  let path = $state(untrack(() => initial))
  let name = $state('')
  let color = $state('')
  let info = $state<Inspection | null>(null)
  let error = $state('')
  let busy = $state(false)

  const problem = $derived.by(() => {
    if (!info || !path.trim()) return ''
    if (!info.exists) return t('board.folderMissing')
    if (info.taken_by && info.taken_archived) return ''
    if (info.taken_by) return t('board.folderTaken', { project: info.taken_by })
    return ''
  })
  const revive = $derived(!!info?.taken_by && info.taken_archived)
  const ready = $derived(!busy && (revive || (!!info?.exists && !info.taken_by && !!name.trim())))

  $effect(() => {
    const target = path.trim()
    if (!target) {
      info = null
      return
    }
    inspect(target).then((result) => {
      if (target !== path.trim()) return
      if (!name || name === info?.name) name = result.name
      info = result
    })
  })

  async function browse() {
    const chosen = await open({ directory: true, defaultPath: path || undefined })
    if (typeof chosen === 'string') path = chosen
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault()
    if (!ready) return
    busy = true
    try {
      if (revive) await unshelve(projects.archived.find((p) => p.name === info?.taken_by)?.path ?? path.trim())
      else await add(path.trim(), name, color)
      onclose()
    } catch (e) {
      error = errorText(e)
      busy = false
    }
  }

  function goTo(project: string) {
    projects.current = projects.list.find((p) => p.name === project)?.path ?? projects.current
    onclose()
  }
</script>

<Dialog title={t('board.addProject')} {onclose} width={520}>
  <form onsubmit={submit}>
    <p class="intro">{t('board.addProjectIntro')}</p>
    <div class="fields">
      <div class="group">
        <label for="folder">{t('board.projectFolder')}</label>
        <div class="row">
          <input
            id="folder"
            class="field mono"
            bind:value={path}
            placeholder={t('board.folderPlaceholder')}
            aria-invalid={!!problem}
            aria-describedby="folder-state"
            spellcheck="false"
          />
          <button type="button" class="btn btn-secondary browse" onclick={browse}>{t('common.browse')}</button>
        </div>
        <span id="folder-state" class="state">
          {#if problem}
            <span class="bad">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"
                ><circle cx="12" cy="12" r="9" /><path d="M12 8v5M12 16h.01" /></svg
              >
              {problem}
              {#if info?.taken_by}
                <button type="button" class="link" onclick={() => goTo(info!.taken_by!)}
                  >{t('board.goTo', { project: info.taken_by })}</button
                >
              {:else if info?.near}
                <span>{t('board.closestFolder')}</span>
                <button type="button" class="link mono" onclick={() => (path = info!.near!)}>{home(info.near)}</button>
              {/if}
            </span>
          {:else if revive}
            <span class="good">{t('board.folderArchived', { project: info?.taken_by ?? '' })}</span>
          {:else if info?.git}
            <span class="good">
              <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 12l5 5L20 6" /></svg>
              {#if info.branch}<Trans k="board.gitFoundBranch">{#snippet branch()}<code>{info?.branch}</code>{/snippet}</Trans>{:else}{t('board.gitFound')}{/if}
            </span>
          {:else if info}
            <span>{t('board.noGit')}</span>
          {/if}
        </span>
      </div>
      <div class="group">
        <label for="name">{t('common.name')}</label>
        <input id="name" class="field" bind:value={name} disabled={!info?.exists || !!info.taken_by} />
        <span class="state">{t('board.nameFromFolder')}</span>
      </div>
      {#if !revive}
        <div class="group">
          <span class="label">{t('board.color')}</span>
          <ColorPick value={color} label={t('board.colorLabel')} onpick={(picked) => (color = picked)} />
          <span class="state">{color ? t('board.colorChosen') : t('board.colorAuto')}</span>
        </div>
      {/if}
      {#if error}<p class="bad" role="alert">{error}</p>{/if}
    </div>
    <footer>
      <button type="button" class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!ready}>{revive ? t('board.unarchive', { project: info?.taken_by ?? '' }) : t('board.addProjectButton')}</button>
    </footer>
  </form>
</Dialog>

<style>
  .intro {
    margin: 0;
    padding: 0 22px 16px;
    color: var(--text-dim);
    font-size: 13px;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 4px 22px 20px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label,
  .label {
    font-weight: 600;
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .mono {
    font: 13px var(--mono);
  }

  .browse {
    height: 42px;
    background: var(--surface-4);
    color: var(--text);
    font-weight: 600;
  }

  .state {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .state > span {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .good {
    color: var(--done-text);
  }

  .bad {
    margin: 0;
    color: var(--err-text);
  }

  code {
    font-size: 12.5px;
  }

  svg {
    flex: none;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .link {
    color: var(--accent-link);
    text-decoration: underline;
  }

  .link.mono {
    font-family: var(--mono);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  footer .btn {
    height: 40px;
  }
</style>
