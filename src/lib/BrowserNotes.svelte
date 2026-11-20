<script lang="ts" module>
  export type Picked = { ref: string; selector: string; role: string; name: string; html: string; rect: [number, number, number, number] }
  export type Note = { path: [number, number][]; picked?: Picked | null; comment: string }
</script>

<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import type { BrowserTab } from './browser.svelte'
  import { t } from './i18n/index.svelte'
  import { base64 } from './paste'
  import type { Comment } from './stream'

  let {
    tab,
    canvas,
    notes = $bindable(),
    onsend,
    onclose,
    onfail,
  }: {
    tab: BrowserTab
    canvas: HTMLCanvasElement
    notes: Note[]
    onsend: (comments: Comment[], image: string) => void
    onclose: () => void
    onfail: (e: unknown) => void
  } = $props()

  let drawing = $state<[number, number][] | null>(null)
  let fields = $state<HTMLTextAreaElement[]>([])
  let sending = $state(false)
  let folded = $state(false)
  let hovered = $state<Picked | null>(null)
  let aiming = false
  let shift = $state(0)
  let edge = 0
  const COLUMN = 250

  $effect(() => {
    canvas.style.translate = shift ? `${-shift}px 0` : ''
    return () => (canvas.style.translate = '')
  })

  function approach(x: number) {
    const box = canvas.parentElement!.getBoundingClientRect()
    const want = x - box.left < 24 ? 0 : x - box.left > box.width - COLUMN - 24 ? COLUMN : shift
    clearTimeout(edge)
    if (want !== shift) edge = window.setTimeout(() => (shift = want), 300)
  }

  function fold() {
    folded = true
    shift = 0
  }

  function at(e: PointerEvent): [number, number] {
    const box = (e.currentTarget as Element).getBoundingClientRect()
    return [Math.round(e.clientX - box.left), Math.round(e.clientY - box.top)]
  }

  function down(e: PointerEvent) {
    if (e.button !== 0) return
    ;(e.currentTarget as Element).setPointerCapture(e.pointerId)
    drawing = [at(e)]
  }

  function move(e: PointerEvent) {
    if (drawing) return void drawing.push(at(e))
    if (!folded) approach(e.clientX)
    if (aiming) return
    aiming = true
    const [x, y] = at(e)
    invoke<Picked | null>('browser_pick', { tab: tab.id, x, y, w: 0, h: 0 })
      .then((p) => (hovered = p), () => (hovered = null))
      .finally(() => (aiming = false))
  }

  async function up() {
    if (!drawing) return
    const path = $state.snapshot(drawing)
    drawing = null
    const xs = path.map((p) => p[0])
    const ys = path.map((p) => p[1])
    const [x, y] = [Math.min(...xs), Math.min(...ys)]
    notes.push({ path, comment: '' })
    folded = false
    const note = notes[notes.length - 1]
    note.picked = await invoke<Picked | null>('browser_pick', { tab: tab.id, x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y }).catch((e) => (onfail(e), null))
    fields[notes.indexOf(note)]?.focus()
  }

  const points = (path: [number, number][]) => path.map((p) => p.join(',')).join(' ')
  const pin = (note: Note): [number, number] => (note.path.length <= 2 && note.picked ? [note.picked.rect[0], note.picked.rect[1]] : note.path[0])
  const what = (p?: Picked | null) => (p ? `${p.role}${p.name ? ` “${p.name}”` : ''}` : p === null ? t('browser.spot') : '…')

  async function capture() {
    const scale = canvas.width / canvas.clientWidth
    const shot = document.createElement('canvas')
    shot.width = canvas.width
    shot.height = canvas.height
    const g = shot.getContext('2d')!
    g.drawImage(canvas, 0, 0)
    g.scale(scale, scale)
    const style = getComputedStyle(canvas)
    const accent = style.getPropertyValue('--accent').trim()
    const ink = style.getPropertyValue('--accent-ink').trim()
    g.lineWidth = 3
    g.lineJoin = g.lineCap = 'round'
    g.strokeStyle = accent
    notes.forEach((note, i) => {
      if (note.path.length > 2) {
        g.beginPath()
        note.path.forEach(([x, y], j) => (j ? g.lineTo(x, y) : g.moveTo(x, y)))
        g.stroke()
      } else if (note.picked) {
        g.strokeRect(...note.picked.rect)
      }
      const [x, y] = pin(note)
      g.fillStyle = accent
      g.beginPath()
      g.arc(x, y, 11, 0, Math.PI * 2)
      g.fill()
      g.fillStyle = ink
      g.font = 'bold 12px sans-serif'
      g.textAlign = 'center'
      g.textBaseline = 'middle'
      g.fillText(String(i + 1), x, y + 0.5)
    })
    const blob = await new Promise<Blob | null>((done) => shot.toBlob(done, 'image/png'))
    if (!blob) return ''
    const name = `annotations-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.png`
    return invoke<string>('save_attachment', { name, data: await base64(blob) })
  }

  async function send() {
    if (!notes.length || sending) return
    sending = true
    try {
      const image = await capture()
      const page = { url: tab.url, title: tab.title, tab: tab.id, shot: image || undefined }
      const comments = notes.map((note, i) => ({
        path: '',
        line: 0,
        removed: false,
        code: '',
        note: note.comment.trim(),
        page: { ...page, n: i + 1, what: note.picked ? what(note.picked) : '', selector: note.picked?.selector ?? '', ref: note.picked?.ref ?? '', html: note.picked?.html ?? '' },
      }))
      onsend(comments, image)
      notes = []
    } catch (e) {
      onfail(e)
    }
    sending = false
  }
