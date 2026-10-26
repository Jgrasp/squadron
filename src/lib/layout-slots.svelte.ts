import { invoke } from '@tauri-apps/api/core'
import { t } from './i18n/index.svelte'
import { arrange, movedAmong, SLOTS, type Arrangement, type Slot } from './slots'

export const arrangement = $state<Record<string, Arrangement>>({})

let timer = 0
function save() {
  clearTimeout(timer)
  timer = window.setTimeout(() => invoke('set_layout', { layout: $state.snapshot(arrangement) }), 250)
}

export async function loadLayout() {
  Object.assign(arrangement, await invoke<Record<string, Arrangement>>('get_layout'))
}

export const all = (slot: Slot) =>
  arrange(
    SLOTS[slot].items.map(([id]) => id),
    arrangement[slot]?.order,
  )

export const hidden = (slot: Slot, id: string) => (arrangement[slot]?.hidden ?? SLOTS[slot].hidden ?? []).includes(id)

export const shown = (slot: Slot) => all(slot).filter((id) => !hidden(slot, id))

const itemOf = (slot: Slot, id: string) => SLOTS[slot].items.find(([i]) => i === id)

export const labelOf = (slot: Slot, id: string, tr = t) => {
  const key = itemOf(slot, id)?.[1]
  return key ? tr(key) : id
}

export function place(slot: Slot, id: string, index: number, among = all(slot)) {
  arrangement[slot] = { ...arrangement[slot], order: movedAmong(all(slot), among, id, index) }
  save()
}

export function toggle(slot: Slot, id: string) {
  if (SLOTS[slot].locked?.includes(id)) return
  const list = arrangement[slot]?.hidden ?? SLOTS[slot].hidden ?? []
  arrangement[slot] = { ...arrangement[slot], hidden: list.includes(id) ? list.filter((h) => h !== id) : [...list, id] }
  save()
}

export function arrangeSlot(slot: Slot, order?: string[] | null, hide?: string[] | null) {
  const known = SLOTS[slot].items.map(([id]) => id)
  const unknown = [...(order ?? []), ...(hide ?? [])].filter((id) => !known.includes(id))
  if (unknown.length) throw new Error(`Unknown items in ${slot}: ${unknown.join(', ')}. Items: ${known.join(', ')}.`)
  const locked = (hide ?? []).filter((id) => SLOTS[slot].locked?.includes(id))
  if (locked.length) throw new Error(`${locked.join(', ')} cannot be hidden.`)
  arrangement[slot] = {
    order: order ? arrange(known, order) : arrangement[slot]?.order,
    hidden: hide ?? arrangement[slot]?.hidden,
  }
  save()
}

export function reset(slot?: Slot) {
  for (const key of slot ? [slot] : Object.keys(arrangement)) delete arrangement[key]
  save()
}

export const detailOf = (slot: Slot, id: string, tr = t) => {
  const key = itemOf(slot, id)?.[2]
  return key ? tr(key) : ''
}

type Sortable = { enabled: boolean; onplace: (id: string, index: number) => void }

export function sortable(node: HTMLElement, options: Sortable) {
  let opts = options
  function down(event: PointerEvent) {
    const target = event.target as HTMLElement
    const item = target.closest<HTMLElement>('[data-item]')
    if (!opts.enabled || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || !item || !node.contains(item) || target.closest('[data-keep]')) return
    const items = [...node.querySelectorAll<HTMLElement>('[data-item]')]
    const from = items.indexOf(item)
    const row = getComputedStyle(node).flexDirection.startsWith('row')
    const rects = items.map((el) => el.getBoundingClientRect())
    const lead = (r: DOMRect) => (row ? r.left : r.top)
    const size = (r: DOMRect) => (row ? r.width : r.height)
    const gap = rects.length > 1 ? lead(rects[1]) - lead(rects[0]) - size(rects[0]) : 0
    const offset = size(rects[from]) + gap
    const axis = (px: number) => (px ? `translate${row ? 'X' : 'Y'}(${px}px)` : '')
    const start = row ? event.clientX : event.clientY
    let to = from
    let far = false
    const move = (e: PointerEvent) => {
      if (!e.buttons) return up()
      const delta = (row ? e.clientX : e.clientY) - start
      if (!far) {
        if (Math.abs(delta) <= 4) return
        far = true
        item.setPointerCapture(e.pointerId)
        item.classList.add('dragging')
        document.documentElement.style.userSelect = 'none'
        getSelection()?.removeAllRanges()
        for (const el of items) el.style.transition = el === item ? 'none' : 'transform 160ms ease'
      }
      item.style.transform = axis(delta)
      const center = lead(rects[from]) + size(rects[from]) / 2 + delta
      to = rects.filter((r, k) => k !== from && center > lead(r) + size(r) / 2).length
      items.forEach((el, k) => {
        if (el !== item) el.style.transform = axis(from < to && k > from && k <= to ? -offset : to < from && k >= to && k < from ? offset : 0)
      })
    }
    const up = () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      window.removeEventListener('pointercancel', up)
      if (!far) return
      item.classList.remove('dragging')
      document.documentElement.style.userSelect = ''
      for (const el of items) {
        el.style.transition = 'none'
        el.style.transform = ''
      }
      if (to !== from) opts.onplace(item.dataset.item!, to)
      const swallow = (e: Event) => e.stopPropagation()
      node.addEventListener('click', swallow, true)
      setTimeout(() => node.removeEventListener('click', swallow, true))
      requestAnimationFrame(() => items.forEach((el) => (el.style.transition = '')))
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
    window.addEventListener('pointercancel', up)
  }
  const still = (e: DragEvent) => opts.enabled && e.preventDefault()
  node.addEventListener('pointerdown', down)
  node.addEventListener('dragstart', still)
  return {
    update: (next: Sortable) => (opts = next),
    destroy: () => (node.removeEventListener('pointerdown', down), node.removeEventListener('dragstart', still)),
  }
}
