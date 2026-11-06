<script lang="ts" module>
  import { markdown } from '@codemirror/lang-markdown'
  import { syntaxHighlighting, syntaxTree } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { Compartment, EditorState, Prec, Transaction, type Range } from '@codemirror/state'
  import { Decoration, EditorView, ViewPlugin, placeholder as hint, type DecorationSet, type ViewUpdate } from '@codemirror/view'
  import { classHighlighter } from '@lezer/highlight'
  import { minimalSetup } from 'codemirror'

  function fenceLines(view: EditorView) {
    const doc = view.state.doc
    const marks: Range<Decoration>[] = []
    for (const { from, to } of view.visibleRanges)
      syntaxTree(view.state).iterate({
        from,
        to,
        enter: (node) => {
          if (node.name !== 'FencedCode') return
          const first = doc.lineAt(node.from).number
          const last = doc.lineAt(node.to).number
          for (let n = first; n <= last; n++) {
            const edge = n === first ? ' cm-fence-open' : n === last ? ' cm-fence-close' : ''
            marks.push(Decoration.line({ class: `cm-fence${edge}` }).range(doc.line(n).from))
          }
          return false
        },
      })
    return Decoration.set(marks, true)
  }

  const fences = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      constructor(view: EditorView) {
        this.decorations = fenceLines(view)
      }
      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged || syntaxTree(update.startState) !== syntaxTree(update.state)) this.decorations = fenceLines(update.view)
      }
    },
    { decorations: (plugin) => plugin.decorations },
  )
</script>

<script lang="ts">
  import { onMount } from 'svelte'

  let {
    value = $bindable(''),
    placeholder = '',
    id,
    label,
    autofocus = false,
    onkeydown,
    onpaste,
    oncaret,
  }: {
    value?: string
    placeholder?: string
    id?: string
    label: string
    autofocus?: boolean
    onkeydown?: (event: KeyboardEvent) => void
    onpaste?: (event: ClipboardEvent) => void
    oncaret?: (caret: number) => void
  } = $props()

  let host: HTMLDivElement
  let view = $state<EditorView>()
  const hinting = new Compartment()

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          Prec.highest(
            EditorView.domEventHandlers({
              keydown: (event) => (onkeydown?.(event), event.defaultPrevented),
              paste: (event) => (onpaste?.(event), event.defaultPrevented),
            }),
          ),
          minimalSetup,
          markdown({ codeLanguages: languages, addKeymap: false }),
          syntaxHighlighting(classHighlighter),
          fences,
          EditorView.lineWrapping,
          EditorView.theme({}, { dark: true }),
          hinting.of(hint(placeholder)),
          EditorView.contentAttributes.of({ ...(id ? { id } : {}), ...(autofocus ? { autofocus: '' } : {}), 'aria-label': label, 'aria-multiline': 'true', spellcheck: 'true' }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) value = update.state.doc.toString()
            if (update.docChanged || update.selectionSet) oncaret?.(update.state.selection.main.head)
          }),
        ],
      }),
    })
    if (autofocus) view.focus()
    return () => view?.destroy()
  })

  $effect(() => {
    const next = value
    if (!view || next === view.state.doc.toString()) return
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next }, selection: { anchor: next.length }, annotations: Transaction.addToHistory.of(false) })
  })

  $effect(() => {
    view?.dispatch({ effects: hinting.reconfigure(hint(placeholder)) })
  })

  export const focus = () => view?.focus()

  export const content = () => view?.state.doc.toString() ?? value

  export const selection = () => view?.state.selection.main ?? { from: 0, to: 0 }

  export function setSelectionRange(from: number, to: number) {
    if (!view) return
    const end = view.state.doc.length
    view.dispatch({ selection: { anchor: Math.min(from, end), head: Math.min(to, end) }, scrollIntoView: true })
  }

  export function insert(text: string) {
    view?.dispatch(view.state.replaceSelection(text), { scrollIntoView: true, userEvent: 'input.paste' })
  }
</script>

<div class="composer-field" bind:this={host}></div>

<style>
  .composer-field {
    min-width: 0;
  }

  .composer-field :global(.cm-editor) {
    height: 100%;
    background: none;
    color: inherit;
    font: inherit;
  }

  .composer-field :global(.cm-editor.cm-focused) {
    outline: none;
  }

  .composer-field :global(.cm-scroller) {
    font: inherit;
    line-height: 1.5;
  }

  .composer-field :global(.cm-content) {
    padding: 0;
    caret-color: var(--text);
  }

  .composer-field :global(.cm-line) {
    padding: 0;
  }

  .composer-field :global(.cm-cursor) {
    border-left-color: var(--text);
  }

  .composer-field :global(.cm-placeholder) {
    color: var(--text-faint);
  }

  .composer-field :global(.cm-fence) {
    padding: 0 10px;
    background: var(--bg-main);
    font: 12.5px/1.6 var(--mono);
    color: #e8ddd4;
  }

  .composer-field :global(.cm-fence-open) {
    padding-top: 4px;
    border-radius: 5px 5px 0 0;
  }

  .composer-field :global(.cm-fence-close) {
    padding-bottom: 4px;
    border-radius: 0 0 5px 5px;
  }

  .composer-field :global(:is(.cm-fence-open, .cm-fence-close)),
  .composer-field :global(:is(.cm-fence-open, .cm-fence-close) *) {
    color: var(--text-faint);
  }

  .composer-field :global(.cm-fence :is(.tok-keyword, .tok-meta)) {
    color: #9ad6cb;
  }

  .composer-field :global(.cm-fence :is(.tok-string, .tok-string2, .tok-link)) {
    color: #f0a57a;
  }

  .composer-field :global(.cm-fence .tok-comment) {
    color: #a89c94;
    font-style: italic;
  }

  .composer-field :global(.cm-fence :is(.tok-number, .tok-bool, .tok-atom)) {
    color: #f7c7aa;
  }

  .composer-field :global(.cm-fence :is(.tok-typeName, .tok-className, .tok-namespace)) {
    color: #e7c3ae;
  }

  .composer-field :global(.cm-fence :is(.tok-heading, .tok-tagName)) {
    color: #9ad6cb;
  }
</style>
