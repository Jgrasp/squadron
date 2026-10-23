import { invoke } from '@tauri-apps/api/core'
import { arrange, moved, SLOTS, type Arrangement, type Slot } from './slots'

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

export const labelOf = (slot: Slot, id: string) => SLOTS[slot].items.find(([i]) => i === id)?.[1] ?? id

export function place(slot: Slot, id: string, index: number) {
  arrangement[slot] = { ...arrangement[slot], order: moved(all(slot), id, index) }
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

export const detailOf = (slot: Slot, id: string) => SLOTS[slot].items.find(([i]) => i === id)?.[2] ?? ''

export function sortable(node: HTMLElement, options: { slot: Slot; enabled: boolean }) {
  let opts = options
  function down(event: PointerEvent) {
    const target = event.target as HTMLElement
    const item = target.closest<HTMLElement>('[data-item]')
    if (!opts.enabled || event.button !== 0 || !item || !node.contains(item) || target.closest('[data-keep]')) return
    event.preventDefault()
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
    item.setPointerCapture(event.pointerId)
    item.classList.add('dragging')
    for (const el of items) el.style.transition = el === item ? 'none' : 'transform 160ms ease'
    const move = (e: PointerEvent) => {
      if (!e.buttons) return up()
      const delta = (row ? e.clientX : e.clientY) - start
      item.style.transform = axis(delta)
      const center = lead(rects[from]) + size(rects[from]) / 2 + delta
      to = rects.filter((r, k) => k !== from && center > lead(r) + size(r) / 2).length
      items.forEach((el, k) => {
        if (el !== item) el.style.transform = axis(from < to && k > from && k <= to ? -offset : to < from && k >= to && k < from ? offset : 0)
      })
    }
    const up = () => {
      item.removeEventListener('pointermove', move)
      item.removeEventListener('pointerup', up)
      item.removeEventListener('pointercancel', up)
      item.classList.remove('dragging')
      for (const el of items) {
        el.style.transition = 'none'
        el.style.transform = ''
      }
      if (to !== from) place(opts.slot, item.dataset.item!, to)
      requestAnimationFrame(() => items.forEach((el) => (el.style.transition = '')))
    }
    item.addEventListener('pointermove', move)
    item.addEventListener('pointerup', up)
    item.addEventListener('pointercancel', up)
  }
  node.addEventListener('pointerdown', down)
  return {
    update: (next: { slot: Slot; enabled: boolean }) => (opts = next),
    destroy: () => node.removeEventListener('pointerdown', down),
  }
}
