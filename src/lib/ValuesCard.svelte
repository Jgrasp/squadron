<script lang="ts">
  import { giveValues, type ValueField } from './agents.svelte'
  import { t } from './i18n/index.svelte'
  import type { Agent, Ask } from './stream'

  let { agent, ask }: { agent: Agent; ask: Ask } = $props()

  const fields = $derived((ask.input.fields ?? []) as ValueField[])
  let values = $state<Record<string, string>>({})
  let shown = $state<Record<string, boolean>>({})
  const filled = $derived(fields.filter((f) => values[f.key]))

  function save() {
    if (!filled.length) return
    giveValues(agent, ask, Object.fromEntries(filled.map((f) => [f.key, values[f.key]])))
  }

  function focus(node: HTMLInputElement, first: boolean) {
    if (first) node.focus()
  }
</script>

<section class="card" aria-label={t('agent.valuesTitle', { count: fields.length })}>
  <div class="head">
    <span class="icon" aria-hidden="true">*</span>
    <div class="titles">
      <span class="title">{t('agent.valuesTitle', { count: fields.length })}</span>
      <span class="path">{ask.input.path}</span>
    </div>
  </div>

  <form
    class="fields"
    onsubmit={(e) => {
      e.preventDefault()
      save()
    }}
  >
    {#each fields as field, i (field.key)}
      <label class="entry">
        <span class="legend">
          <span class="key">{field.key}</span>
          {#if field.hint}<span class="hint">{field.hint}</span>{/if}
        </span>
        <span class="input">
          <input
            type={field.secret && !shown[field.key] ? 'password' : 'text'}
            bind:value={values[field.key]}
            autocomplete="off"
            spellcheck="false"
            use:focus={i === 0}
          />
          {#if field.secret}
            <button type="button" class="reveal" onclick={() => (shown[field.key] = !shown[field.key])}>
              {shown[field.key] ? t('agent.valueHide') : t('agent.valueShow')}
            </button>
          {/if}
        </span>
      </label>
    {/each}
    <div class="footer">
      <button class="btn save" aria-disabled={!filled.length}>{t('agent.valuesSave')}</button>
      <button type="button" class="btn cancel" onclick={() => giveValues(agent, ask, '')}>{t('common.cancel')}</button>
      <span class="hint">{t('agent.valuesHint', { agent: agent.name })}</span>
    </div>
  </form>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
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
    font: 700 18px var(--mono);
  }

  .titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--wait-text);
  }

  .path {
    font: 12px var(--mono);
    color: var(--wait-muted);
    overflow-wrap: anywhere;
  }

  .fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .entry {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .legend {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }

  .key {
    font: 600 13px var(--mono);
    color: var(--wait-text);
  }

  .hint {
    font-size: 12px;
    color: var(--wait-muted);
  }

  .input {
    display: flex;
    align-items: center;
    border-radius: 4px;
    background: var(--option-off);
    box-shadow: inset 0 0 0 1px var(--wait-border);
  }

  .input:focus-within {
    box-shadow: inset 0 0 0 2px var(--accent-soft);
  }

  input {
    flex: 1;
    min-width: 0;
    height: 36px;
    padding: 0 12px;
    border: 0;
    outline: none;
    background: none;
    color: var(--wait-text);
    font: 13px var(--mono);
  }

  .reveal {
    height: 28px;
    margin-right: 4px;
    padding: 0 10px;
    border-radius: 3px;
    color: var(--wait-text-2);
    font-size: 12px;
  }

  .reveal:hover {
    background: var(--option-on);
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 2px;
  }

  .save {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }

  .save[aria-disabled='true'] {
    background: var(--wait-btn);
    color: #d9b6a3;
    opacity: 1;
  }

  .cancel {
    background: var(--surface-3);
    color: #e3d7cf;
  }
</style>
