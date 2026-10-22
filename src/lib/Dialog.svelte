<script lang="ts">
  import type { Snippet } from 'svelte'

  let {
    title,
    width = 0,
    full = false,
    onclose,
    badge,
    children,
  }: { title: string; width?: number; full?: boolean; onclose: () => void; badge?: Snippet; children: Snippet } =
    $props()

  let dialog: HTMLDialogElement

  $effect(() => {
    dialog.showModal()
  })
</script>

<dialog
  bind:this={dialog}
  class:full
  style:width={full ? null : `${width}px`}
  aria-labelledby="dialog-title"
  oncancel={(e) => {
    e.preventDefault()
    onclose()
  }}
>
  <div class="head">
    <div class="title">
      <h1 id="dialog-title">{title}</h1>
      {@render badge?.()}
    </div>
    <button class="close" aria-label="Close dialog" onclick={onclose}>×</button>
  </div>
  {@render children()}
</dialog>

<style>
  dialog {
    max-width: calc(100vw - 48px);
    padding: 0;
    border: none;
    border-radius: 8px;
    background: var(--bg-main);
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 30px 80px rgba(0, 0, 0, 0.6);
  }

  dialog.full[open] {
    width: calc(100vw - 48px);
    height: calc(100vh - 48px);
    max-height: none;
    display: flex;
    flex-direction: column;
  }

  dialog::backdrop {
    background: var(--overlay);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 22px 6px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }

  .close {
    width: 32px;
    height: 32px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-2);
  }
</style>
