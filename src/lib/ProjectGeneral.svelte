<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import ColorPick from './ColorPick.svelte'
  import { t } from './i18n/index.svelte'
  import { editProject, home, move, type Project } from './projects.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  async function relocate() {
    const chosen = await open({ directory: true, defaultPath: project.path })
    if (typeof chosen === 'string') act(() => move(project.path, chosen))
  }
</script>

<section>
  <h4>{t('settings.projectGeneral')}</h4>
  <div class="options">
    <div class="option">
      <label class="label" for="project-name"><strong>{t('common.name')}</strong></label>
      <input id="project-name" class="field" value={project.name} onchange={(e) => act(() => editProject(project.path, { name: e.currentTarget.value }))} />
    </div>
    <div class="option">
      <span class="label"><strong>{t('settings.color')}</strong></span>
      <ColorPick value={project.color} label={t('settings.colorOf', { name: project.name })} onpick={(color) => act(() => editProject(project.path, { color }))} />
    </div>
    <label class="option">
      <span class="label">
        <strong>{t('settings.pinnedTab')}</strong>
        <small>{t('settings.pinHint')}</small>
      </span>
      <input type="checkbox" class="switch" checked={project.pinned} onchange={(e) => act(() => editProject(project.path, { pinned: e.currentTarget.checked }))} />
    </label>
    <div class="option">
      <span class="label">
        <strong>{t('settings.folder')}</strong>
        <small class="path">{home(project.path)}</small>
      </span>
      <button class="btn btn-ghost" onclick={relocate}>{t('settings.changeFolder')}</button>
    </div>
  </div>
</section>

<style>
  .path {
    font-family: var(--mono);
  }
</style>
