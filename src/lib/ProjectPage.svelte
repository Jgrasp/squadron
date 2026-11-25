<script lang="ts">
  import type { Snippet } from 'svelte'

  let { children }: { children: Snippet } = $props()
</script>

<div class="project">{@render children()}</div>

<style>
  .project {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .project :global(section) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .project :global(h4) {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .project :global(.intro) {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }

  .project :global(.options) {
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }

  .project :global(.option) {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    padding: 8px 14px;
  }

  .project :global(.option + .option) {
    border-top: 1px solid var(--border);
  }

  .project :global(.option.sub) {
    min-height: 44px;
    padding-left: 40px;
  }

  .project :global(.option.column) {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
    padding: 12px 14px 14px;
  }

  .project :global(.option.column.sub) {
    padding-left: 40px;
  }

  .project :global(.option .label) {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .project :global(.option strong) {
    font-size: 13px;
    font-weight: 600;
  }

  .project :global(.option small) {
    color: var(--text-dim);
    font-size: 12px;
  }

  .project :global(.option .field),
  .project :global(.option .select) {
    flex: none;
    width: 260px;
  }

  .project :global(.option .field) {
    height: 32px;
    padding: 0 10px;
    font-size: 12.5px;
  }

  .project :global(.option .prompt) {
    width: auto;
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }

  .project :global(.option .add) {
    display: flex;
    gap: 8px;
  }

  .project :global(.option .add .field) {
    flex: 1;
    width: auto;
  }

  .project :global(.option .btn) {
    flex: none;
    height: 32px;
  }

  .project :global(.switch) {
    appearance: none;
    flex: none;
    position: relative;
    width: 32px;
    height: 18px;
    margin: 0;
    border-radius: 999px;
    background: var(--surface-5);
    cursor: pointer;
    transition: background 0.15s;
  }

  .project :global(.switch::before) {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s;
  }

  .project :global(.switch:checked) {
    background: var(--accent);
  }

  .project :global(.switch:checked::before) {
    transform: translateX(14px);
    background: var(--ink);
  }

  .project :global(.switch:disabled) {
    cursor: default;
    opacity: 0.5;
  }

  .project :global(.switch:focus-visible) {
    outline: 2px solid var(--accent-soft);
    outline-offset: 2px;
  }
</style>
