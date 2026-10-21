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

export const hidden = (slot: Slot, id: string) => !!arrangement[slot]?.hidden?.includes(id)

export const shown = (slot: Slot) => all(slot).filter((id) => !hidden(slot, id))

export const labelOf = (slot: Slot, id: string) => SLOTS[slot].items.find(([i]) => i === id)?.[1] ?? id

export function place(slot: Slot, id: string, index: number) {
  arrangement[slot] = { ...arrangement[slot], order: moved(all(slot), id, index) }
  save()
}

export function toggle(slot: Slot, id: string) {
  if (SLOTS[slot].locked?.includes(id)) return
  const list = arrangement[slot]?.hidden ?? []
  arrangement[slot] = { ...arrangement[slot], hidden: list.includes(id) ? list.filter((h) => h !== id) : [...list, id] }
  save()
}

export function reset(slot?: Slot) {
  for (const key of slot ? [slot] : Object.keys(arrangement)) delete arrangement[key]
  save()
}

export function sortable(node: HTMLElement, slot: Slot) {
  function down(event: PointerEvent) {
    const item = (event.target as HTMLElement).closest<HTMLElement>('[data-item]')
    if (!node.classList.contains('customizing') || event.button !== 0 || !item || (event.target as HTMLElement).closest('[data-keep]')) return
    event.preventDefault()
    const id = item.dataset.item!
    item.setPointerCapture(event.pointerId)
    item.classList.add('dragging')
    const row = getComputedStyle(node).flexDirection.startsWith('row')
    const move = (e: PointerEvent) => {
      if (!e.buttons) return up()
      const others = [...node.querySelectorAll<HTMLElement>('[data-item]')].filter((el) => el !== item)
      const index = others.filter((el) => {
        const r = el.getBoundingClientRect()
        return row ? e.clientX > r.left + r.width / 2 : e.clientY > r.top + r.height / 2
      }).length
      if (index !== all(slot).indexOf(id)) place(slot, id, index)
    }
    const up = () => {
      item.classList.remove('dragging')
      item.removeEventListener('pointermove', move)
      item.removeEventListener('pointerup', up)
      item.removeEventListener('pointercancel', up)
    }
    item.addEventListener('pointermove', move)
    item.addEventListener('pointerup', up)
    item.addEventListener('pointercancel', up)
  }
  node.addEventListener('pointerdown', down)
  return { destroy: () => node.removeEventListener('pointerdown', down) }
}
