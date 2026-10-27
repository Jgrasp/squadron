<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { preview } from './Attachment.svelte'
  import { t } from './i18n/index.svelte'
  import { viewer } from './captures.svelte'

  let dialog = $state<HTMLDialogElement>()
  let src = $state('')
  let failed = $state(false)

  const path = $derived(viewer.paths[viewer.index])
  const count = $derived(viewer.paths.length)

  $effect(() => {
    if (path && dialog && !dialog.open) dialog.showModal()
  })

  $effect(() => {
    if (!path) return
    const wanted = path
    failed = false
    preview(wanted)
      .then((data) => wanted === path && (src = data))
      .catch(() => wanted === path && ((src = ''), (failed = true)))
  })

  function close() {
    dialog?.close()
    viewer.paths = []
    src = ''
  }

  function step(by: number) {
    if (count > 1) viewer.index = (viewer.index + by + count) % count
  }

  function key(event: KeyboardEvent) {
    if (event.key === 'ArrowLeft') step(-1)
    else if (event.key === 'ArrowRight') step(1)
    else return
    event.preventDefault()
  }

  const name = $derived(path?.split('/').pop() ?? '')
</script>

{#if path}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <dialog
    bind:this={dialog}
    aria-label={name}
    oncancel={(e) => (e.preventDefault(), close())}
    onkeydown={key}
    onclick={(e) => e.target === e.currentTarget && close()}
  >
    <div class="bar">
      <span class="name" title={path}>{name}</span>
      {#if count > 1}<span class="count">{viewer.index + 1} / {count}</span>{/if}
      <button type="button" class="btn btn-secondary" onclick={() => invoke('open_attachment', { path })}>{t('conversation.openCaptureOutside')}</button>
      <!-- svelte-ignore a11y_autofocus -->
      <button type="button" class="close" autofocus aria-label={t('conversation.closeCapture')} onclick={close}>×</button>
    </div>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="stage" onclick={(e) => e.target === e.currentTarget && close()}>
      {#if count > 1}
        <button type="button" class="nav prev" aria-label={t('conversation.previousCapture')} onclick={() => step(-1)}>‹</button>
      {/if}
      {#if failed}
        <span class="gone">{path}</span>
      {:else if src}
        <img {src} alt={name} />
      {/if}
      {#if count > 1}
        <button type="button" class="nav next" aria-label={t('conversation.nextCapture')} onclick={() => step(1)}>›</button>
      {/if}
    </div>
  </dialog>
{/if}

<style>
  dialog[open] {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    max-width: none;
    max-height: none;
    margin: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
  }

  dialog::backdrop {
    background: rgba(8, 6, 5, 0.92);
  }

  .bar {
    display: flex;
    flex: none;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
  }

  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font: 12.5px var(--mono);
    color: var(--text-2);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .count {
    font-size: 12.5px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .close {
    width: 32px;
    height: 32px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 18px;
  }

  .stage {
    position: relative;
    display: grid;
    flex: 1;
    min-height: 0;
    place-items: center;
    padding: 0 72px 24px;
  }

  img {
    display: block;
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
    box-shadow: 0 0 0 1px var(--border-strong);
    object-fit: contain;
  }

  .gone {
    font: 12px var(--mono);
    color: var(--text-dim);
  }

  .nav {
    position: absolute;
    top: 50%;
    width: 44px;
    height: 44px;
    margin-top: -34px;
    border-radius: 50%;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 26px;
    line-height: 1;
  }

  .nav:hover,
  .close:hover {
    color: var(--text-strong);
  }

  .prev {
    left: 16px;
  }

  .next {
    right: 16px;
  }
</style>
