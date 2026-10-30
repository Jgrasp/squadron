<script lang="ts" module>
  import { HighlightStyle, LanguageDescription, syntaxHighlighting } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import {
    SearchQuery,
    closeSearchPanel,
    findNext,
    findPrevious,
    getSearchQuery,
    openSearchPanel,
    replaceAll,
    replaceNext,
    search,
    setSearchQuery,
  } from '@codemirror/search'
  import { Compartment, EditorState, Prec, StateEffect, StateField, type Extension } from '@codemirror/state'
  import { EditorView, GutterMarker, ViewPlugin, gutter, keymap, runScopeHandlers, type Panel, type ViewUpdate } from '@codemirror/view'
  import { tags as t } from '@lezer/highlight'
  import { basicSetup } from 'codemirror'
  import { lineMarks } from './diff'
  import { t as say } from './i18n/index.svelte'

  export type Goto = { line: number; query?: { search: string; caseSensitive: boolean; wholeWord: boolean; regexp: boolean } }

  type Bridge = { change: (key: string, doc: string) => void; save: (key: string, doc: string) => void; cursor: (line: number, col: number) => void }

  const states = new Map<string, EditorState>()
  const language = new Compartment()
  const tracking = new Compartment()
  const phrases = new Compartment()
  let bridge: Bridge | null = null

  export function forget(key: string) {
    states.delete(key)
  }

  export function languageOf(name: string) {
    return LanguageDescription.matchFilename(languages, name)
  }

  const theme = EditorView.theme(
    {
      '&': { height: '100%', backgroundColor: 'var(--bg-app)', color: '#e3d7cf', fontSize: '12.5px' },
      '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '21px' },
      '.cm-content': { padding: '8px 0', caretColor: '#ede6e1' },
      '.cm-gutters': { backgroundColor: 'var(--bg-app)', color: '#918579', border: 'none' },
      '.cm-lineNumbers .cm-gutterElement': { padding: '0 10px 0 14px' },
      '.cm-activeLine': { backgroundColor: 'rgba(237, 230, 225, 0.05)' },
      '.cm-activeLineGutter': { backgroundColor: 'transparent', color: '#ede6e1' },
      '&.cm-focused .cm-cursor': { borderLeftColor: '#ede6e1', borderLeftWidth: '2px' },
      '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
        backgroundColor: 'rgba(224, 138, 91, 0.28)',
      },
      '&.cm-focused': { outline: 'none' },
      '.cm-matchingBracket': { backgroundColor: 'rgba(154, 214, 203, 0.18)', outline: 'none' },
      '.cm-searchMatch': { backgroundColor: 'rgba(240, 165, 122, 0.25)' },
      '.cm-searchMatch-selected': { backgroundColor: 'rgba(240, 165, 122, 0.45)' },
      '.cm-foldPlaceholder': { backgroundColor: 'var(--surface-3)', border: 'none', color: 'var(--text-muted)' },
      '.cm-panels': { backgroundColor: 'var(--bg-sidebar)', color: 'var(--text)', borderColor: 'var(--border)' },
      '.cm-panels.cm-panels-top': { position: 'absolute', top: '8px', right: '18px', left: 'auto', zIndex: '5', border: 'none', background: 'transparent' },
      '.cm-find': {
        width: '380px',
        padding: '6px',
        display: 'flex',
        flexDirection: 'column',
        gap: '6px',
        borderRadius: '7px',
        border: '1px solid var(--border-strong)',
        background: 'var(--menu)',
        boxShadow: '0 10px 30px rgba(0, 0, 0, 0.45)',
        fontFamily: 'var(--sans)',
      },
      '.cm-find-row': { display: 'flex', alignItems: 'center', gap: '4px' },
      '.cm-find-replace': { display: 'none' },
      '.cm-find.replacing .cm-find-replace': { display: 'flex' },
      '.cm-find-field': {
        flex: '1',
        minWidth: '0',
        height: '28px',
        display: 'flex',
        alignItems: 'center',
        gap: '2px',
        padding: '0 3px 0 8px',
        border: '1px solid var(--border-strong)',
        borderRadius: '5px',
        background: 'var(--bg-app)',
      },
      '.cm-find-field:focus-within': { borderColor: 'var(--border-focus)' },
      '.cm-find-input': { flex: '1', minWidth: '0', border: 'none', outline: 'none', background: 'transparent', color: 'var(--text-strong)', font: '12.5px var(--mono)' },
      '.cm-find-input::placeholder': { color: 'var(--text-faint)', fontFamily: 'var(--sans)' },
      '.cm-find-count': { flex: 'none', padding: '0 4px', color: 'var(--text-dim)', fontSize: '11.5px', whiteSpace: 'nowrap' },
      '.cm-find-count[data-none]': { color: 'var(--err)' },
      '.cm-find-toggle': { width: '24px', height: '22px', flex: 'none', borderRadius: '4px', color: 'var(--text-dim)', font: '600 11px var(--mono)' },
      '.cm-find-toggle:hover': { background: 'var(--surface-4)', color: 'var(--text)' },
      '.cm-find-toggle[aria-pressed=true]': { background: 'color-mix(in srgb, var(--accent) 22%, transparent)', color: 'var(--accent-soft)' },
      '.cm-find-button': { width: '26px', height: '26px', flex: 'none', borderRadius: '5px', color: 'var(--text-muted)', fontSize: '13px' },
      '.cm-find-button:hover': { background: 'var(--surface-4)', color: 'var(--text-strong)' },
      '.cm-find-more': { width: '18px', color: 'var(--text-dim)', fontSize: '10px' },
      '.cm-find-text': { height: '28px', padding: '0 10px', flex: 'none', borderRadius: '5px', border: '1px solid var(--border-strong)', background: 'var(--surface-2)', color: 'var(--text-2)', fontSize: '12px' },
      '.cm-find-text:hover': { background: 'var(--surface-4)', color: 'var(--text-strong)' },
      '.cm-panels input, .cm-panels button': { fontFamily: 'var(--sans)' },
      '.cm-tooltip': { backgroundColor: 'var(--menu)', border: '1px solid var(--border-strong)', color: 'var(--text)' },
      '.cm-tooltip-autocomplete > ul > li[aria-selected]': { backgroundColor: 'var(--menu-hover)', color: 'var(--text-strong)' },
      '.cm-changes': { width: '4px', marginRight: '8px' },
      '.cm-changes .cm-gutterElement': { padding: '0' },
      '.cm-mark-added': { height: '100%', background: '#9ad6cb' },
      '.cm-mark-removed': { height: '3px', background: '#f59e8c' },
    },
    { dark: true },
  )

  const style = HighlightStyle.define([
    { tag: [t.keyword, t.modifier, t.operatorKeyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.self], color: '#9ad6cb' },
    { tag: [t.string, t.special(t.string), t.regexp, t.character], color: '#f0a57a' },
    { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: '#a89c94', fontStyle: 'italic' },
    { tag: [t.number, t.bool, t.null, t.atom], color: '#f7c7aa' },
    { tag: [t.typeName, t.className, t.namespace], color: '#e7c3ae' },
    { tag: [t.function(t.variableName), t.function(t.propertyName)], color: '#ede6e1' },
    { tag: [t.tagName, t.heading], color: '#9ad6cb', fontWeight: '600' },
    { tag: [t.attributeName, t.propertyName], color: '#d8cdc6' },
    { tag: t.link, color: '#f0a57a', textDecoration: 'underline' },
    { tag: t.invalid, color: '#f59e8c' },
  ])

  const setMarks = StateEffect.define<ReturnType<typeof lineMarks>>()
  const marks = StateField.define({
    create: () => ({ added: new Set<number>(), removed: new Set<number>() }),
    update: (value, tr) => tr.effects.find((e) => e.is(setMarks))?.value ?? value,
  })

  class Mark extends GutterMarker {
    kind: string
    constructor(kind: string) {
      super()
      this.kind = kind
    }
    toDOM() {
      const el = document.createElement('div')
      el.className = `cm-mark-${this.kind}`
      return el
    }
  }
  const added = new Mark('added')
  const removed = new Mark('removed')

  function changes(base: string | null): Extension {
    return [
      marks,
      ViewPlugin.fromClass(
        class {
          timer = 0
          constructor(view: EditorView) {
            this.run(view, 0)
          }
          update(u: ViewUpdate) {
            if (u.docChanged) this.run(u.view, 300)
          }
          run(view: EditorView, delay: number) {
            clearTimeout(this.timer)
            this.timer = window.setTimeout(() => view.dispatch({ effects: setMarks.of(lineMarks(base ?? '', view.state.doc.toString())) }), delay)
          }
          destroy() {
            clearTimeout(this.timer)
          }
        },
      ),
      gutter({
        class: 'cm-changes',
        lineMarker(view, line) {
          const doc = view.state.doc
          const n = doc.lineAt(line.from).number
          const found = view.state.field(marks)
          if (found.added.has(n)) return added
          return found.removed.has(n) || (n === doc.lines && found.removed.has(n + 1)) ? removed : null
        },
        lineMarkerChange: (u) => u.transactions.some((tr) => tr.effects.some((e) => e.is(setMarks))),
        initialSpacer: () => added,
      }),
    ]
  }

  function words() {
    return EditorState.phrases.of({
      Find: say('shell.cmFind'),
      Replace: say('shell.cmReplace'),
      next: say('shell.cmNext'),
      previous: say('shell.cmPrevious'),
      all: say('shell.cmAll'),
      'match case': say('shell.cmMatchCase'),
      regexp: say('shell.cmRegexp'),
      'by word': say('shell.cmByWord'),
      replace: say('shell.cmReplaceOne'),
      'replace all': say('shell.cmReplaceAll'),
      close: say('shell.cmClose'),
      'current match': say('shell.cmCurrentMatch'),
      'replaced $ matches': say('shell.cmReplacedMatches'),
      'replaced match on line $': say('shell.cmReplacedOnLine'),
      'on line': say('shell.cmOnLine'),
      'Go to line': say('shell.cmGoToLine'),
      go: say('shell.cmGo'),
      'Folded lines': say('shell.cmFoldedLines'),
      'Unfolded lines': say('shell.cmUnfoldedLines'),
      'folded code': say('shell.cmFoldedCode'),
      unfold: say('shell.cmUnfold'),
      'Fold line': say('shell.cmFoldLine'),
      'Unfold line': say('shell.cmUnfoldLine'),
      to: say('shell.cmTo'),
      Completions: say('shell.cmCompletions'),
      Diagnostics: say('shell.cmDiagnostics'),
      'No diagnostics': say('shell.cmNoDiagnostics'),
    })
  }

  function make<K extends keyof HTMLElementTagNameMap>(tag: K, props: Partial<HTMLElementTagNameMap[K]>, ...children: (Node | string)[]) {
    const node = Object.assign(document.createElement(tag), props)
    node.append(...children)
    return node
  }

  function findPanel(view: EditorView): Panel {
    let query = getSearchQuery(view.state)
    const input = make('input', { className: 'cm-find-input', placeholder: say('shell.cmFind'), value: query.search, spellcheck: false })
    input.setAttribute('main-field', 'true')
    const swap = make('input', { className: 'cm-find-input', placeholder: say('shell.cmReplace'), value: query.replace, spellcheck: false })
    const count = make('span', { className: 'cm-find-count' })
    const toggle = (text: string, title: string) => {
      const button = make('button', { type: 'button', className: 'cm-find-toggle', textContent: text, title, onclick: () => (button.ariaPressed = String(button.ariaPressed !== 'true'), commit()) })
      return button
    }
    const sensitive = toggle('Aa', say('shell.cmMatchCase'))
    const whole = toggle('ab', say('shell.cmByWord'))
    const regexp = toggle('.*', say('shell.cmRegexp'))
    const button = (text: string, title: string, run: () => unknown, className = 'cm-find-button') => make('button', { type: 'button', className, textContent: text, title, onclick: run })
    const more = button('▸', say('shell.findToggleReplace'), () => {
      const open = dom.classList.toggle('replacing')
      more.textContent = open ? '▾' : '▸'
      if (open) swap.focus()
    }, 'cm-find-button cm-find-more')
    const dom = make(
      'div',
      { className: 'cm-find' },
      make('div', { className: 'cm-find-row' }, more, make('div', { className: 'cm-find-field' }, input, count, sensitive, whole, regexp), button('↑', say('shell.cmPrevious'), () => findPrevious(view)), button('↓', say('shell.cmNext'), () => findNext(view)), button('×', say('shell.cmClose'), () => closeSearchPanel(view))),
      make('div', { className: 'cm-find-row cm-find-replace' }, make('span', { className: 'cm-find-more' }), make('div', { className: 'cm-find-field' }, swap), button(say('shell.findReplaceOne'), '', () => replaceNext(view), 'cm-find-text'), button(say('shell.findReplaceAll'), '', () => replaceAll(view), 'cm-find-text')),
    )

    function show() {
      input.value = query.search
      swap.value = query.replace
      sensitive.ariaPressed = String(query.caseSensitive)
      whole.ariaPressed = String(query.wholeWord)
      regexp.ariaPressed = String(query.regexp)
    }

    function commit() {
      const next = new SearchQuery({ search: input.value, replace: swap.value, caseSensitive: sensitive.ariaPressed === 'true', wholeWord: whole.ariaPressed === 'true', regexp: regexp.ariaPressed === 'true' })
      if (next.eq(query)) return
      query = next
      view.dispatch({ effects: setSearchQuery.of(next) })
    }

    function counted() {
      count.removeAttribute('data-none')
      if (!query.search || !query.valid) return (count.textContent = '')
      const { from, to } = view.state.selection.main
      const cursor = query.getCursor(view.state)
      let total = 0
      let at = 0
      for (let found = cursor.next(); !found.done && total < 1000; found = cursor.next()) {
        total++
        if (found.value.from === from && found.value.to === to) at = total
      }
      if (!total) count.setAttribute('data-none', '')
      count.textContent = !total ? say('shell.findNone') : at ? say('shell.findCount', { at: String(at), count: String(total) }) : String(total)
    }

    input.oninput = swap.oninput = commit
    dom.onkeydown = (event) => {
      if (runScopeHandlers(view, event, 'search-panel')) return event.preventDefault()
      if (event.key !== 'Enter') return
      event.preventDefault()
      if (event.target === swap) replaceNext(view)
      else (event.shiftKey ? findPrevious : findNext)(view)
    }
    show()

    return {
      dom,
      top: true,
      mount: counted,
      update(u) {
        let changed = false
        for (const tr of u.transactions)
          for (const effect of tr.effects)
            if (effect.is(setSearchQuery)) {
              changed = true
              if (!effect.value.eq(query)) (query = effect.value), show()
            }
        if (changed || u.docChanged || u.selectionSet) counted()
      },
    }
  }

  function jump(view: EditorView, { line, query }: Goto) {
    const at = view.state.doc.line(Math.min(Math.max(line, 1), view.state.doc.lines))
    view.dispatch({ selection: { anchor: at.from } })
    let range = { from: at.from, to: at.from }
    const effects: StateEffect<unknown>[] = [EditorView.scrollIntoView(at.from, { y: 'center' })]
    if (query) {
      const next = new SearchQuery(query)
      const found = next.valid ? next.getCursor(view.state, at.from, at.to).next() : null
      if (found && !found.done) range = found.value
      openSearchPanel(view)
      effects.push(setSearchQuery.of(next))
    }
    view.dispatch({ selection: { anchor: range.from, head: range.to }, effects })
  }

  function create(key: string, text: string, base: string | null | undefined) {
    return EditorState.create({
      doc: text,
      extensions: [
        Prec.highest(keymap.of([{ key: 'Mod-s', run: (view) => (bridge?.save(key, view.state.doc.toString()), true) }])),
        syntaxHighlighting(style),
        search({ top: true, createPanel: findPanel }),
        basicSetup,
        theme,
        language.of([]),
        phrases.of(words()),
        tracking.of(base === undefined ? [] : changes(base)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) bridge?.change(key, u.state.doc.toString())
          if (u.docChanged || u.selectionSet) {
            const head = u.state.selection.main.head
            const line = u.state.doc.lineAt(head)
            bridge?.cursor(line.number, head - line.from + 1)
          }
        }),
      ],
    })
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte'

  let {
    key,
    name,
    text,
    base,
    onchange,
    onsave,
    oncursor,
    goto = null,
    ongoto = () => {},
  }: {
    key: string
    name: string
    text: string
    base: string | null | undefined
    onchange: (key: string, doc: string) => void
    onsave: (key: string, doc: string) => void
    oncursor: (line: number, col: number) => void
    goto?: Goto | null
    ongoto?: () => void
  } = $props()

  let host: HTMLDivElement
  let view: EditorView | null = null
  let shown = ''

  export function replace(next: string) {
    if (!view) return
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } })
  }

  $effect(() => {
    bridge = { change: (k, doc) => onchange(k, doc), save: (k, doc) => onsave(k, doc), cursor: (l, c) => oncursor(l, c) }
    return () => {
      if (view) states.set(shown, view.state)
      view?.destroy()
      view = null
      bridge = null
    }
  })

  $effect(() => {
    const next = key
    untrack(() => {
      const state = states.get(next) ?? create(next, text, base)
      if (!view) view = new EditorView({ state, parent: host })
      else if (shown !== next) {
        states.set(shown, view.state)
        view.setState(state)
      }
      shown = next
      states.set(next, state)
      const head = state.selection.main.head
      const line = state.doc.lineAt(head)
      oncursor(line.number, head - line.from + 1)
      languageOf(name)
        ?.load()
        .then((support) => {
          if (shown === next) view?.dispatch({ effects: language.reconfigure(support) })
        })
      view.focus()
    })
  })

  $effect(() => {
    const target = goto
    void key
    if (!target) return
    untrack(() => {
      if (view) jump(view, target)
      ongoto()
    })
  })

  $effect(() => {
    const value = base
    untrack(() => view?.dispatch({ effects: tracking.reconfigure(value === undefined ? [] : changes(value)) }))
  })

  $effect(() => {
    const next = words()
    untrack(() => view?.dispatch({ effects: phrases.reconfigure(next) }))
  })
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .editor :global(.cm-editor) {
    height: 100%;
  }
</style>
