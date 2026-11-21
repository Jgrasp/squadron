<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { editProject, type Project } from './projects.svelte'
  import { REVIEW_PROMPT } from './pulls.svelte'
  import { settings } from './settings.svelte'

  let { project, act }: { project: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()
</script>

<section>
  <h4>{t('settings.projectReview')}</h4>
  <div class="options">
    <label class="option">
      <span class="label">
        <strong>{t('review.autoReview')}</strong>
        <small>{settings.pilot ? t('review.autoReviewHint') : t('review.askAgentNeedsPilot')}</small>
      </span>
      <input type="checkbox" class="switch" checked={project.auto_review} onchange={(e) => act(() => editProject(project.path, { auto_review: e.currentTarget.checked }))} />
    </label>
    <label class="option">
      <span class="label">
        <strong>{t('review.projectPrompt')}</strong>
        <small>{t('review.projectPromptHint')}</small>
      </span>
      <input
        type="checkbox"
        class="switch"
        checked={!!project.review_prompt}
        onchange={(e) => act(() => editProject(project.path, { review_prompt: e.currentTarget.checked ? settings.review_prompt || REVIEW_PROMPT : '' }))}
      />
    </label>
    {#if project.review_prompt}
      <div class="option column">
        <textarea
          class="field prompt"
          rows="8"
          aria-label={t('review.projectPromptOf', { name: project.name })}
          value={project.review_prompt}
          onchange={(e) => act(() => editProject(project.path, { review_prompt: e.currentTarget.value }))}
        ></textarea>
      </div>
    {/if}
  </div>
</section>

<style>
  .option.column .prompt {
    width: auto;
    height: auto;
    padding: 10px 12px;
    resize: vertical;
    line-height: 1.5;
  }
</style>
