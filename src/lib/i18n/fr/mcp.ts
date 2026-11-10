import type { Catalog } from '../index.svelte'

export default {
  tab: 'Serveurs MCP',
  intro: "Un serveur à la demande ne démarre dans aucune session. Un agent le démarre pour lui seul avec /mcp, ou de lui-même quand sa tâche en a besoin, et le garde après une relance jusqu'à ce qu'il l'arrête.",
  onDemand: 'À la demande',
  everywhere: 'Dans chaque session',
  everywhereIntro: 'Claude Code démarre ces serveurs dans chaque session de chaque agent, depuis sa configuration utilisateur.',
  toOnDemand: 'Passer à la demande',
  toEverywhere: 'Remettre partout',
  description: 'À quoi il sert, pour que les agents sachent quand le démarrer',
  noneOnDemand: 'Aucun serveur à la demande pour l’instant.',
  noneEverywhere: 'Aucun serveur dans la configuration utilisateur de Claude Code.',
  hintOff: 'à la demande · éteint',
  hintOn: 'à la demande · allumé',
  start: 'Démarrer pour cet agent',
  stop: 'Arrêter pour cet agent',
  started: '{server} démarre pour cet agent.',
  stopped: '{server} est arrêté pour cet agent.',
} satisfies Catalog['mcp']
