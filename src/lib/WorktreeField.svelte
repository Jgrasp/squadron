<script lang="ts">
  let { name = $bindable(), suggestion }: { name: string | null; suggestion: string } = $props()

  const slug = (text: string) =>
    text
      .toLowerCase()
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 40) || 'agent'
</script>

<div class="worktree">
  <label class="check">
    <input type="checkbox" checked={name !== null} onchange={(e) => (name = e.currentTarget.checked ? slug(suggestion) : null)} />
    <span>
      <strong>Work in a new git worktree</strong>
      <small>Claude Code creates <span class="mono">.claude/worktrees/{name || '…'}</span> on the branch <span class="mono">worktree-{name || '…'}</span>, and the agent works there.</small>
    </span>
  </label>
  {#if name !== null}
    <input class="field mono" aria-label="Worktree name" value={name} oninput={(e) => (name = e.currentTarget.value)} onchange={(e) => (name = slug(e.currentTarget.value))} />
  {/if}
</div>

<style>
  .worktree {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .check {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    font-weight: 400;
    cursor: pointer;
  }

  .check input {
    margin-top: 3px;
    accent-color: var(--accent);
  }

  .check > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check strong {
    font-weight: 600;
  }

  .check small {
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-dim);
  }

  .field {
    height: 36px;
    margin-left: 23px;
    width: calc(100% - 23px);
  }
</style>
