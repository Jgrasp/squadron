<script lang="ts">
  import Markdown from './Markdown.svelte'
  import { locale } from './i18n/index.svelte'

  const files = import.meta.glob<string>('../../CHANGELOG*.md', { query: '?raw', import: 'default', eager: true })

  const text = $derived((files[`../../CHANGELOG.${locale()}.md`] ?? files['../../CHANGELOG.md']).replace(/^# .*\n/, ''))
</script>

<div class="changelog">
  <Markdown {text} />
</div>

<style>
  .changelog :global(h2) {
    color: var(--accent);
  }

  .changelog :global(h3) {
    font-size: 13px;
    color: var(--text-muted);
  }

  .changelog :global(h2:not(:first-child)) {
    margin-top: 28px;
    padding-top: 20px;
    border-top: 1px solid var(--border);
  }
</style>
