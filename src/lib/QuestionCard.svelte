<script lang="ts">
  import { answer } from './agents.svelte'
  import { t } from './i18n/index.svelte'
  import type { Agent, Ask } from './stream'

  let { agent, ask }: { agent: Agent; ask: Ask } = $props()

  type Option = { label: string; description?: string }
  type Question = { question: string; header?: string; multiSelect?: boolean; options: Option[] }

  const questions = $derived((ask.input.questions ?? []) as Question[])
  const simple = $derived(questions.length === 1 && !questions[0].multiSelect)
  let picked = $state<Record<string, string[]>>({})
  const done = $derived(questions.filter((q) => picked[q.question]?.length).length)

  const clean = (label: string) => label.replace(/\s*\((recommended|recommandé)\)\s*$/i, '')
  const recommended = (label: string) => /\((recommended|recommandé)\)\s*$/i.test(label)

  function choose(q: Question, label: string) {
    if (simple) return answer(agent, ask, { [q.question]: label })
    const current = picked[q.question] ?? []
    picked[q.question] = q.multiSelect
      ? current.includes(label)
        ? current.filter((l) => l !== label)
        : [...current, label]
      : [label]
  }

  function send() {
    if (done < questions.length) return
    answer(agent, ask, Object.fromEntries(questions.map((q) => [q.question, picked[q.question].join(', ')])))
  }

  function keys(event: KeyboardEvent) {
    if (!simple || (event.target as HTMLElement).closest('input, textarea, [contenteditable]')) return
    const option = questions[0].options[Number(event.key) - 1]
    if (option) choose(questions[0], option.label)
  }
</script>

<svelte:window onkeydown={keys} />

{#snippet choice(option: Option)}
  <span class="text">
    <span class="label">{clean(option.label)}</span>
    {#if option.description}<span>{option.description}</span>{/if}
  </span>
  {#if recommended(option.label)}<span class="badge">{t('common.recommended')}</span>{/if}
{/snippet}

<section class="card" aria-label={t('agent.agentQuestion')}>
  <div class="head">
    <span class="icon" aria-hidden="true">?</span>
    <span class="title">{simple ? questions[0].question : t('agent.questionsFrom', { count: questions.length, agent: agent.name })}</span>
    {#if !simple}<span class="count">{t('agent.answeredCount', { done, total: questions.length })}</span>{/if}
  </div>

  {#if simple}
    <div class="options" role="group" aria-label={t('agent.answerChoices')}>
      {#each questions[0].options as option, i (option.label)}
        <button class="option" onclick={() => choose(questions[0], option.label)}>
          <span class="key">{i + 1}</span>
          <span class="text">
            <strong>{clean(option.label)}</strong>
            {#if option.description}<span>{option.description}</span>{/if}
          </span>
          {#if recommended(option.label)}<span class="badge">{t('common.recommended')}</span>{/if}
        </button>
      {/each}
    </div>
    <span class="hint">{t('agent.oneClickHint')}</span>
  {:else}
    {#each questions as q, n (q.question)}
      <div class="question" role="group" aria-label={q.question} class:current={!picked[q.question]?.length && questions.findIndex((x) => !picked[x.question]?.length) === n}>
        <div class="legend">
          <span>{n + 1} · {q.question}</span>
          {#if picked[q.question]?.length}
            <span class="ok">{t('agent.answered')}</span>
          {:else if q.multiSelect}
            <span class="multi">{t('agent.severalChoices')}</span>
          {/if}
        </div>
        <div class="choices" class:grid={q.multiSelect} class:described={q.options.some((o) => o.description)}>
          {#each q.options as option (option.label)}
            {#if q.multiSelect}
              <label class:on={picked[q.question]?.includes(option.label)}>
                <input
                  type="checkbox"
                  checked={picked[q.question]?.includes(option.label)}
                  onchange={() => choose(q, option.label)}
                />
                {@render choice(option)}
              </label>
            {:else}
              <button
                aria-pressed={picked[q.question]?.includes(option.label)}
                onclick={() => choose(q, option.label)}
              >
                {@render choice(option)}
              </button>
            {/if}
          {/each}
        </div>
      </div>
    {/each}
    <div class="footer">
      <button class="btn send" aria-disabled={done < questions.length} onclick={send}>{t('agent.sendAnswers')}</button>
      <span class="hint">{t('agent.answerEveryHint')}</span>
    </div>
  {/if}
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    border-radius: 6px;
    background: var(--wait-bg);
    box-shadow: 0 0 0 1px var(--wait-border);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 16px;
    font-weight: 700;
  }

  .title {
    flex: 1;
    font-size: 15px;
    font-weight: 600;
    color: var(--wait-text);
  }

  .count {
    font-size: 12.5px;
    color: var(--wait-text-2);
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    border-radius: 4px;
    background: var(--option);
    color: var(--wait-text);
    text-align: left;
  }

  .option:hover {
    background: var(--option-on);
  }

  .key {
    width: 22px;
    height: 22px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    background: var(--wait-btn);
    color: var(--wait-btn-text);
    font: 12px var(--mono);
  }

  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .text strong {
    font-weight: 600;
  }

  .text span {
    font-size: 12.5px;
    color: var(--wait-muted);
  }

  .badge {
    padding: 3px 8px;
    border-radius: 3px;
    background: var(--wait-btn);
    color: var(--wait-btn-text);
    font-size: 11.5px;
    font-weight: 600;
  }

  .hint {
    font-size: 12.5px;
    color: var(--wait-muted);
  }

  .question {
    margin: 0;
    padding: 12px 14px;
    border: none;
    border-radius: 4px;
    background: var(--wait-note);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .question.current {
    background: var(--option);
    box-shadow: inset 0 0 0 1px #a55e37;
  }

  .legend {
    padding: 0 0 4px;
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-weight: 600;
    color: var(--wait-text);
  }

  .ok {
    font-size: 12px;
    color: var(--ok-text);
  }

  .multi {
    font-size: 12px;
    font-weight: 500;
    color: var(--wait-text-2);
  }

  .choices {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .choices.grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .choices.described {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  }

  .choices.described button,
  .choices.described label {
    align-items: flex-start;
    padding: 9px 12px;
    text-align: left;
  }

  .choices.described input {
    margin-top: 2px;
  }

  .choices .label {
    color: inherit;
    font-size: inherit;
  }

  .choices .text span:not(.label) {
    font-weight: 400;
  }

  .choices button,
  .choices label {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--option-off);
    color: var(--wait-muted);
    cursor: pointer;
  }

  .choices button[aria-pressed='true'],
  .choices label.on {
    background: var(--option-on);
    box-shadow: inset 0 0 0 2px var(--accent-soft);
    color: var(--wait-text);
    font-weight: 600;
  }

  .choices input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--accent);
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .send {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .send[aria-disabled='true'] {
    background: var(--wait-btn);
    color: #d9b6a3;
    opacity: 1;
  }
</style>
