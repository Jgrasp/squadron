<script lang="ts">
  import { mount, unmount } from 'svelte'
  import { COLUMNS } from './board'
  import { find } from './agents.svelte'
  import { projects } from './projects.svelte'
  import { pullOf } from './pulls.svelte'
  import { codeOf, nameOf } from './taskref'
  import { t } from './i18n/index.svelte'
  import StatusIcon from './StatusIcon.svelte'
  import TaskDialog from './TaskDialog.svelte'

  let { id, project, host }: { id: string; project: string; host: HTMLElement } = $props()

  const card = $derived(projects.list.find((p) => p.path === project)?.cards.find((c) => c.id === id))
  const agent = $derived(card?.agent ? find(card.agent) : undefined)
  const pull = $derived(agent ? pullOf(agent) : null)
  $effect(() => {
    const click = (event: MouseEvent) => {
      if ((event.target as HTMLElement).closest('a, button')) return
      const dialog = mount(TaskDialog, { target: document.body, props: { id, onclose: () => unmount(dialog) } })
    }
    host.addEventListener('click', click)
    return () => host.removeEventListener('click', click)
  })
</script>

{#if card}
  {#if codeOf(card.title)}<span class="code">{codeOf(card.title)}</span>{/if}
  <span class="head" class:wide={!codeOf(card.title)}>
    <strong>{nameOf(card.title)}</strong>
    <span class="meta">
      {#if agent}<StatusIcon status={agent.status} />{/if}
      {[t(COLUMNS.find(([c]) => c === card.column)![1]), agent?.name, pull?.state === 'open' ? `PR #${pull.number}` : '', card.ticket?.key].filter(Boolean).join(' · ')}
    </span>
  </span>
{/if}

<style>
  .code {
    grid-row: span 2;
    align-self: start;
    margin-top: 2px;
    padding: 2px 7px;
    border-radius: 4px;
    background: var(--surface-3);
    font: 12px var(--mono);
    color: var(--text-strong);
  }

  .head {
    grid-column: 2;
    display: flex;
    flex-direction: column;
  }

  .wide {
    grid-column: 1 / -1;
  }

  strong {
    color: var(--text-strong);
    font-weight: 600;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }
</style>
