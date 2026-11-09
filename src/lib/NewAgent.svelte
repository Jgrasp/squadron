<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { open as chooseFiles } from '@tauri-apps/plugin-dialog'
  import { ui } from './actions.svelte'
  import AgentDefinition from './AgentDefinition.svelte'
  import AgentPicker from './AgentPicker.svelte'
  import { agents } from './agents.svelte'
  import Attachment from './Attachment.svelte'
  import { agentTypes, catalogs, models, type Definition } from './catalogs.svelte'
  import { families, find as findCommand, options as argumentsOf } from './commands'
  import Trans from './Trans.svelte'
  import Dialog from './Dialog.svelte'
  import { t } from './i18n/index.svelte'
  import LaunchOptions from './LaunchOptions.svelte'
  import { defaultsFor, type Project } from './projects.svelte'
  import { tick, untrack } from 'svelte'
  import { Mention } from './mention.svelte'
  import { imagePaste } from './paste'
  import Composer from './Composer.svelte'
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

  type Choice = { value: string; label: string; text: string; at?: number; detail?: string; usage?: string }

  let message: ReturnType<typeof Composer>
  let dismissed = $state('')
  let chosen = $state('')
  let pick = $state(0)
  const mentions = new Mention(() => text, () => project.path)
  const skills = $derived(
    agents.list.findLast((a) => a.project === project.path && a.skills.length)?.skills ?? agents.list.findLast((a) => a.skills.length)?.skills ?? [],
  )
  const commands = $derived(
    ((catalogs[project.path] ?? Object.values(catalogs).at(-1))?.commands ?? skills.map((name) => ({ name, description: '' }))).filter((c) =>
      skills.some((s) => s === c.name || s.endsWith(`:${c.name}`)),
    ),
  )
  const stage = $derived.by(() => {
    if (text === dismissed || mentions.found) return null
    const m = /^\/(\S*)(?:\s+(\S*))?$/.exec(text)
    if (!m) return null
    if (m[2] === undefined) return { query: m[1].toLowerCase() }
    const command = findCommand(commands, m[1])
    return command ? { command, query: m[2].toLowerCase() } : null
  })
  const choices = $derived.by((): Choice[] => {
    if (text === dismissed) return []
    if (!stage) return mentions.choices
    const q = stage.query
    if (!stage.command)
      return commands
        .filter((c) => c.name.toLowerCase().includes(q))
        .map((c) => ({ value: c.name, label: `/${c.name}`, detail: c.description, usage: c.argumentHint, text: `/${c.name} ` }))
        .sort((a, b) => Number(!a.value.startsWith(q)) - Number(!b.value.startsWith(q)) || a.value.localeCompare(b.value))
    const name = stage.command.name
    return argumentsOf(stage.command)
      .filter((o) => `${o.value} ${o.label ?? ''}`.toLowerCase().includes(q))
      .map((o) => ({ value: o.value, label: o.label ?? o.value, detail: o.detail, text: `/${name} ${o.value} ` }))
  })
  const usage = $derived(stage?.command && (choices.length || !stage.query) ? stage.command : null)

  $effect(() => {
    void mentions.found?.query
    void stage?.command
    pick = 0
  })

  function choose(choice: Choice) {
    text = choice.text
    chosen = choice.at === undefined ? choice.text : ''
    const at = (mentions.caret = choice.at ?? choice.text.length)
    message.focus()
    tick().then(() => message.setSelectionRange(at, at))
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
    } else if (event.key === 'Backspace' && text === chosen && text.endsWith(' ') && message.selection().from === text.length) {
      event.preventDefault()
      text = chosen = text.replace(/[^\s/]+ $/, '')
    } else if ((choices.length || usage) && event.key === 'Escape') {
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
      <div class="write">
        <div class="box">
          <Composer
            id="first-message"
            label={t('board.firstMessage')}
            bind:value={text}
            bind:this={message}
            placeholder={t('board.firstMessagePlaceholder')}
            autofocus
            onkeydown={keys}
            onpaste={(e) => clip.paste(e, message)}
            oncaret={mentions.track}
            files={mentions.paths}
          />
          <div class="boxbar">
            <button type="button" class="btn-icon" aria-label={t('board.attach')} title={t('board.attachTitle')} onclick={browseFiles}>
              <svg width="15" height="15" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
                ><path d="M21 11.5l-8.6 8.6a5.5 5.5 0 0 1-7.8-7.8l8.6-8.6a3.7 3.7 0 0 1 5.2 5.2l-8.6 8.6a1.8 1.8 0 0 1-2.6-2.6l7.9-7.9" /></svg
              >
            </button>
          </div>
        </div>
        {#if choices.length || usage}
          <div class="mentions" class:slash={!!stage} role="listbox" aria-label={usage ? t('agent.argumentsOf', { command: usage.name }) : stage ? t('agent.skills') : t('agent.files')}>
            {#if usage}
              <div class="usage" role="presentation">
                <span class="name">/{usage.name}</span>
                <span class="what">{usage.description}</span>
                {#if usage.argumentHint}<code>{usage.argumentHint}</code>{/if}
              </div>
            {:else if stage}
              <div class="section" role="presentation">{t('agent.skills')}</div>
            {/if}
            {#each choices as choice, i (choice.value)}
              <button type="button" role="option" aria-selected={i === pick} onmousemove={() => (pick = i)} onmousedown={(e) => e.preventDefault()} onclick={() => choose(choice)}>
                <span class="line">{choice.label}{#if i === pick && choice.usage}<code>{choice.usage}</code>{/if}</span>
                {#if i === pick && choice.detail}<span class="about">{choice.detail}</span>{/if}
              </button>
            {/each}
            {#if stage}
              <div class="keys-help" role="presentation">
                {#if choices.length}<Trans k="agent.keysMove">{#snippet up()}<kbd>↑</kbd>{/snippet}{#snippet down()}<kbd>↓</kbd>{/snippet}</Trans>{' · '}<Trans k="agent.keysChoose"
                    >{#snippet tab()}<kbd>Tab</kbd>{/snippet}{#snippet enter()}<kbd>{t('agent.keyEnter')}</kbd>{/snippet}</Trans
                  >{' · '}{/if}{#if text === chosen && text.endsWith(' ')}<Trans k="agent.keysBack">{#snippet key()}<kbd>⌫</kbd>{/snippet}</Trans>{' · '}{/if}<Trans k="agent.keysClose"
                  >{#snippet key()}<kbd>{t('agent.keyEsc')}</kbd>{/snippet}</Trans
                >
              </div>
            {/if}
          </div>
        {/if}
      </div>
      {#if files.length || attachError}
        <div class="files" aria-label={t('board.taskFiles')}>
          {#each files as file, i (file)}<Attachment path={file} onremove={() => files.splice(i, 1)} />{/each}
          {#if attachError}<span class="attach-error" role="alert">{attachError}</span>{/if}
        </div>
      {/if}

      <LaunchOptions bind:options>
        {#snippet before()}
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
        {/snippet}
        {#snippet picker()}
          <AgentPicker project={project.path} agent={options.agent} onpick={persona} onread={(agent) => (reading = agent)} />
        {/snippet}
      </LaunchOptions>
    </div>

    <footer>
      <button type="button" class="btn btn-secondary" onclick={onclose}>{t('common.cancel')}<span class="keys"><kbd>{t('agent.keyEsc')}</kbd></span></button>
      <button class="btn btn-primary" disabled={!ready}>{t('board.startAgent')}<span class="keys"><kbd>Ctrl</kbd><kbd>↵</kbd></span></button>
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

  .slash {
    position: static;
    margin-top: 4px;
    max-height: 340px;
  }

  .slash button {
    white-space: normal;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .mentions code {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 12px var(--mono);
    color: var(--accent-soft);
  }

  .about {
    display: block;
    margin-top: 3px;
    font: 12.5px/18px var(--sans);
    color: var(--text-muted);
  }

  .section {
    padding: 6px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent-soft);
  }

  .usage {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: baseline;
    gap: 4px 12px;
    margin: -6px -6px 6px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);
  }

  .usage .name {
    font: 13px var(--mono);
    color: var(--text-strong);
  }

  .what {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .usage code {
    grid-column: 1 / -1;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  .keys-help {
    position: sticky;
    bottom: -6px;
    margin: 6px -6px -6px;
    padding: 7px 16px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
    font-size: 11.5px;
    color: var(--text-dim);
  }

  .keys-help kbd {
    padding: 0 4px;
    border: 0;
    border-radius: 3px;
    background: var(--surface-3);
    font: 11px var(--mono);
    color: var(--text-2);
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

  .box :global(.composer-field) {
    padding: 12px 14px;
    color: var(--text);
  }

  .box :global(.cm-editor) {
    height: 7.5em;
  }

  .boxbar {
    display: flex;
    padding: 0 8px 8px;
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

  footer .btn {
    height: 40px;
    gap: 10px;
  }

  footer .btn:first-child {
    margin-left: auto;
  }

  .keys {
    display: flex;
    gap: 3px;
    opacity: 0.5;
  }

  kbd {
    padding: 1px 5px;
    border: 1px solid currentColor;
    border-radius: 4px;
    font: 500 11px var(--mono);
  }
</style>
