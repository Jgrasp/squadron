<script lang="ts">
  import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
  import Attachment from './Attachment.svelte'
  import Dialog from './Dialog.svelte'
  import { t } from './i18n/index.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import type { Project } from './projects.svelte'
  import { untrack } from 'svelte'
  import { imagePaste } from './paste'
  import { settings } from './settings.svelte'
  import Trans from './Trans.svelte'
  import type { Options } from './stream'

  let {
    project,
    draft = { name: '', text: '' },
    start,
    onclose,
  }: {
    project: Project
    draft?: { name: string; text: string }
    start: (name: string, options: Options, text: string, goal: string, worktree: string | null, files: string[]) => void
    onclose: () => void
  } = $props()

  let name = $state(untrack(() => draft.name))
  let text = $state(untrack(() => draft.text))
  let options = $state<Options>({ ...settings.defaults })
  let goal = $state('')
  let worktree = $state<string | null>(null)
  let files = $state<string[]>([])
  let attachError = $state('')

  function attach(paths: string[]) {
    files = [...files, ...paths.filter((p) => !files.includes(p))]
  }

  const clip = imagePaste(attach, (e) => (attachError = e))

  async function browseFiles() {
    const chosen = await chooseFiles({ multiple: true })
    if (chosen) attach(Array.isArray(chosen) ? chosen : [chosen])
  }

  const ready = true

  function submit(event?: Event) {
    event?.preventDefault()
    if (!ready) return
    start(name.trim(), $state.snapshot(options), text.trim(), goal.trim(), worktree, $state.snapshot(files))
    onclose()
  }
</script>

<Dialog title={t('common.newAgent')} {onclose} width={620}>
  {#snippet badge()}
    <span class="in" style:--project={project.color}><span class="dot" aria-hidden="true"></span>{t('board.inProject', { project: project.name })}</span>
  {/snippet}
  <form onsubmit={submit}>
    <div class="fields">
      <div class="group">
        <label for="agent-name">{t('common.name')}</label>
        <input id="agent-name" class="field" bind:value={name} placeholder={t('board.namePlaceholder')} />
      </div>

      <OptionsFields bind:options />

      <div class="group">
        <label for="first-message">{t('board.firstMessage')}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea
          id="first-message"
          class="field"
          rows="4"
          bind:value={text}
          placeholder={t('board.firstMessagePlaceholder')}
          autofocus
          onkeydown={(e) => (clip.key(e), e.ctrlKey && e.key === 'Enter' && submit(e))}
          onpaste={clip.paste}
        ></textarea>
        <div class="files" aria-label={t('board.taskFiles')}>
          {#each files as file, i (file)}<Attachment path={file} onremove={() => files.splice(i, 1)} />{/each}
          <button type="button" class="btn btn-secondary attach" title={t('board.attachTitle')} onclick={browseFiles}>
            <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
              ><path d="M21 11.5l-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6l7.9-7.9" /></svg
            >{t('board.attach')}
          </button>
          {#if attachError}<span class="attach-error" role="alert">{attachError}</span>{/if}
        </div>
      </div>

      <div class="group">
        <label for="done-when">{t('common.doneWhen')}</label>
        <textarea
          id="done-when"
          class="field"
          rows="2"
          bind:value={goal}
          placeholder={t('board.agentGoalPlaceholder')}
          onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
        ></textarea>
      </div>
    </div>

    <footer>
      <span class="hint"><Trans k="board.ctrlEnterToStart">{#snippet keys()}<kbd>Ctrl ↵</kbd>{/snippet}</Trans></span>
      <button type="button" class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}</button>
      <button class="btn btn-primary" disabled={!ready}>{t('board.startAgent')}</button>
    </footer>
  </form>
</Dialog>

<style>
  .in {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 26px;
    padding: 0 10px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--project) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--project) 60%, transparent);
    font-size: 12.5px;
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
    background: var(--project);
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 10px 22px 20px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  label {
    font-weight: 600;
  }

  textarea {
    height: auto;
    padding: 12px 14px;
    resize: none;
    line-height: 1.5;
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
  }

  .attach {
    height: 32px;
    gap: 6px;
  }

  .attach-error {
    flex-basis: 100%;
    font-size: 12.5px;
    color: var(--err-text);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 22px 20px;
    border-top: 1px solid var(--border);
  }

  .hint {
    flex: 1;
    font-size: 12.5px;
    color: var(--text-dim);
  }

  kbd {
    font-family: var(--mono);
  }

  footer .btn {
    height: 40px;
  }
</style>
