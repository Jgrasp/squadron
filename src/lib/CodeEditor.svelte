<script lang="ts" module>
  import { HighlightStyle, LanguageDescription, syntaxHighlighting } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { Compartment, EditorState, Prec, StateEffect, StateField, type Extension } from '@codemirror/state'
  import { EditorView, GutterMarker, ViewPlugin, gutter, keymap, type ViewUpdate } from '@codemirror/view'
  import { tags as t } from '@lezer/highlight'
  import { basicSetup } from 'codemirror'
  import { lineMarks } from './diff'

  type Bridge = { change: (key: string, doc: string) => void; save: (key: string, doc: string) => void; cursor: (line: number, col: number) => void }

  const states = new Map<string, EditorState>()
  const language = new Compartment()
  const tracking = new Compartment()
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

  function create(key: string, text: string, base: string | null | undefined) {
    return EditorState.create({
      doc: text,
      extensions: [
        Prec.highest(keymap.of([{ key: 'Mod-s', run: (view) => (bridge?.save(key, view.state.doc.toString()), true) }])),
        syntaxHighlighting(style),
        basicSetup,
        theme,
        language.of([]),
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
  }: {
    key: string
    name: string
    text: string
    base: string | null | undefined
    onchange: (key: string, doc: string) => void
    onsave: (key: string, doc: string) => void
    oncursor: (line: number, col: number) => void
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
    const value = base
    untrack(() => view?.dispatch({ effects: tracking.reconfigure(value === undefined ? [] : changes(value)) }))
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
