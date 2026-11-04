<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import AgentDefinition from './AgentDefinition.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import Attachment from './Attachment.svelte'
  import { agentTypes, models, type Definition } from './catalogs.svelte'
  import { families } from './commands'
  import Dialog from './Dialog.svelte'
  import { t, type Key } from './i18n/index.svelte'
  import OptionRow from './OptionRow.svelte'
  import { advisorChoices, advisorIdle, advisorLabel, advising, effortLabel, modelLabel } from './format'
  import { layout, setLaunchSettings } from './layout.svelte'
  import { defaultsFor, type Project } from './projects.svelte'
  import { tick, untrack } from 'svelte'
  import { Mention, type FileChoice } from './mention.svelte'
  import { imagePaste } from './paste'
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
  let options = $state<Options>(untrack(() => defaultsFor(project.path)))
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

  $effect(() => {
    let stop: (() => void) | undefined
    let gone = false
    untrack(() => ui.drops++)
    getCurrentWebview()
      .onDragDropEvent(({ payload }) => payload.type === 'drop' && attach(payload.paths))
      .then((unlisten) => (gone ? unlisten() : (stop = unlisten)))
      .catch(() => {})
    return () => {
      gone = true
      ui.drops--
      stop?.()
    }
  })

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

  const EFFORTS: Options['effort'][] = ['low', 'medium', 'high', 'xhigh', 'max']
  const MODES: Options['mode'][] = ['default', 'acceptEdits', 'plan', 'auto']
  const TITLES: Record<Options['mode'], Key> = {
    default: 'settings.modeDefault',
    acceptEdits: 'settings.modeAcceptEdits',
    plan: 'settings.modePlan',
    auto: 'settings.modeAuto',
  }
  const HINTS: Record<Options['mode'], Key> = {
    default: 'settings.modeDefaultHint',
    acceptEdits: 'settings.modeAcceptEditsHint',
    plan: 'settings.modePlanHint',
    auto: 'settings.modeAutoHint',
  }

  const catalog = $derived(models().filter((m) => m.value !== 'default'))
  const modelChoices = $derived(
    catalog.length ? catalog.map((m) => ({ value: m.value, label: modelLabel(m.value), detail: m.description })) : ['opus', 'sonnet', 'haiku'].map((value) => ({ value, label: modelLabel(value) })),
  )
  const levels = $derived(catalog.find((m) => m.value === options.model)?.supportedEffortLevels)
  const effortChoices = $derived(EFFORTS.map((value) => ({ value, label: effortLabel(value), off: !!levels && !levels.includes(value) })))
  const modeChoices = $derived(MODES.map((value) => ({ value, label: t(TITLES[value]), detail: t(HINTS[value]), tone: `tone-${value}` })))

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
      <div class="write">
        <div class="box">
          <!-- svelte-ignore a11y_autofocus -->
          <textarea
            id="first-message"
            rows="5"
            aria-label={t('board.firstMessage')}
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
          <div class="boxbar">
            <button type="button" class="btn-icon" aria-label={t('board.attach')} title={t('board.attachTitle')} onclick={browseFiles}>
              <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                ><path d="M21 11.5l-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6l7.9-7.9" /></svg
              >
            </button>
          </div>
        </div>
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
      {#if files.length || attachError}
        <div class="files" aria-label={t('board.taskFiles')}>
          {#each files as file, i (file)}<Attachment path={file} onremove={() => files.splice(i, 1)} />{/each}
          {#if attachError}<span class="attach-error" role="alert">{attachError}</span>{/if}
        </div>
      {/if}

      <div class="summary">
        <dl>
          <div><dt>{t('settings.model')}</dt><dd>{modelLabel(options.model)}</dd></div>
          <div><dt>{t('settings.effort')}</dt><dd>{effortLabel(options.effort)}</dd></div>
          {#if advising(options)}
            <div class="advisor" class:idle={advisorIdle(options)}><dt>{t('settings.advisor')}</dt><dd>{modelLabel(options.advisor!)}</dd></div>
          {/if}
        </dl>
        <button type="button" class="btn btn-secondary launch" aria-expanded={layout.launchSettings} onclick={() => setLaunchSettings(!layout.launchSettings)}>
          <svg width="14" height="14" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h9M17 7h3M4 17h3M11 17h9" /><circle cx="15" cy="7" r="2" /><circle cx="9" cy="17" r="2" /></svg>
          {t('board.launchSettings')}
        </button>
      </div>

      {#if layout.launchSettings}
        <div class="settings">
          <div class="group">
            <label for="agent-name">{t('common.name')}</label>
            <input id="agent-name" class="field" bind:value={name} placeholder={t('board.namePlaceholder')} />
          </div>
          <div class="group">
            <label for="done-when">{t('common.doneWhen')}</label>
            <textarea
              id="done-when"
              class="field"
              rows="3"
              bind:value={goal}
              placeholder={t('board.agentGoalPlaceholder')}
              onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
            ></textarea>
          </div>
          {#if prefilled}<p class="prefilled">{t('persona.prefilled')}</p>{/if}
          <div class="rows">
            <OptionRow label={t('settings.model')} value={modelLabel(options.model)} choices={modelChoices} selected={options.model} onpick={(v) => (options.model = v)}>
              {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><rect x="5" y="5" width="14" height="14" rx="3" /><path d="M9 1v4M15 1v4M9 19v4M15 19v4M1 9h4M1 15h4M19 9h4M19 15h4" /></svg>{/snippet}
            </OptionRow>
            <OptionRow label={t('settings.effort')} value={effortLabel(options.effort)} choices={effortChoices} selected={options.effort} onpick={(v) => (options.effort = v as Options['effort'])}>
              {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M5 20v-4M10 20v-8M15 20v-12M20 20V4" /></svg>{/snippet}
            </OptionRow>
            <OptionRow
              label={t('settings.advisor')}
              value={advisorLabel(options.advisor)}
              tone={advising(options) ? 'tone-advisor' : ''}
              choices={advisorChoices(options.model)}
              selected={options.advisor ?? ''}
              onpick={(v) => (options.advisor = v || undefined)}
            >
              {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M12 3l2.4 5.6L20 11l-5.6 2.4L12 19l-2.4-5.6L4 11l5.6-2.4z" /></svg>{/snippet}
            </OptionRow>
            <OptionRow label={t('settings.permissionMode')} value={t(TITLES[options.mode])} tone="tone-{options.mode}" choices={modeChoices} selected={options.mode} onpick={(v) => (options.mode = v as Options['mode'])}>
              {#snippet icon()}<svg width="14" height="14" viewBox="0 0 24 24"><path d="M12 3l7 3v5c0 5-3 8-7 10-4-2-7-5-7-10V6z" /></svg>{/snippet}
            </OptionRow>
            <AgentPicker project={project.path} agent={options.agent} onpick={persona} onread={(agent) => (reading = agent)} />
          </div>
        </div>
      {/if}
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
    margin: -4px 0 0;
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

  .box {
    display: flex;
    flex-direction: column;
    border-radius: 6px;
    background: var(--surface-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }

  .box:focus-within {
    box-shadow: inset 0 0 0 1px var(--border-focus);
  }

  .box textarea {
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    outline: none;
  }

  .boxbar {
    display: flex;
    padding: 0 8px 8px;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  dl {
    display: flex;
    gap: 18px;
    margin: 0;
    padding: 0 2px;
  }

  dt {
    font-size: 10px;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-faint);
  }

  dd {
    margin: 1px 0 0;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .advisor dt {
    color: var(--advisor);
  }

  .advisor dd {
    color: var(--advisor-text);
  }

  .advisor.idle dd {
    color: var(--text-faint);
    text-decoration: line-through;
  }

  .launch {
    height: 32px;
    margin-left: auto;
    gap: 7px;
  }

  .launch svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .launch[aria-expanded='true'] {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .rows {
    display: flex;
    flex-direction: column;
    padding: 4px;
    border-radius: 6px;
    background: var(--surface-2);
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
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
