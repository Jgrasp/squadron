<script lang="ts">
  import { t } from './i18n/index.svelte'
  import { editProject, type Project } from './projects.svelte'
  import { REVIEW_PROMPT } from './pulls.svelte'
  import { saveSettings, settings } from './settings.svelte'

  let { project, act }: { project?: Project; act: (work: () => Promise<unknown>) => Promise<void> } = $props()

  const change = (review_prompt: string) => act(() => saveSettings({ ...$state.snapshot(settings), review_prompt }))
</script>

<section>
  <h4>{t('settings.projectReview')}</h4>
  {#if !project}
    <div class="options">
      <div class="option column">
        <span class="label">
          <strong>{t('review.promptTitle')}</strong>
          <small>{t('review.promptIntro')}</small>
        </span>
        <textarea
          class="field prompt"
          rows="5"
          aria-label={t('review.promptTitle')}
          value={settings.review_prompt || REVIEW_PROMPT}
          onchange={(e) => change(e.currentTarget.value.trim() === REVIEW_PROMPT ? '' : e.currentTarget.value.trim())}
        ></textarea>
        {#if settings.review_prompt}
          <span><button class="btn btn-ghost" onclick={() => change('')}>{t('review.promptReset')}</button></span>
        {/if}
      </div>
    </div>
  {:else}
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
  {/if}
</section>
