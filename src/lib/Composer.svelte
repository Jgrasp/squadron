<script lang="ts" module>
  import { markdown } from '@codemirror/lang-markdown'
  import { syntaxHighlighting, syntaxTree } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { Compartment, EditorState, Prec, Transaction, type Range } from '@codemirror/state'
  import { Decoration, EditorView, ViewPlugin, WidgetType, placeholder as hint, type DecorationSet, type ViewUpdate } from '@codemirror/view'
  import { classHighlighter } from '@lezer/highlight'
  import { minimalSetup } from 'codemirror'
  import { KIND_ICONS, kindOf } from './Attachment.svelte'
  import { citedFiles } from './mention.svelte'

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

  const FOLDER = '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M3 10h18" fill="none"/>'

  const span = (className: string, text: string) => Object.assign(document.createElement('span'), { className, textContent: text })

  class FileChip extends WidgetType {
    path: string
    count: number

    constructor(path: string, count: number) {
      super()
      this.path = path
      this.count = count
    }

    eq(other: FileChip) {
      return other.path === this.path && other.count === this.count
    }

    toDOM() {
      const chip = document.createElement('span')
      const path = this.path.replace(/\/$/, '')
      const name = path.split('/').pop()!
      chip.title = this.path
      if (this.path.endsWith('/')) {
        chip.className = 'cm-file-chip folder'
        chip.innerHTML = `<svg viewBox="0 0 24 24" aria-hidden="true">${FOLDER}</svg>`
        chip.append(span('parent', path.slice(0, -name.length)), name, span('count', String(this.count)))
      } else {
        const kind = kindOf(this.path)
        chip.className = `cm-file-chip ${kind}`
        chip.innerHTML = `<i><svg viewBox="0 0 24 24" aria-hidden="true">${KIND_ICONS[kind]}</svg></i>`
        chip.append(name)
      }
      return chip
    }
  }

  function fileChips(list: string[]) {
    const files = new Set(list)
    const counts = new Map<string, number>()
    const count = (dir: string) => counts.get(dir) ?? counts.set(dir, list.filter((f) => f.startsWith(dir) && !f.endsWith('/')).length).get(dir)!
    const build = (view: EditorView, typing = false) => {
      if (!files.size) return Decoration.none
      const doc = view.state.doc
      const head = view.state.selection.main.head
      const marks: Range<Decoration>[] = []
      for (const { from, to } of view.visibleRanges)
        for (let n = doc.lineAt(from).number; n <= doc.lineAt(to).number; n++) {
          const line = doc.line(n)
          for (const cited of citedFiles(line.text, files)) {
            const start = line.from + cited.from
            const end = line.from + cited.to
            if (head > start && (head < end || (typing && head === end))) continue
            marks.push(Decoration.replace({ widget: new FileChip(cited.path, cited.path.endsWith('/') ? count(cited.path) : 0) }).range(start, end))
          }
        }
      return Decoration.set(marks, true)
    }
    const plugin = ViewPlugin.fromClass(
      class {
        decorations: DecorationSet
        constructor(view: EditorView) {
          this.decorations = build(view)
        }
        update(update: ViewUpdate) {
          if (update.docChanged || update.viewportChanged || update.selectionSet) this.decorations = build(update.view, update.transactions.some((tr) => tr.isUserEvent('input.type')))
        }
      },
      { decorations: (plugin) => plugin.decorations },
    )
    return [plugin, EditorView.atomicRanges.of((view) => view.plugin(plugin)?.decorations ?? Decoration.none)]
  }
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
    files = [],
  }: {
    value?: string
    placeholder?: string
    id?: string
    label: string
    autofocus?: boolean
    onkeydown?: (event: KeyboardEvent) => void
    onpaste?: (event: ClipboardEvent) => void
    oncaret?: (caret: number) => void
    files?: string[]
  } = $props()

  let host: HTMLDivElement
  let view = $state<EditorView>()
  const hinting = new Compartment()
  const chips = new Compartment()

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
          chips.of(fileChips(files)),
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
    if (autofocus) requestAnimationFrame(() => view?.focus())
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

  $effect(() => {
    view?.dispatch({ effects: chips.reconfigure(fileChips(files)) })
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
    margin-left: 0;
    border-left-color: var(--text);
  }

  .composer-field :global(.cm-placeholder) {
    color: var(--text-faint);
  }

  .composer-field :global(.cm-file-chip) {
    --kind: var(--text-dim);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    margin: 1px 0;
    padding: 0 7px 0 3px;
    border-radius: 4px;
    vertical-align: -1px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    font: 12.5px var(--mono);
    color: var(--text-2);
    white-space: nowrap;
  }

  .composer-field :global(.cm-file-chip i) {
    display: grid;
    place-items: center;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--kind) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--kind) 35%, transparent);
    color: var(--kind);
  }

  .composer-field :global(.cm-file-chip svg) {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .composer-field :global(.cm-file-chip.folder) {
    gap: 4px;
    padding-left: 6px;
  }

  .composer-field :global(.cm-file-chip.folder svg) {
    width: 14px;
    height: 14px;
    margin-right: 1px;
    color: var(--text-muted);
    fill: currentColor;
    fill-opacity: 0.28;
    stroke-width: 1.8;
  }

  .composer-field :global(.cm-file-chip .parent) {
    color: var(--text-faint);
  }

  .composer-field :global(.cm-file-chip .count) {
    margin-left: 3px;
    color: var(--text-dim);
    font-size: 11px;
  }

  .composer-field :global(.cm-file-chip.sheet) {
    --kind: #4cbf85;
  }

  .composer-field :global(.cm-file-chip.pdf) {
    --kind: #e5655a;
  }

  .composer-field :global(.cm-file-chip.doc) {
    --kind: #6ea8f0;
  }

  .composer-field :global(.cm-file-chip.archive) {
    --kind: #e0b65b;
  }

  .composer-field :global(.cm-file-chip.code) {
    --kind: #b79cf0;
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
