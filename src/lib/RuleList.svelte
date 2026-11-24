<script lang="ts">
  import { removeRule, type Project } from './projects.svelte'
  import { t } from './i18n/index.svelte'
  import { ruleLabel, ruleTool } from './rules'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  let whole = $state<string | null>(null)
</script>

<ul>
  {#each project.rules as rule (rule)}
    <li>
      <button class="rule" class:whole={whole === rule} aria-expanded={whole === rule} onclick={() => (whole = whole === rule ? null : rule)}>
        <span class="tool">{ruleTool(rule)}</span>
        <code>{ruleLabel(rule)}</code>
      </button>
      <button class="remove" aria-label={t('common.remove')} title={t('common.remove')} onclick={() => act(() => removeRule(project.path, rule))}>×</button>
    </li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
    border-top: 1px solid var(--border);
  }

  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding-right: 8px;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  li:hover {
    background: var(--surface-2);
  }

  .rule {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 36px;
    padding: 0 0 0 14px;
    text-align: left;
  }

  .tool {
    flex: none;
    width: 38px;
    font-size: 11.5px;
    color: var(--text-dim);
  }

  code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: 12px/1.5 var(--mono);
    color: var(--text-2);
  }

  .whole code {
    padding: 8px 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .remove {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 16px;
  }

  .remove:hover {
    background: var(--surface-3);
    color: var(--text-strong);
  }
</style>
