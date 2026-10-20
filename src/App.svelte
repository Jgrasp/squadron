<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'

  let dir = $state('')
  let text = $state('')
  let error = $state('')
  let lines: string[] = $state([])

  listen<string>('claude', (event) => lines.push(event.payload))

  async function send(event: SubmitEvent) {
    event.preventDefault()
    error = ''
    try {
      await invoke('send', { dir, text })
      text = ''
    } catch (e) {
      error = String(e)
    }
  }
</script>

<main>
  <form onsubmit={send}>
    <label>Dossier <input bind:value={dir} placeholder="/chemin/du/projet" required /></label>
    <label>Message <input bind:value={text} required /></label>
    <button>Envoyer</button>
    <button type="button" onclick={() => invoke('interrupt')}>Interrompre</button>
  </form>
  {#if error}<p role="alert">{error}</p>{/if}
  <pre>{lines.join('\n')}</pre>
</main>