</script>

<svg
  class="ink"
  role="application"
  aria-label={t('browser.notesArea')}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={() => (drawing = null)}
  onpointerleave={() => (clearTimeout(edge), (hovered = null))}
  style:translate={shift ? `${-shift}px 0` : null}
>
  {#if hovered && !(drawing && drawing.length > 1)}
    <rect class="hover" x={hovered.rect[0]} y={hovered.rect[1]} width={hovered.rect[2]} height={hovered.rect[3]} />
  {/if}
  {#each notes as note, i (note)}
    {#if note.path.length > 2}
      <polyline points={points(note.path)} />
    {:else if note.picked}
      <rect class="box" x={note.picked.rect[0]} y={note.picked.rect[1]} width={note.picked.rect[2]} height={note.picked.rect[3]} />
    {/if}
    <g class="badge" transform={`translate(${pin(note).join(',')})`}>
      <circle r="11" />
      <text dy="0.35em">{i + 1}</text>
    </g>
  {/each}
  {#if drawing && drawing.length > 1}
    <polyline points={points(drawing)} />
  {/if}
</svg>

{#if folded}
  <div class="tab">
    {#if notes.length}<span class="count">{notes.length}</span>{/if}
    <button class="icon" aria-label={t('browser.openNotes')} title={t('browser.openNotes')} onclick={() => (folded = false)}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
    </button>
  </div>
{:else}
  <aside class="notes" aria-label={t('browser.notesTitle')}>
    <div class="head">
      <strong>{t('browser.notesTitle')}</strong>
      <span class="total">{notes.length || ''}</span>
      <button class="icon" aria-label={t('browser.foldNotes')} title={t('browser.foldNotes')} onclick={fold}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
      </button>
      <button class="icon" aria-label={t('browser.closeNotes')} title={t('browser.closeNotes')} onclick={onclose}>×</button>
    </div>
    <div class="list">
      {#each notes as note, i (note)}
        <div class="note">
          <div class="top">
            <span class="n">{i + 1}</span>
            <span class="what" title={note.picked?.selector}>{what(note.picked)}</span>
            <button class="icon" aria-label={t('browser.removeNote', { n: i + 1 })} onclick={() => notes.splice(i, 1)}>×</button>
          </div>
          <textarea
            bind:this={fields[i]}
            bind:value={note.comment}
            placeholder={t('browser.notePlaceholder')}
            aria-label={t('browser.notePlaceholder')}
            onkeydown={(e) => e.key === 'Enter' && (e.ctrlKey || e.metaKey) && (e.preventDefault(), send())}
          ></textarea>
        </div>
      {:else}
        <p>{t('browser.notesHint')}</p>
      {/each}
    </div>
    <div class="foot">
      <button class="btn btn-ghost" disabled={!notes.length} onclick={() => (notes = [])}>{t('browser.clearNotes')}</button>
      <button class="btn btn-primary" disabled={!notes.length || sending} onclick={send}>{t('browser.sendNotes')}</button>
    </div>
  </aside>
{/if}

<style>
  .ink {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    cursor: crosshair;
    touch-action: none;
    transition: translate 0.18s ease;
  }

  polyline,
  .box {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .box {
    stroke-width: 2;
    stroke-dasharray: 6 4;
  }

  .hover {
    fill: color-mix(in srgb, var(--accent) 12%, transparent);
    stroke: var(--accent);
    stroke-width: 1;
  }

  .badge circle {
    fill: var(--accent);
  }

  .badge text {
    fill: var(--accent-ink);
    font-size: 12px;
    font-weight: 700;
    text-anchor: middle;
  }

  .notes {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 250px;
    display: flex;
    flex-direction: column;
    background: var(--bg-sidebar);
    box-shadow:
      -1px 0 0 var(--border),
      -14px 0 36px rgba(0, 0, 0, 0.35);
  }

  .head {
    height: 40px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border);
    font-size: 13px;
  }

  .head strong {
    font-weight: 600;
  }

  .total {
    flex: 1;
    margin-left: 6px;
    color: var(--text-dim);
  }

  .icon {
    width: 24px;
    height: 24px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 4px;
    color: var(--text-dim);
    font-size: 15px;
  }

  .icon:hover {
    background: var(--surface-4);
    color: var(--text-strong);
  }

  .icon svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
  }

  .list p {
    margin: 0;
    padding: 6px 4px;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .note {
    padding: 8px;
    border-radius: 4px;
    background: var(--surface-1);
  }

  .top {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
  }

  .n,
  .count {
    min-width: 18px;
    height: 18px;
    flex: none;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 11px;
    font-weight: 700;
    line-height: 18px;
    text-align: center;
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font: 11.5px var(--mono);
    color: var(--text-dim);
  }

  textarea {
    width: 100%;
    min-height: 44px;
    max-height: 160px;
    field-sizing: content;
    display: block;
    padding: 6px 8px;
    border: none;
    border-radius: 4px;
    outline: none;
    resize: vertical;
    background: var(--surface-2);
    color: var(--text);
    font: 12.5px/1.4 var(--sans);
  }

  textarea:focus {
    box-shadow: inset 0 0 0 2px var(--border-focus);
  }

  textarea::placeholder {
    color: var(--text-faint);
  }

  .foot {
    flex: none;
    display: flex;
    gap: 6px;
    padding: 10px 8px;
    border-top: 1px solid var(--border);
  }

  .foot .btn {
    height: 30px;
    padding: 0 12px;
    font-size: 12.5px;
  }

  .foot .btn-primary {
    flex: 1;
  }

  .tab {
    position: absolute;
    top: 12px;
    right: 0;
    width: 34px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    border-radius: 6px 0 0 6px;
    background: var(--bg-sidebar);
    box-shadow:
      0 0 0 1px var(--border-strong),
      -8px 8px 24px rgba(0, 0, 0, 0.35);
  }

  @media (prefers-reduced-motion: reduce) {
    .ink {
      transition: none;
    }
  }
</style>
