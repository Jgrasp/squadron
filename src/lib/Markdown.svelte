<script lang="ts" module>
  import { LanguageDescription, type Language } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { classHighlighter, highlightCode } from '@lezer/highlight'
  import MarkdownIt from 'markdown-it'
  import { t } from './i18n/index.svelte'
  import { isImage, preview } from './Attachment.svelte'
  import { isPullLink } from './linked.svelte'
  import { projectFile, within } from './filelink'

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
      const ref = projectFile(href)
      if (alone && ref) {
        tokens[i - 1].tag = tokens[i + 1].tag = 'div'
        tokens[i - 1].hidden = tokens[i + 1].hidden = false
        tokens[i - 1].attrSet('class', 'file-slot')
        tokens[i - 1].attrSet('data-project-file', ref)
        return
      }
      for (const c of token.children) {
        if (c.type !== 'link_open') continue
        const link = String(c.attrGet('href') ?? '')
        if (isPullLink(link)) c.attrSet('data-pr-inline', '')
        else if (projectFile(link) || localFile(link)) c.attrSet('data-project-file-inline', projectFile(link) || localFile(link))
      }
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
  const SHELLS = ['bash', 'sh', 'shell', 'zsh', 'console', 'terminal']
  const runButton = () => `<button type="button" class="run" data-run><svg width="11" height="11" viewBox="0 0 24 24" aria-hidden="true"><path d="M7 4.5v15l12-7.5z"/></svg><span>${t('common.run')}</span></button>`
  md.renderer.rules.fence = (tokens, idx, options, env, self) => {
    const lang = tokens[idx].info.trim().split(/\s+/)[0] || 'text'
    const source = fence(tokens, idx, options, env, self)
    if (lang === 'html' && env?.streaming)
      return `<div class="code"><div class="code-head"><span>html</span></div><div class="drawing"><span>${t('conversation.drawing')}</span></div></div>`
    if (lang === 'html') {
      const tabs = `<span class="views"><button type="button" data-view="preview" aria-pressed="true">${t('conversation.preview')}</button><button type="button" data-view="source" aria-pressed="false">HTML</button></span>`
      return `<div class="code"><div class="code-head">${tabs}${copyButton()}</div><div class="rendered mockup" data-mockup></div><div class="source" hidden>${source}</div></div>`
    }
    if (SHELLS.includes(lang))
      return `<div class="code"><div class="code-head"><span>${md.utils.escapeHtml(lang)}</span><span class="buttons">${copyButton()}${runButton()}</span></div>${source}</div>`
    if (lang !== 'markdown' && lang !== 'md')
      return `<div class="code"><div class="code-head"><span>${md.utils.escapeHtml(lang)}</span>${copyButton()}</div>${source}</div>`
    const tabs = `<span class="views"><button type="button" data-view="preview" aria-pressed="true">${t('conversation.preview')}</button><button type="button" data-view="source" aria-pressed="false">${t('conversation.markdown')}</button></span>`
    return `<div class="code"><div class="code-head">${tabs}${copyButton()}</div><div class="rendered">${md.render(tokens[idx].content)}</div><div class="source" hidden>${source}</div></div>`
  }
</script>

