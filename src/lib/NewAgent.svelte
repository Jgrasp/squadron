<script lang="ts">
  import Dialog from './Dialog.svelte'
  import { t } from './i18n/index.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import type { Project } from './projects.svelte'
  import { untrack } from 'svelte'
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
    start: (name: string, options: Options, text: string, goal: string, worktree: string | null) => void
    onclose: () => void
  } = $props()

  let name = $state(untrack(() => draft.name))
  let text = $state(untrack(() => draft.text))
  let options = $state<Options>({ ...settings.defaults })
  let goal = $state('')
  let worktree = $state<string | null>(null)

  const ready = true

  function submit(event?: Event) {
    event?.preventDefault()
    if (!ready) return
    start(name.trim(), $state.snapshot(options), text.trim(), goal.trim(), worktree)
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
          onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
        ></textarea>
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
