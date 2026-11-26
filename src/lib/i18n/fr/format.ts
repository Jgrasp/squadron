import type { Catalog } from '../index.svelte'

export default {
  justNow: 'à l’instant',
  minutesAgo: 'il y a {count} min',
  hoursAgo: 'il y a {count} h',
  weeksAgo: { one: 'il y a {count} semaine', other: 'il y a {count} semaines' },
  monthsAgo: { one: 'il y a {count} mois', other: 'il y a {count} mois' },
  yearsAgo: { one: 'il y a {count} an', other: 'il y a {count} ans' },
  seconds: '{s} s',
  minutes: '{m} min {s} s',
  thousands: '{n} k',
  millions: '{n} M',
  billions: '{n} Md',
  modeDefault: 'tout demander',
  modeAcceptEdits: 'accepter les modifications',
  modePlan: 'plan',
  modeAuto: 'auto',
} satisfies Catalog['format']