<script lang="ts">
  import { mount, tick, unmount, untrack } from 'svelte'
  import PullCard from './PullCard.svelte'
  import TaskRef from './TaskRef.svelte'
  import FileCard from './FileCard.svelte'
  import TaskDialog from './TaskDialog.svelte'
  import { projects } from './projects.svelte'
  import { codeOf, leading, leads, mentions, taskAt, taskId } from './taskref'
  import Attachment from './Attachment.svelte'
  import { showCapture } from './captures.svelte'

  let {
    text,
    streaming = false,
    cards = false,
    project = '',
    onrun,
  }: { text: string; streaming?: boolean; cards?: boolean; project?: string; onrun?: (command: string) => void } = $props()

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
    return md.render(streaming ? text.slice(0, shown) : text, { streaming })
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

  const projectCards = $derived(cards && project ? (projects.list.find((p) => p.path === project)?.cards ?? []) : [])
  const taskTitles = $derived(projectCards.map((c) => `${c.id} ${c.column} ${c.title}`).join('\n'))

  $effect(() => {
    void html
    void taskTitles
    if (streaming || !cards) return
    const mounted: ReturnType<typeof mount>[] = []
    const added: Element[] = []
    const undo: (() => void)[] = []
    let gone = false
    tick().then(() => {
      if (gone) return
      for (const slot of box?.querySelectorAll<HTMLElement>('[data-file]') ?? []) {
        slot.querySelector('a')?.setAttribute('hidden', '')
        const path = within(slot.dataset.file!, project)
        mounted.push(path ? mount(FileCard, { target: slot, props: { path, project } }) : mount(Attachment, { target: slot, props: { path: slot.dataset.file!, look: 'card', large: true } }))
      }
      const files = new Set<string>()
      for (const slot of box?.querySelectorAll<HTMLElement>('[data-project-file]') ?? []) {
        slot.querySelector('a')?.setAttribute('hidden', '')
        files.add(slot.dataset.projectFile!)
        mounted.push(mount(FileCard, { target: slot, props: { path: slot.dataset.projectFile!, project } }))
      }
      const below = new Map<Element, Element>()
      for (const link of box?.querySelectorAll<HTMLAnchorElement>('a[data-project-file-inline]') ?? []) {
        const path = within(link.dataset.projectFileInline!, project)
        if (!path) continue
        const code = document.createElement('code')
        code.textContent = link.textContent
        link.replaceWith(code)
        undo.unshift(() => code.replaceWith(link))
        if (files.has(path)) continue
        files.add(path)
        let top: Element = code
        while (top.parentElement && top.parentElement !== box) top = top.parentElement
        const slot = document.createElement('div')
        slot.className = 'file-slot'
        ;(below.get(top) ?? top).after(slot)
        below.set(top, slot)
        added.push(slot)
        mounted.push(mount(FileCard, { target: slot, props: { path, project } }))
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
      const tasks = untrack(() => projectCards)
      for (const link of box?.querySelectorAll<HTMLAnchorElement>('a[href^="squadron:task/"]') ?? []) {
        const card = tasks.find((c) => c.id === taskId(link.getAttribute('href')!))
        const item = link.closest('li')
        if (card && item && leads(item, link) && !item.classList.contains('task-item')) {
          const note = document.createElement('div')
          note.className = 'task-note'
          note.append(...item.childNodes)
          const next = document.createTreeWalker(note, NodeFilter.SHOW_TEXT)
          next.currentNode = link
          let after = next.nextNode()
          while (after && link.contains(after)) after = next.nextNode()
          const said = after?.textContent ?? ''
          if (after) after.textContent = said.replace(/^\s*[—–:-]?\s*/, '')
          link.hidden = true
          const slot = document.createElement('span')
          slot.className = 'task-slot'
          item.append(slot, note)
          item.classList.add('task-item')
          mounted.push(mount(TaskRef, { target: slot, props: { id: card.id, project, host: item } }))
          undo.unshift(() => {
            if (after) after.textContent = said
            link.hidden = false
            item.classList.remove('task-item')
            item.replaceChildren(...note.childNodes)
          })
          continue
        }
        const chip = document.createElement(card ? 'button' : 'span')
        chip.textContent = link.textContent
        if (card) {
          Object.assign(chip, { type: 'button', className: `task-chip${chip.textContent?.trim() === codeOf(card.title) ? '' : ' named'}`, title: card.title })
          chip.dataset.task = card.id
          chip.dataset.column = card.column
        }
        link.replaceWith(chip)
        undo.unshift(() => chip.replaceWith(link))
      }
      for (const item of box?.querySelectorAll<HTMLLIElement>('li:not(.task-item)') ?? []) {
        const card = taskAt(tasks, item.textContent ?? '')
        if (!card) continue
        const note = document.createElement('div')
        note.className = 'task-note'
        note.append(...item.childNodes)
        const first = document.createTreeWalker(note, NodeFilter.SHOW_TEXT).nextNode()
        const said = first?.textContent ?? ''
        const code = leading(codeOf(card.title)!)
        if (first && code.test(said)) first.textContent = said.replace(code, '')
        const slot = document.createElement('span')
        slot.className = 'task-slot'
        item.append(slot, note)
        item.classList.add('task-item')
        mounted.push(mount(TaskRef, { target: slot, props: { id: card.id, project, host: item } }))
        undo.unshift(() => {
          if (first) first.textContent = said
          item.classList.remove('task-item')
          item.replaceChildren(...note.childNodes)
        })
      }
      const walker = document.createTreeWalker(box!, NodeFilter.SHOW_TEXT, {
        acceptNode: (node) => (node.parentElement?.closest('pre, code, a, button, .task-slot, .code') ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
      })
      const texts: Text[] = []
      while (walker.nextNode()) texts.push(walker.currentNode as Text)
      for (const text of texts) {
        const parts = mentions(tasks, text.data)
        if (!parts.some((part) => typeof part !== 'string')) continue
        const nodes = parts.map((part) => {
          if (typeof part === 'string') return document.createTextNode(part)
          const chip = document.createElement('button')
          Object.assign(chip, { type: 'button', className: 'task-chip', title: part.card.title, textContent: part.code })
          chip.dataset.task = part.card.id
          chip.dataset.column = part.card.column
          return chip
        })
        text.replaceWith(...nodes)
        undo.unshift(() => {
          nodes[0].replaceWith(text)
          nodes.slice(1).forEach((n) => n.remove())
        })
      }
    })
    return () => {
      gone = true
      mounted.forEach((m) => unmount(m))
      added.forEach((el) => el.remove())
      undo.forEach((f) => f())
    }
  })

  function command(code: string) {
    const lines = code.trim().split('\n')
    return (lines.every((l) => l.startsWith('$ ')) ? lines.map((l) => l.slice(2)) : lines).join('\n')
  }

  function click(event: MouseEvent) {
    const target = event.target as HTMLElement
    const task = target.closest<HTMLElement>('[data-task]')
    if (task) {
      const dialog = mount(TaskDialog, { target: document.body, props: { id: task.dataset.task!, onclose: () => unmount(dialog) } })
      return
    }
    const shot = target.closest<HTMLElement>('[data-shot]')
    if (shot) return void (shot.classList.contains('gone') || showCapture(shot, shot.dataset.shot!))
    const block = target.closest('.code')
    const view = target.closest<HTMLButtonElement>('[data-view]')
    if (block && view) {
      for (const tab of block.querySelectorAll(':scope > .code-head [data-view]')) tab.setAttribute('aria-pressed', String(tab === view))
      block.querySelector(':scope > .rendered')?.toggleAttribute('hidden', view.dataset.view !== 'preview')
      block.querySelector(':scope > .source')?.toggleAttribute('hidden', view.dataset.view !== 'source')
      return
    }
    const code = block?.querySelector(':scope > pre, :scope > .source > pre')?.textContent
    if (target.closest('[data-run]') && code && onrun) return onrun(command(code))
    const button = target.closest<HTMLButtonElement>('[data-copy]')
    if (!button || code === undefined || code === null) return
    navigator.clipboard.writeText(code.replace(/\n$/, ''))
    const label = button.querySelector('span')!
    label.textContent = t('common.copied')
    setTimeout(() => (label.textContent = t('common.copy')), 1500)
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="prose" class:streaming class:runnable={!!onrun && !streaming} onclick={click} bind:this={box}>{@html html}</div>

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

  .prose :global(:is(ul, ol):not(:has(> li:not(.task-item)))) {
    padding-left: 0;
  }

  .prose :global(li.task-item) {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 12px;
    margin: 8px 0;
    padding: 10px 14px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--border);
    list-style: none;
    cursor: pointer;
  }

  .prose :global(li.task-item:hover) {
    background: var(--surface-2);
  }

  .prose :global(.task-slot) {
    display: contents;
  }

  .prose :global(a[href^='squadron:']) {
    pointer-events: none;
    color: inherit;
    text-decoration: none;
  }

  .prose :global(.task-chip) {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px;
    border-radius: 4px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border);
    font: inherit;
    font-family: var(--mono);
    font-size: 0.9em;
    color: var(--text-strong);
    cursor: pointer;
  }

  .prose :global(.task-chip.named) {
    font-family: inherit;
    font-size: inherit;
  }

  .prose :global(.task-chip::before) {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 3px;
    background: var(--text-faint);
  }

  .prose :global(.task-chip[data-column='progress']::before) {
    background: var(--ok);
  }

  .prose :global(.task-chip[data-column='review']::before) {
    background: var(--accent);
  }

  .prose :global(.task-chip[data-column='done']::before) {
    background: var(--done);
  }

  .prose :global(.task-chip:hover) {
    box-shadow: inset 0 0 0 1px var(--text-faint);
  }

  .prose :global(.task-note) {
    grid-column: 2;
    font-size: 13px;
  }

  .prose :global(.task-slot:has(.wide) + .task-note) {
    grid-column: 1 / -1;
  }

  .prose :global(.task-note:empty) {
    display: none;
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

  .prose :global(.buttons) {
    display: flex;
    gap: 6px;
  }

  .prose :global(.run) {
    display: none;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-ink);
    font: 600 12.5px var(--sans);
  }

  .prose.runnable :global(.run) {
    display: inline-flex;
  }

  .prose :global(.run svg) {
    fill: currentColor;
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

  .prose :global(.drawing) {
    display: grid;
    place-items: center;
    height: 120px;
    font-size: 12.5px;
    color: var(--text-faint);
  }

  .prose :global(.drawing span) {
    animation: pulse 1.6s ease-in-out infinite;
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
