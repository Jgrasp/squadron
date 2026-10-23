<script lang="ts" module>
  import MarkdownIt from 'markdown-it'

  const md = new MarkdownIt({ html: false, linkify: true }).disable('image')
  const fence = md.renderer.rules.fence!
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const lang = md.utils.escapeHtml(tokens[idx].info.trim().split(/\s+/)[0] || 'text')
    return `<div class="code"><div class="code-head"><span>${lang}</span><button type="button" class="copy" data-copy>Copy</button></div>${fence(tokens, idx, options, env, self)}</div>`
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte'

  let { text, streaming = false }: { text: string; streaming?: boolean } = $props()

  let shown = $state(untrack(() => text.length))
  let frame = 0

  $effect(() => {
    const target = text.length
    if (!streaming || untrack(() => shown) > target) {
      shown = target
      return
    }
    if (frame) return
    const step = () => {
      const left = text.length - shown
      if (left <= 0) {
        frame = 0
        return
      }
      shown += Math.max(1, Math.ceil(left / 10))
      frame = requestAnimationFrame(step)
    }
    frame = requestAnimationFrame(step)
    return () => {
      cancelAnimationFrame(frame)
      frame = 0
    }
  })

  const html = $derived(md.render(streaming ? text.slice(0, shown) : text))

  function copy(event: MouseEvent) {
    const button = (event.target as HTMLElement).closest<HTMLButtonElement>('[data-copy]')
    const code = button?.closest('.code')?.querySelector('pre')?.textContent
    if (!button || code === undefined || code === null) return
    navigator.clipboard.writeText(code.replace(/\n$/, ''))
    button.textContent = 'Copied'
    setTimeout(() => (button.textContent = 'Copy'), 1500)
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="prose" class:streaming onclick={copy}>{@html html}</div>

<style>
  .prose {
    line-height: 1.65;
    overflow-wrap: anywhere;
  }

  .prose :global(.code) {
    margin: 0 0 10px;
    border-radius: 6px;
    background: var(--surface-1);
    box-shadow: inset 0 0 0 1px var(--border);
    overflow: hidden;
  }

  .prose :global(.code-head) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 6px 4px 12px;
    border-bottom: 1px solid var(--border);
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .prose :global(.copy) {
    height: 24px;
    padding: 0 10px;
    border-radius: 4px;
    font: 600 11.5px var(--sans);
    color: var(--text-2);
  }

  .prose :global(.copy:hover) {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .prose :global(.code pre) {
    margin: 0;
    max-height: 420px;
    overflow: auto;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
  }

  .prose :global(> :first-child) {
    margin-top: 0;
  }

  .prose :global(> :last-child) {
    margin-bottom: 0;
  }

  .prose :global(p),
  .prose :global(ul),
  .prose :global(ol),
  .prose :global(pre),
  .prose :global(blockquote),
  .prose :global(table) {
    margin: 0 0 10px;
  }

  .prose :global(h1),
  .prose :global(h2),
  .prose :global(h3),
  .prose :global(h4) {
    margin: 14px 0 6px;
    font-size: 15.5px;
    font-weight: 700;
    line-height: 1.4;
  }

  .prose :global(h1) {
    font-size: 17px;
  }

  .prose :global(ul),
  .prose :global(ol) {
    padding-left: 18px;
  }

  .prose :global(li + li) {
    margin-top: 2px;
  }

  .prose :global(code) {
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--surface-3);
    font-size: 12.5px;
  }

  .prose :global(pre) {
    padding: 12px 14px;
    border-radius: 4px;
    background: var(--bg-main);
    box-shadow: inset 0 0 0 1px var(--border);
    overflow-x: auto;
    line-height: 1.55;
  }

  .prose :global(pre code) {
    padding: 0;
    background: none;
    font-size: 12.5px;
  }

  .prose :global(blockquote) {
    padding-left: 12px;
    border-left: 2px solid var(--border-strong);
    color: var(--text-muted);
  }

  .prose :global(hr) {
    border: none;
    height: 1px;
    background: var(--border-strong);
  }

  .prose :global(table) {
    border-collapse: collapse;
    font-size: 13px;
  }

  .prose :global(th),
  .prose :global(td) {
    padding: 5px 10px;
    border: 1px solid var(--border-strong);
    text-align: left;
  }

  .prose :global(th) {
    background: var(--surface-2);
    font-weight: 600;
  }

  .streaming :global(> :last-child::after) {
    content: '';
    display: inline-block;
    width: 7px;
    height: 15px;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--accent);
    animation: blink 1s steps(1) infinite;
  }
</style>
