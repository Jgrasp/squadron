<script lang="ts">
  import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
  import AgentDefinition from './AgentDefinition.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import Attachment from './Attachment.svelte'
  import { agentTypes, models, type Definition } from './catalogs.svelte'
  import { families } from './commands'
  import Dialog from './Dialog.svelte'
  import { t } from './i18n/index.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import type { Project } from './projects.svelte'
  import { tick, untrack } from 'svelte'
  import { Mention, type FileChoice } from './mention.svelte'
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
  let prefilled = $state(false)
  let reading = $state<string | null>(null)

  function persona(agent: string | undefined, found: Definition | null) {
    options.agent = agent
    prefilled = false
    if (!agent) return
    const model = found?.model ?? agentTypes(project.path).find((a) => a.name === agent)?.model
    if (model && (families(models()).includes(model) || models().some((m) => m.value === model))) {
      options.model = model
      prefilled = true
    }
    const effort = found?.effort as Options['effort'] | undefined
    if (effort && ['low', 'medium', 'high', 'xhigh', 'max'].includes(effort)) {
      options.effort = effort
      prefilled = true
    }
  }

  function attach(paths: string[]) {
    files = [...files, ...paths.filter((p) => !files.includes(p))]
  }

  const clip = imagePaste(attach, (e) => (attachError = e))

  let message: HTMLTextAreaElement
  let dismissed = $state('')
  let pick = $state(0)
  const mentions = new Mention(() => text, () => project.path)
  const choices = $derived(text === dismissed ? [] : mentions.choices)

  $effect(() => {
    void mentions.found?.query
    pick = 0
  })

  function choose(choice: FileChoice) {
    text = choice.text
    mentions.caret = choice.at
    message.focus()
    tick().then(() => message.setSelectionRange(choice.at, choice.at))
  }

  function keys(event: KeyboardEvent) {
    clip.key(event)
    if (choices.length && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
      event.preventDefault()
      pick = (pick + (event.key === 'ArrowDown' ? 1 : -1) + choices.length) % choices.length
      tick().then(() => document.querySelector('.mentions [aria-selected="true"]')?.scrollIntoView({ block: 'nearest' }))
    } else if (choices.length && (event.key === 'Tab' || (event.key === 'Enter' && !event.ctrlKey && !event.shiftKey))) {
      event.preventDefault()
      choose(choices[pick])
    } else if (choices.length && event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      dismissed = text
    } else if (event.ctrlKey && event.key === 'Enter') submit(event)
  }

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

      <AgentPicker project={project.path} agent={options.agent} onpick={persona} onread={(agent) => (reading = agent)} />

      {#if prefilled}<p class="prefilled">{t('persona.prefilled')}</p>{/if}

      <OptionsFields bind:options />

      <div class="group">
        <label for="first-message">{t('board.firstMessage')}</label>
        <div class="write">
          <!-- svelte-ignore a11y_autofocus -->
          <textarea
            id="first-message"
            class="field"
            rows="4"
            bind:value={text}
            bind:this={message}
            placeholder={t('board.firstMessagePlaceholder')}
            autofocus
            onkeydown={keys}
            onpaste={clip.paste}
            oninput={mentions.track}
            onkeyup={mentions.track}
            onmouseup={mentions.track}
          ></textarea>
          {#if choices.length}
            <div class="mentions" role="listbox" aria-label={t('agent.files')}>
              {#each choices as choice, i (choice.value)}
                <button type="button" role="option" aria-selected={i === pick} onmousemove={() => (pick = i)} onmousedown={(e) => e.preventDefault()} onclick={() => choose(choice)}
                  >{choice.label}</button
                >
              {/each}
            </div>
          {/if}
        </div>
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

{#if reading}<AgentDefinition project={project.path} name={reading} onclose={() => (reading = null)} />{/if}

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

  .prefilled {
    margin: -10px 0 0;
    color: var(--persona);
    font-size: 12px;
  }

  .write {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .mentions {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 20;
    max-height: 240px;
    overflow-y: auto;
    padding: 6px;
    border-radius: 6px;
    background: var(--menu);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 14px 36px rgba(0, 0, 0, 0.5);
  }

  .mentions button {
    display: block;
    width: 100%;
    min-height: 32px;
    padding: 6px 10px;
    border-radius: 4px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    text-align: left;
    font: 13px var(--mono);
    color: var(--text-strong);
  }

  .mentions button[aria-selected='true'] {
    background: var(--menu-hover);
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
