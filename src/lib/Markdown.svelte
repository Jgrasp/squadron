<script lang="ts" module>
  import MarkdownIt from 'markdown-it'

  const md = new MarkdownIt({ html: false, linkify: true }).disable('image')
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
</script>

<div class="prose" class:streaming>{@html html}</div>

<style>
  .prose {
    line-height: 1.65;
    overflow-wrap: anywhere;
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
