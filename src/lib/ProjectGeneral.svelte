<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { shelve } from './agents.svelte'
  import ColorPick from './ColorPick.svelte'
  import { t } from './i18n/index.svelte'
  import { editProject, home, move, type Project } from './projects.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  async function relocate() {
    const chosen = await open({ directory: true, defaultPath: project.path })
    if (typeof chosen === 'string') act(() => move(project.path, chosen))
  }
</script>

<div class="group">
  <label class="label" for="project-name">{t('common.name')}</label>
  <input id="project-name" class="field" value={project.name} onchange={(e) => act(() => editProject(project.path, { name: e.currentTarget.value }))} />
</div>

<div class="group">
  <span class="label">{t('settings.color')}</span>
  <ColorPick value={project.color} label={t('settings.colorOf', { name: project.name })} onpick={(color) => act(() => editProject(project.path, { color }))} />
</div>

<div class="group">
  <span class="label">{t('settings.pinning')}</span>
  <button class="pin" aria-pressed={project.pinned} onclick={() => act(() => editProject(project.path, { pinned: !project.pinned }))}>
    <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 4h6l-1 6 4 4H6l4-4zM12 14v6" /></svg>
    {project.pinned ? t('settings.pinned') : t('settings.pin')}
  </button>
  <p class="hint">{t('settings.pinHint')}</p>
</div>

<div class="group">
  <span class="label">{t('settings.folder')}</span>
  <div class="folder">
    <code>{home(project.path)}</code>
    {#if !project.exists}<span class="missing">{t('settings.folderMissing')}</span>{/if}
  </div>
  <div class="actions">
    <button class="btn btn-secondary" onclick={relocate}>{t('settings.changeFolder')}</button>
    <button class="btn btn-secondary" onclick={() => act(() => shelve(project))}>{t('settings.archiveEllipsis')}</button>
  </div>
</div>

<style>
  .group {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    max-width: 560px;
  }

  .group .field {
    align-self: stretch;
    height: 36px;
  }

  .label {
    font-weight: 600;
  }

  .hint {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .pin {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 14px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text-muted);
  }

  .pin svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linejoin: round;
  }

  .pin[aria-pressed='true'] {
    background: var(--surface-4);
    box-shadow: inset 0 0 0 1px var(--project);
    color: var(--text-strong);
    font-weight: 600;
  }

  .folder {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .missing {
    color: var(--err-text);
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .actions .btn {
    height: 34px;
  }
</style>
