<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { languageOf } from './lib/CodeEditor.svelte'
  import { fenced } from './lib/filelink'
  import { errorText, t } from './lib/i18n/index.svelte'
  import Markdown from './lib/Markdown.svelte'

  const query = new URLSearchParams(location.search)
  const project = query.get('project') ?? ''
  const path = query.get('path') ?? ''
  const name = path.split('/').pop() ?? path

  let text = $state<string | null>()
  let error = $state('')

  function load() {
    invoke<string | null>('read_project_file', { project, path })
      .then((found) => ((text = found), (error = '')))
      .catch((e) => (error = errorText(e)))
  }

  load()
  getCurrentWindow().onFocusChanged(({ payload }) => payload && load())
</script>

<main>
  {#if error}
    <p class="note">{error}</p>
  {:else if text === null}
    <p class="note">{t('conversation.fileMissing')}</p>
  {:else if text === undefined}
    <p class="note">{t('common.loading')}</p>
  {:else}
    <Markdown text={fenced(text, languageOf(name)?.name.toLowerCase())} />
  {/if}
</main>

<style>
  main {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: auto;
    padding: 16px;
    background: var(--bg-main);
  }

  main > :global(.prose) {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  main :global(.code) {
    flex: 1;
    margin: 0;
  }

  main :global(.rendered),
  main :global(.code pre) {
    max-height: none;
  }

  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-dim);
  }
</style>
