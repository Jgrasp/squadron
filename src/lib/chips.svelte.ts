import { MediaQuery } from 'svelte/reactivity'
import { settings, type ChipsMotion } from './settings.svelte'

const reduced = new MediaQuery('(prefers-reduced-motion: reduce)')

export const chipClock = $state({ tick: 0 })

export const chipMotion = (): ChipsMotion => (reduced.current ? 'off' : settings.chips_motion)

export function chipAt(count: number, tick: number, { err, paused, shown, turns }: { err: number; paused: boolean; shown: number; turns: boolean }) {
  if (err >= 0) return err
  if (paused) return Math.min(shown, count - 1)
  return turns ? tick % count : 0
}

$effect.root(() => {
  $effect(() => {
    if (chipMotion() === 'off') return
    const timer = setInterval(() => chipClock.tick++, settings.chips_every * 1000)
    return () => clearInterval(timer)
  })
})
