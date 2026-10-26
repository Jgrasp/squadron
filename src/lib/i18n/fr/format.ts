import type { Catalog } from '../index.svelte'

export default {
  justNow: 'à l’instant',
  minutesAgo: 'il y a {count} min',
  hoursAgo: 'il y a {count} h',
  seconds: '{s} s',
  minutes: '{m} min {s} s',
  thousands: '{n} k',
  millions: '{n} M',
  modeDefault: 'tout demander',
  modeAcceptEdits: 'accepter les modifications',
  modePlan: 'plan',
  modeAuto: 'auto',
  meta: '{model} · effort {effort} · {mode}',
} satisfies Catalog['format']
