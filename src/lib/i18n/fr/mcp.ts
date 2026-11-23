import type { Catalog } from '../index.svelte'

export default {
  tab: 'Serveurs MCP',
  intro: "Serveurs de la configuration utilisateur de Claude Code. À la demande, un serveur ne démarre dans aucune session : un agent le démarre pour lui seul avec /mcp, ou de lui-même quand sa tâche en a besoin.",
  onDemand: 'À la demande',
  everywhere: 'Chaque session',
  description: 'À quoi il sert, pour que les agents sachent quand le démarrer',
  none: 'Aucun serveur dans la configuration utilisateur de Claude Code.',
  hintOff: 'à la demande · éteint',
  hintOn: 'à la demande · allumé',
  start: 'Démarrer pour cet agent',
  stop: 'Arrêter pour cet agent',
  started: '{server} démarre pour cet agent.',
  stopped: '{server} est arrêté pour cet agent.',
} satisfies Catalog['mcp']
