<script lang="ts" module>
  import { LanguageDescription, type Language } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { classHighlighter, highlightCode } from '@lezer/highlight'
  import MarkdownIt from 'markdown-it'
  import { t } from './i18n/index.svelte'
  import { isImage, preview } from './Attachment.svelte'
  import { isPullLink } from './linked.svelte'

  const grammars = new Map<string, Language | null>()
  let loaded = $state(0)

  function colored(code: string, lang: string) {
    if (!grammars.has(lang)) {
      const found = LanguageDescription.matchLanguageName(languages, lang, true)
      grammars.set(lang, null)
      found?.load().then((support) => {
        grammars.set(lang, support.language)
        loaded++
      })
    }
    const grammar = grammars.get(lang)
    if (!grammar) return ''
    let html = ''
    const escape = md.utils.escapeHtml
    highlightCode(code, grammar.parser.parse(code), classHighlighter, (text, classes) => (html += classes ? `<span class="${classes}">${escape(text)}</span>` : escape(text)), () => (html += '\n'))
    return html
  }

  function localFile(href: string) {
    let path = href.replace(/^file:\/\//, '')
    try {
      path = decodeURI(path)
    } catch {}
    return path.startsWith('/') && /\/[^/]+\.[a-z0-9]{1,8}$/i.test(path) ? path : ''
  }

  let theme = ''

  function themeCss() {
    if (theme) return theme
    const rules: string[] = []
    for (const sheet of document.styleSheets)
      try {
        for (const rule of sheet.cssRules) if (rule instanceof CSSFontFaceRule || (rule instanceof CSSStyleRule && rule.selectorText === ':root')) rules.push(rule.cssText)
      } catch {}
    return (theme = rules.join('\n'))
  }

  const page = (code: string) =>
    `<!doctype html><html><head><meta charset="utf-8"><style>${themeCss()}\nhtml{color-scheme:dark}body{margin:0;padding:16px;background:var(--bg-main);color:var(--text);font:13.5px/1.5 var(--sans)}</style></head><body>${code}</body></html>`

  const md = new MarkdownIt({ html: false, linkify: true, highlight: (code, lang) => (lang ? colored(code, lang) : '') })
  md.core.ruler.push('pull_cards', (state) => {
    const tokens = state.tokens
    tokens.forEach((token, i) => {
      if (token.type !== 'inline' || !token.children) return
      const kids = token.children.filter((c) => c.type !== 'softbreak' && !(c.type === 'text' && !c.content.trim()))
      const href = String(kids[0]?.attrGet('href') ?? '')
      const alone = kids.length === 3 && kids[0].type === 'link_open' && kids[2].type === 'link_close' && tokens[i - 1]?.type === 'paragraph_open'
      if (alone && isPullLink(href)) {
        tokens[i - 1].tag = tokens[i + 1].tag = 'div'
        tokens[i - 1].attrSet('class', 'pr-slot')
        tokens[i - 1].attrSet('data-pr', href)
        return
      }
      const file = localFile(href)
      if (alone && file) {
        tokens[i - 1].tag = tokens[i + 1].tag = 'div'
        tokens[i - 1].hidden = tokens[i + 1].hidden = false
        tokens[i - 1].attrSet('class', 'file-slot')
        tokens[i - 1].attrSet('data-file', file)
        return
      }
      for (const c of token.children) if (c.type === 'link_open' && isPullLink(String(c.attrGet('href') ?? ''))) c.attrSet('data-pr-inline', '')
    })
  })

  const shots = new Map<string, string>()
  md.renderer.rules.image = (tokens, idx) => {
    const alt = md.utils.escapeHtml(tokens[idx].content)
    let path = String(tokens[idx].attrGet('src') ?? '')
    try {
      path = decodeURI(path)
    } catch {}
    if (!path.startsWith('/') || !isImage(path)) return alt
    const at = md.utils.escapeHtml(path)
    return `<button type="button" class="shot" data-shot="${at}" title="${at}"><img alt="${alt}"></button>`
  }
  const fence = md.renderer.rules.fence!
  const copyButton = () => `<button type="button" class="copy" data-copy><svg width="13" height="13" viewBox="0 0 24 24" aria-hidden="true"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/></svg><span>${t('common.copy')}</span></button>`
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const lang = tokens[idx].info.trim().split(/\s+/)[0] || 'text'
    const source = fence(tokens, idx, options, env, self)
    if (lang === 'html') {
      const tabs = `<span class="views"><button type="button" data-view="preview" aria-pressed="true">${t('conversation.preview')}</button><button type="button" data-view="source" aria-pressed="false">HTML</button></span>`
      return `<div class="code"><div class="code-head">${tabs}${copyButton()}</div><div class="rendered mockup" data-mockup></div><div class="source" hidden>${source}</div></div>`
    }
    if (lang !== 'markdown' && lang !== 'md')
      return `<div class="code"><div class="code-head"><span>${md.utils.escapeHtml(lang)}</span>${copyButton()}</div>${source}</div>`
    const tabs = `<span class="views"><button type="button" data-view="preview" aria-pressed="true">${t('conversation.preview')}</button><button type="button" data-view="source" aria-pressed="false">${t('conversation.markdown')}</button></span>`
    return `<div class="code"><div class="code-head">${tabs}${copyButton()}</div><div class="rendered">${md.render(tokens[idx].content)}</div><div class="source" hidden>${source}</div></div>`
  }
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { mount, tick, unmount, untrack } from 'svelte'
  import PullCard from './PullCard.svelte'
  import Attachment from './Attachment.svelte'

  let { text, streaming = false, cards = false }: { text: string; streaming?: boolean; cards?: boolean } = $props()

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

  const html = $derived.by(() => {
    void loaded
    return md.render(streaming ? text.slice(0, shown) : text)
  })

  let box = $state<HTMLElement>()

  $effect(() => {
    void html
    tick().then(() => {
      for (const shot of box?.querySelectorAll<HTMLElement>('[data-shot]') ?? []) {
        const path = shot.dataset.shot!
        const img = shot.querySelector('img')!
        if (shots.has(path)) img.src = shots.get(path)!
        else
          preview(path)
            .then((data) => (shots.set(path, data), (img.src = data)))
            .catch(() => shot.classList.add('gone'))
      }
    })
  })

  $effect(() => {
    void html
    if (streaming) return
    const watching: ResizeObserver[] = []
    tick().then(() => {
      for (const slot of box?.querySelectorAll<HTMLElement>('[data-mockup]') ?? []) {
        const frame = document.createElement('iframe')
        frame.setAttribute('sandbox', 'allow-same-origin')
        frame.title = t('conversation.mockup')
        frame.srcdoc = page(slot.parentElement?.querySelector(':scope > .source code')?.textContent ?? '')
        frame.addEventListener('load', () => {
          const doc = frame.contentDocument
          if (!doc) return
          const fit = () => (frame.style.height = `${doc.documentElement.scrollHeight}px`)
          fit()
          const watch = new ResizeObserver(fit)
          watch.observe(doc.body)
          watching.push(watch)
        })
        slot.replaceChildren(frame)
      }
    })
    return () => watching.forEach((w) => w.disconnect())
  })

  $effect(() => {
    void html
    if (streaming || !cards) return
    const mounted: ReturnType<typeof mount>[] = []
    const added: Element[] = []
    tick().then(() => {
      for (const slot of box?.querySelectorAll<HTMLElement>('[data-file]') ?? []) {
        slot.querySelector('a')?.setAttribute('hidden', '')
        mounted.push(mount(Attachment, { target: slot, props: { path: slot.dataset.file!, look: 'card', large: true } }))
      }
      for (const slot of box?.querySelectorAll<HTMLElement>('[data-pr]') ?? [])
        mounted.push(mount(PullCard, { target: slot, props: { url: slot.dataset.pr!, onready: () => slot.classList.add('ready') } }))
      const shown = new Set([...(box?.querySelectorAll<HTMLElement>('[data-pr]') ?? [])].map((slot) => slot.dataset.pr))
      const last = new Map<Element, Element>()
      for (const link of box?.querySelectorAll<HTMLAnchorElement>('a[data-pr-inline]') ?? []) {
        const href = link.getAttribute('href') ?? link.href
        if (!shown.has(href) && shown.size < 4) {
          shown.add(href)
          let top: Element = link
          while (top.parentElement && top.parentElement !== box) top = top.parentElement
          const slot = document.createElement('div')
          slot.className = 'pr-unfurl'
          ;(last.get(top) ?? top).after(slot)
          last.set(top, slot)
          added.push(slot)
          mounted.push(mount(PullCard, { target: slot, props: { url: href } }))
        }
        const spot = document.createElement('span')
        link.before(spot)
        mounted.push(mount(PullCard, { target: spot, props: { url: link.href, inline: true, short: shown.has(href), onready: () => (link.hidden = true) } }))
      }
    })
    return () => {
      mounted.forEach((m) => unmount(m))
      added.forEach((el) => el.remove())
    }
  })

  function click(event: MouseEvent) {
    const target = event.target as HTMLElement
    const shot = target.closest<HTMLElement>('[data-shot]')
    if (shot) return void invoke('open_attachment', { path: shot.dataset.shot }).catch(() => shot.classList.add('gone'))
    const block = target.closest('.code')
    const view = target.closest<HTMLButtonElement>('[data-view]')
    if (block && view) {
      for (const tab of block.querySelectorAll(':scope > .code-head [data-view]')) tab.setAttribute('aria-pressed', String(tab === view))
      block.querySelector(':scope > .rendered')?.toggleAttribute('hidden', view.dataset.view !== 'preview')
      block.querySelector(':scope > .source')?.toggleAttribute('hidden', view.dataset.view !== 'source')
      return
    }
    const button = target.closest<HTMLButtonElement>('[data-copy]')
    const code = block?.querySelector(':scope > pre, :scope > .source > pre')?.textContent
    if (!button || code === undefined || code === null) return
    navigator.clipboard.writeText(code.replace(/\n$/, ''))
    const label = button.querySelector('span')!
    label.textContent = t('common.copied')
    setTimeout(() => (label.textContent = t('common.copy')), 1500)
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="prose" class:streaming onclick={click} bind:this={box}>{@html html}</div>

<style>
  .prose {
    line-height: 1.65;
    overflow-wrap: anywhere;
  }

  .prose :global(.pr-slot.ready > a) {
    display: none;
  }

  .prose :global(.file-slot) {
    margin: 4px 0 10px;
  }

  .prose :global(li:has(> .file-slot)) {
    list-style: none;
  }

  .prose :global(.shot) {
    display: block;
    max-width: 100%;
    margin: 4px 0 10px;
    padding: 0;
    border-radius: 6px;
    box-shadow: 0 0 0 1px var(--border);
    overflow: hidden;
    cursor: zoom-in;
  }

  .prose :global(.shot img) {
    display: block;
    max-width: 100%;
    max-height: 480px;
  }

  .prose :global(.shot.gone) {
    padding: 8px 12px;
    color: var(--text-dim);
    cursor: default;
  }

  .prose :global(.shot.gone img) {
    display: none;
  }

  .prose :global(.shot.gone)::after {
    content: attr(title);
    font: 12px var(--mono);
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
    padding: 6px 6px 6px 12px;
    border-bottom: 1px solid var(--border);
    font: 11.5px var(--mono);
    color: var(--text-faint);
  }

  .prose :global(.copy) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--text);
    color: var(--ink);
    font: 600 12.5px var(--sans);
  }

  .prose :global(.copy svg) {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
  }

  .prose :global(.copy:hover) {
    background: var(--text-strong);
  }

  .prose :global(.views) {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: 4px;
    background: var(--surface-2);
  }

  .prose :global(.views button) {
    height: 24px;
    padding: 0 10px;
    border-radius: 3px;
    font: 500 12px var(--sans);
    color: var(--text-dim);
  }

  .prose :global(.views button[aria-pressed='true']) {
    background: var(--surface-5);
    color: var(--text-strong);
  }

  .prose :global(.rendered) {
    max-height: 520px;
    overflow: auto;
    padding: 14px 16px;
  }

  .prose :global(.mockup) {
    max-height: none;
    padding: 0;
  }

  .prose :global(.mockup iframe) {
    display: block;
    width: 100%;
    height: 80px;
    border: 0;
  }

  .prose :global(.code:has(> .mockup:empty) > .source) {
    display: block;
  }

  .prose :global(.code:has(> .mockup:empty) > .mockup) {
    display: none;
  }

  .prose :global(.rendered > :first-child) {
    margin-top: 0;
  }

  .prose :global(.rendered > :last-child) {
    margin-bottom: 0;
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

  .prose :global(:is(.tok-keyword, .tok-meta)) {
    color: #9ad6cb;
  }

  .prose :global(:is(.tok-string, .tok-string2, .tok-link)) {
    color: #f0a57a;
  }

  .prose :global(.tok-comment) {
    color: #a89c94;
    font-style: italic;
  }

  .prose :global(:is(.tok-number, .tok-bool, .tok-atom)) {
    color: #f7c7aa;
  }

  .prose :global(:is(.tok-typeName, .tok-className, .tok-namespace)) {
    color: #e7c3ae;
  }

  .prose :global(:is(.tok-propertyName, .tok-attributeName)) {
    color: #d8cdc6;
  }

  .prose :global(:is(.tok-heading, .tok-tagName)) {
    color: #9ad6cb;
    font-weight: 600;
  }

  .prose :global(.tok-inserted) {
    color: #9ad6cb;
  }

  .prose :global(:is(.tok-deleted, .tok-invalid)) {
    color: #f59e8c;
  }
</style>
