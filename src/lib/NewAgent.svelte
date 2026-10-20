<script lang="ts">
  import { create } from './agents.svelte'
  import Dialog from './Dialog.svelte'
  import type { Project } from './projects.svelte'
  import type { Options } from './stream'

  let { project, onclose }: { project: Project; onclose: () => void } = $props()

  const models: Options['model'][] = ['opus', 'sonnet', 'haiku']
  const efforts: Options['effort'][] = ['low', 'medium', 'high', 'xhigh', 'max']
  const modes: { value: Options['mode']; title: string; hint: string }[] = [
    { value: 'default', title: 'Ask for everything', hint: 'Every tool waits for your approval' },
    { value: 'acceptEdits', title: 'Accept edits', hint: 'Edits go through, commands ask' },
    { value: 'plan', title: 'Plan', hint: 'Proposes a plan, acts only once approved' },
    { value: 'auto', title: 'Auto', hint: 'Claude Code decides, and only asks for risky actions' },
  ]

  let saved: Options = { model: 'opus', effort: 'high', mode: 'default' }
  try {
    saved = { ...saved, ...JSON.parse(localStorage.getItem('squadron.options') ?? '{}') }
  } catch {}

  let name = $state('')
  let text = $state('')
  let options = $state<Options>(saved)

  const ready = $derived(text.trim().length > 0)

  function submit(event?: Event) {
    event?.preventDefault()
    if (!ready) return
    try {
      localStorage.setItem('squadron.options', JSON.stringify(options))
    } catch {}
    const title = name.trim() || text.trim().split(/\s+/).slice(0, 5).join(' ')
    create(project.path, title, $state.snapshot(options), text.trim())
    onclose()
  }
</script>

<Dialog title="New agent" {onclose} width={620}>
  {#snippet badge()}
    <span class="in"><span class="dot" aria-hidden="true"></span>in {project.name}</span>
  {/snippet}
  <form onsubmit={submit}>
    <div class="fields">
      <div class="group">
        <label for="agent-name">Name</label>
        <input id="agent-name" class="field" bind:value={name} placeholder="Defaults to the start of the first message" />
      </div>

      <div class="pair">
        <div class="group">
          <span id="model" class="label">Model</span>
          <div class="segments" role="group" aria-labelledby="model">
            {#each models as model (model)}
              <button type="button" aria-pressed={options.model === model} onclick={() => (options.model = model)}>
                {model[0].toUpperCase() + model.slice(1)}
              </button>
            {/each}
          </div>
        </div>
        <div class="group">
          <span id="effort" class="label">Effort</span>
          <div class="segments small" role="group" aria-labelledby="effort">
            {#each efforts as effort (effort)}
              <button type="button" aria-pressed={options.effort === effort} onclick={() => (options.effort = effort)}>
                {effort}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="group">
        <span id="mode" class="label">Permission mode</span>
        <div class="modes" role="group" aria-labelledby="mode">
          {#each modes as mode (mode.value)}
            <button type="button" aria-pressed={options.mode === mode.value} onclick={() => (options.mode = mode.value)}>
              <strong>{mode.title}</strong>
              <span>{mode.hint}</span>
            </button>
          {/each}
        </div>
      </div>

      <div class="group">
        <label for="first-message">First message</label>
        <!-- svelte-ignore a11y_autofocus -->
        <textarea
          id="first-message"
          class="field"
          rows="4"
          bind:value={text}
          autofocus
          onkeydown={(e) => e.ctrlKey && e.key === 'Enter' && submit(e)}
        ></textarea>
      </div>
    </div>

    <footer>
      <span class="hint"><kbd>Ctrl ↵</kbd> to start</span>
      <button type="button" class="btn btn-secondary" onclick={onclose}>Cancel</button>
      <button class="btn btn-primary" disabled={!ready}>Start agent</button>
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
    background: rgba(224, 138, 91, 0.16);
    box-shadow: inset 0 0 0 1px rgba(224, 138, 91, 0.6);
    font-size: 12.5px;
    font-weight: 600;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 4px;
    background: var(--accent);
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

  label,
  .label {
    font-weight: 600;
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .segments {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .segments button {
    flex: 1;
    height: 34px;
    border-radius: 3px;
    color: var(--text-2);
  }

  .segments.small button {
    font-size: 12.5px;
  }

  .segments button[aria-pressed='true'] {
    background: var(--text);
    color: var(--ink);
    font-weight: 700;
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .modes button {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 11px 14px;
    border-radius: 4px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    text-align: left;
  }

  .modes strong {
    font-weight: 600;
  }

  .modes span {
    font-size: 12.5px;
    color: var(--text-dim);
  }

  .modes button[aria-pressed='true'] {
    background: var(--surface-3);
    box-shadow: inset 0 0 0 2px var(--text);
    color: var(--text-strong);
  }

  .modes button[aria-pressed='true'] strong {
    font-weight: 700;
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
