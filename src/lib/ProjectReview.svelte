<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { editProject, type Project } from './projects.svelte'
  import { REVIEW_PROMPT } from './pulls.svelte'
  import { settings } from './settings.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()
</script>

<p class="hint">{t('review.projectPromptHint')}</p>
<label class="toggle">
  <input
    type="checkbox"
    checked={!!project.review_prompt}
    onchange={(e) => act(() => editProject(project.path, { review_prompt: e.currentTarget.checked ? settings.review_prompt || REVIEW_PROMPT : '' }))}
  />
  {t('review.projectPrompt')}
</label>
{#if project.review_prompt}
  <textarea
    class="field prompt"
    rows="8"
    aria-label={t('review.projectPromptOf', { name: project.name })}
    value={project.review_prompt}
    onchange={(e) => act(() => editProject(project.path, { review_prompt: e.currentTarget.value }))}
  ></textarea>
{/if}

<style>
  .hint {
    margin: 0;
    max-width: 620px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: 8px;
    cursor: pointer;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  .prompt {
    align-self: stretch;
    height: auto;
    max-width: 620px;
    padding: 10px 12px;
    resize: vertical;
    font-size: 12.5px;
    line-height: 1.5;
  }
</style>
