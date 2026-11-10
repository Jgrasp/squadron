<script lang="ts" module>
  import { markdown } from '@codemirror/lang-markdown'
  import { syntaxHighlighting, syntaxTree } from '@codemirror/language'
  import { languages } from '@codemirror/language-data'
  import { Compartment, EditorState, Prec, Transaction, type Range } from '@codemirror/state'
  import { Decoration, EditorView, ViewPlugin, WidgetType, placeholder as hint, type DecorationSet, type ViewUpdate } from '@codemirror/view'
  import { classHighlighter } from '@lezer/highlight'
  import { minimalSetup } from 'codemirror'
  import { mount, unmount } from 'svelte'
  import FileChip from './FileChip.svelte'
  import LinkChip from './LinkChip.svelte'
  import { citedFiles, filesIn } from './mention.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { urlsIn } from './urls'

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

  const chips = new WeakMap<HTMLElement, ReturnType<typeof mount>>()

  class FileWidget extends WidgetType {
    path: string
    count: number

    constructor(path: string, count: number) {
      super()
      this.path = path
      this.count = count
    }

    eq(other: FileWidget) {
      return other.path === this.path && other.count === this.count
    }

    toDOM() {
      const host = document.createElement('span')
      chips.set(host, mount(FileChip, { target: host, props: { path: this.path, count: this.count } }))
      return host
    }

    destroy(dom: HTMLElement) {
      const chip = chips.get(dom)
      if (chip) unmount(chip)
    }
  }

  class LinkWidget extends WidgetType {
    url: string

    constructor(url: string) {
      super()
      this.url = url
    }

    eq(other: LinkWidget) {
      return other.url === this.url
    }

    toDOM() {
      const host = document.createElement('span')
      host.addEventListener('mousedown', (event) => {
        if (!(event.ctrlKey || event.metaKey) || event.button !== 0) return
        event.preventDefault()
        invoke('open_link', { url: this.url })
      })
      chips.set(host, mount(LinkChip, { target: host, props: { url: this.url } }))
      return host
    }

    destroy(dom: HTMLElement) {
      const chip = chips.get(dom)
      if (chip) unmount(chip)
    }
  }

  function linkMarks(view: EditorView, typing = false) {
    const doc = view.state.doc
    const tree = syntaxTree(view.state)
    const head = view.state.selection.main.head
    const marks: Range<Decoration>[] = []
    for (const { from, to } of view.visibleRanges)
      for (let n = doc.lineAt(from).number; n <= doc.lineAt(to).number; n++) {
        const line = doc.line(n)
        for (const found of urlsIn(line.text)) {
          const start = line.from + found.from
          const end = line.from + found.to
          if (head > start && (head < end || (typing && head === end))) continue
          if (/^(FencedCode|CodeText)$/.test(tree.resolveInner(start, 1).name)) continue
          marks.push(Decoration.replace({ widget: new LinkWidget(found.url) }).range(start, end))
        }
      }
    return Decoration.set(marks, true)
  }

  const linkChips = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      constructor(view: EditorView) {
        this.decorations = linkMarks(view)
      }
      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged || update.selectionSet || syntaxTree(update.startState) !== syntaxTree(update.state))
          this.decorations = linkMarks(update.view, update.transactions.some((tr) => tr.isUserEvent('input.type')))
      }
    },
    { decorations: (plugin) => plugin.decorations },
  )

  function fileChips(list: string[]) {
    const files = new Set(list)
    const counts = new Map<string, number>()
    const count = (dir: string) => counts.get(dir) ?? counts.set(dir, filesIn(list, dir)).get(dir)!
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
            marks.push(Decoration.replace({ widget: new FileWidget(cited.path, cited.path.endsWith('/') ? count(cited.path) : 0) }).range(start, end))
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
  const chipping = new Compartment()

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
          linkChips,
          EditorView.atomicRanges.of((view) => view.plugin(linkChips)?.decorations ?? Decoration.none),
          chipping.of(fileChips(files)),
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
    if (!view) return
    view.dispatch({ effects: hinting.reconfigure(hint(placeholder)) })
    const scroller = view.scrollDOM
    // WebKitGTK keeps a stale field height after the placeholder changes until a size change forces a new layout
    requestAnimationFrame(() => ((scroller.style.minHeight = '0'), scroller.offsetHeight, (scroller.style.minHeight = '')))
  })

  $effect(() => {
    view?.dispatch({ effects: chipping.reconfigure(fileChips(files)) })
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
