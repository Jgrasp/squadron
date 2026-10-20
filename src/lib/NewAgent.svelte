<script lang="ts">
  import Dialog from './Dialog.svelte'
  import OptionsFields from './OptionsFields.svelte'
  import type { Project } from './projects.svelte'
  import { untrack } from 'svelte'
  import { settings } from './settings.svelte'
  import type { Options } from './stream'

  let {
    project,
    draft = { name: '', text: '' },
    start,
    onclose,
  }: {
    project: Project
    draft?: { name: string; text: string }
    start: (name: string, options: Options, text: string) => void
    onclose: () => void
  } = $props()

  let name = $state(untrack(() => draft.name))
  let text = $state(untrack(() => draft.text))
  let options = $state<Options>({ ...settings.defaults })

  const ready = $derived(text.trim().length > 0)

  function submit(event?: Event) {
    event?.preventDefault()
    if (!ready) return
    start(name.trim(), $state.snapshot(options), text.trim())
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
        <input id="agent-name" class="field" bind:value={name} placeholder="Leave empty to get a title from the first message" />
      </div>

      <OptionsFields bind:options />

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
