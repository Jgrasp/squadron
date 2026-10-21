# Squadron — Mods et pilotage par IA (étude)

Trois sujets :

1. remanier l'interface ;
2. y ajouter ses propres éléments ;
3. laisser une IA administrer Squadron, y compris écrire ces éléments.

## Ce qui existe déjà et servira

- **Un serveur MCP local** (rmcp, HTTP sur `127.0.0.1`, jeton par lancement), déjà branché sur chaque agent pour les permissions. Le serveur de pilotage peut tourner sur ce même serveur.
- **Le pont Rust ↔ fenêtre** en requête/réponse (permissions, fenêtre flottante du Kanban). Il permettra au serveur MCP d'agir sur l'état des agents, qui vit dans la fenêtre.
- **Un registre d'actions nommées** (`shortcuts.ts` : nouvel agent, Kanban, panneaux…). Il deviendra le vocabulaire commun des boutons, des raccourcis, des mods et du MCP.
- **Un panneau latéral générique** (`SidePanel`), qui sert déjà au Diff, à l'éditeur et au terminal. Un panneau de mod s'y branchera de la même façon.
- **Des fichiers JSON simples** (`settings.json`, `projects.json` avec les cartes) qu'un agent peut lire et écrire.
- **Escouade**, la référence, expose un serveur MCP de pilotage sur un port fixe avec un jeton (`mcpPort`). Sa configuration ne montre rien sur des mods.

## Ce que font Claude Code et l'app Claude, et ce qu'on en retient

On s'en inspire sans en dépendre : ces mécanismes changent d'une version à l'autre et restent peu fiables, c'est justement pour cela que Squadron existe.

- **Mods Claude Code** (« function hooks », en accès anticipé). Exemple installé : `marco-mods-aja-fork` (task-board, github-issues). Un mod est un module TypeScript `register(on)` qui :
  - s'abonne à des événements (`session.start`, `tool.call`, `turn.complete`, résultat de `TaskCreate` ou `ExitPlanMode`…) ;
  - ajoute un panneau, des commandes `/` et des outils que Claude peut appeler ;
  - garde un état ;
  - se valide et se teste (`claude plugin validate`, `claude plugin test`) et se recharge à chaud (`--plugin-dir`).

  Leur code tourne avec tous les droits de Claude Code (« lisez le source avant d'installer »).
- **Plugins Claude Code** : un manifeste, puis des compétences, des commandes, des sous-agents, des hooks shell et des serveurs MCP. Ils sont distribués par des dépôts git (« marketplaces ») et activés par utilisateur ou par projet.
- **App Claude** :
  - connecteurs MCP ;
  - extensions de bureau (un serveur MCP empaqueté avec son manifeste) ;
  - artifacts : des pages HTML isolées qui déclarent leurs capacités (stockage, base, appel à Claude) et parlent à l'hôte par une API dédiée.

**Ce qu'on reprend** :

- un manifeste qui regroupe toutes les contributions ;
- des réactions aux événements des agents ;
- des outils de mod proposés aux agents ;
- des panneaux isolés qui déclarent leurs capacités ;
- des commandes de validation et de test ;
- le rechargement à chaud ;
- l'activation et la désactivation de chaque mod.

**Ce qu'on fait autrement** :

- Squadron est l'hôte, donc l'API des mods est la sienne : stable, versionnée (`"api": 1` dans le manifeste) et documentée par Squadron lui-même, sans dépendre des événements internes de Claude Code.
- Pas de code de mod avec tous les droits : du déclaratif d'abord, du code seulement dans un iframe isolé.
- Une erreur de mod se voit toujours (Settings › Mods, Activity), jamais un échec silencieux.
- Aucune mise à jour automatique d'un mod : tout changement de ses permissions redemande l'accord.

## Le principe commun : des emplacements et des actions

- **Emplacement** : un endroit nommé de l'interface qui contient une liste ordonnée d'éléments.
  - Premiers emplacements : `agent.tools` (Editor, Terminal, Diff, Kanban), `menu.squadron`, `composer.actions`, `kanban.card`.
  - Plus tard : en-tête, onglets de projet, liste des agents.
- **Élément** : un bouton, une entrée de menu, un texte ou un panneau, avec un identifiant stable. Il est intégré à Squadron (`editor`, `terminal`…) ou apporté par un mod (`lando.logs`).
- **Action** : ce que Squadron sait faire, avec un nom. Par exemple `panel.open`, `agent.new`, `agent.send`, `terminal.run`, `url.open`, `card.new`, `prompt.insert`. Un bouton, un raccourci, une entrée de menu et un appel MCP déclenchent la même action.

Ce modèle unique couvre les trois sujets :

1. réordonner ou masquer les éléments d'un emplacement ;
2. ajouter des éléments ;
3. laisser l'IA appeler les actions et écrire les éléments.

## Sujet 1 — Remanier l'interface

- **`layout.json`** dans le dossier de configuration : pour chaque emplacement, l'ordre des éléments et ceux qui sont masqués. C'est un fichier, pas le stockage local de la fenêtre, pour que l'IA puisse le lire et l'écrire et qu'il se copie d'une machine à l'autre.
- **Deux façons de le modifier**, qui écrivent le même fichier :
  - le **mode « Customize »**, lancé depuis le menu Squadron : les emplacements s'entourent de pointillés, on glisse-dépose les éléments, on les masque (œil) et on revient au défaut ; `Échap` fait sortir du mode ;
  - l'**onglet Settings › Layout** : chaque emplacement avec ses éléments, à monter, descendre ou masquer, et un retour au défaut.
- **Premier périmètre** : boutons d'outils de l'agent, menu Squadron, actions de la zone de saisie, boutons des cartes Kanban.
- **Plus tard** : déplacer un élément d'un emplacement à un autre (Terminal dans le menu, par exemple), quand les deux emplacements acceptent le même type d'élément.

Taille : S à M.

## Sujet 2 — Ajouter ses éléments : les mods

Un mod est un dossier qui contient un manifeste `mod.json` et ses fichiers. Il y a deux niveaux.

### A. Mods déclaratifs

Le manifeste décrit ce que le mod ajoute, sans code :

- des boutons et des entrées de menu, placés dans un emplacement, qui déclenchent une action ;
- des panneaux de contenu simple : Markdown, page web externe, sortie d'une commande rafraîchie ;
- des raccourcis vers ses actions ;
- des modèles de prompt, insérés dans la zone de saisie ;
- un thème : surcharge des couleurs Cuivre ;
- des réactions aux événements, qui déclenchent une action : agent qui a fini, qui attend, qui a appelé un outil, carte déplacée. Par exemple, une notification système quand un agent attend, ou `lando test` à la fin d'un tour ;
- des outils proposés aux agents, servis par le serveur MCP de Squadron. Par exemple un outil `lando_logs` qui lance une commande et renvoie sa sortie à l'agent.

```json
{
  "id": "lando",
  "name": "Lando",
  "api": 1,
  "contributes": {
    "buttons": [
      { "slot": "agent.tools", "id": "logs", "label": "Logs", "icon": "scroll", "action": { "terminal.run": "lando logs -f" } },
      { "slot": "agent.tools", "id": "info", "label": "Lando", "action": { "panel.open": "info" } }
    ],
    "panels": [{ "id": "info", "title": "Lando", "command": "lando info", "refresh": 10 }],
    "prompts": [{ "id": "review", "label": "Review the diff", "text": "Relis le diff et liste les risques." }],
    "keys": [{ "button": "logs", "key": "Ctrl+L" }],
    "on": [{ "event": "agent.done", "action": { "terminal.run": "lando test" } }],
    "tools": [{ "name": "lando_logs", "description": "Last 200 lines of the Lando logs", "command": "lando logs --tail 200" }]
  },
  "permissions": ["commands"]
}
```

### B. Panneaux web

Un mod peut fournir une page (`panel.html` et son JS). Elle s'affiche dans un iframe isolé et dialogue avec Squadron par une petite API (`postMessage`). Cette API permet par exemple de :

- lire les agents ;
- envoyer un message à un agent ;
- lire un fichier du projet ;
- lancer une commande ;
- créer une carte.

Chaque capacité est déclarée dans le manifeste et acceptée à l'activation du mod.

### Où vivent les mods

- **Mods personnels seulement** : `~/.config/dev.squadron.desktop/mods/<id>/`. Pas de mods rangés dans les dépôts des projets.
- **Plus tard, pas prévu pour l'instant** : installer un mod depuis GitHub.
- **Rechargement à chaud** : Squadron surveille ces dossiers. Une erreur de manifeste s'affiche dans Settings › Mods sans casser l'application.

### Sécurité

- **Pas de JavaScript de mod dans la fenêtre principale.** Il y aurait accès à toutes les commandes Tauri (shell, fichiers, agents). D'où l'iframe isolé et l'API filtrée.
- **Commandes shell** : elles sont listées et approuvées à l'activation du mod, puis tournent sans redemander. Toute nouvelle commande ajoutée au mod redemande l'accord.
- **Réseau** : interdit aux panneaux web, sauf permission déclarée.

Taille : A = M, B = M à L.

## Sujet 3 — Piloter Squadron par une IA

### Le serveur MCP « squadron »

Il expose trois familles d'outils.

- **Lire** :
  - projets, agents (état, coût, ce qu'ils attendent) ;
  - conversation d'un agent (derniers messages), son diff ;
  - cartes Kanban, réglages, layout et mods.
- **Préparer et lancer**, comme l'utilisateur l'aurait fait lui-même :
  - créer un agent et lui envoyer son premier message ;
  - créer, déplacer ou lancer une carte ;
  - ouvrir un panneau, afficher une notification.

  L'IA n'agit jamais à la place de l'utilisateur sur les agents existants :
  - elle ne répond pas aux permissions, aux questions ni aux plans ;
  - elle n'écrit pas à un autre agent ;
  - elle n'en interrompt ni n'en archive aucun.

  Les demandes d'un agent qu'elle a lancé arrivent à l'utilisateur, comme pour tout agent. Sa conversation indique « Launched by X ».
- **Construire** :
  - documentation des emplacements, des actions et du manifeste ;
  - écrire un mod, le vérifier, le recharger ;
  - modifier le layout.

### Qui s'y connecte

Uniquement les agents lancés par Squadron, sur option (par exemple un agent « Admin »). Le serveur est ajouté à leur `--mcp-config`, et l'en-tête `x-squadron-agent` identifie l'agent qui agit. Il n'y a pas d'accès pour les clients externes.

### Garde-fous

- **Périmètre** : par défaut, un agent ne lit et ne lance que dans son propre projet. Le pilotage de tous les projets est une option.
- **Confirmation dans l'interface**, avec la même carte que les permissions, pour :
  - lancer un agent dans un mode qui saute des permissions (`auto`, `acceptEdits`) ;
  - activer un mod qui lance des commandes.
- **Pas d'autopilote** : aucun outil ne valide quoi que ce soit à la place de l'utilisateur.
- **Journal « Activity »** : tout ce que l'IA a fait, et par qui.
- **Plafonds** : nombre d'agents créés, coût du jour.

### Coder un mod à la place de l'utilisateur

Le dossier des mods peut s'ajouter comme un projet Squadron. Un agent l'édite avec ses outils habituels :

1. il lit la documentation fournie par l'outil MCP `mods_docs` ;
2. il vérifie son travail avec `mod_check`, qui contrôle le manifeste, les emplacements, les actions et les permissions ;
3. Squadron recharge le mod à chaud, et le résultat est visible tout de suite.

### Technique et risques

- **Technique** : l'état des agents vit dans la fenêtre (Svelte). Le serveur MCP (Rust) relaie chaque appel à la fenêtre et attend sa réponse, comme pour les permissions. Rust gère directement les cartes, les réglages, le layout et les mods, qui sont des fichiers.
- **Risque principal : l'injection de prompt.** Un agent qui lit une page piégée pourrait utiliser le pilotage. Le risque reste borné : il ne peut que lancer des agents dont les demandes passent par l'utilisateur, et tout ce qui élargit les droits demande une confirmation.

Taille : M pour la lecture et l'action, S pour les outils de mods.

## Ordre proposé

1. **Socle** : registre des emplacements et des actions. Les boutons, menus et raccourcis existants y passent. Taille S. *Fait.*
2. **Sujet 1** : `layout.json`, mode Customize et Settings › Layout. Taille S à M. *Fait : outils de l'agent, menu ⋯ de l'agent, menu Squadron.*
3. **Sujet 2A** : mods déclaratifs, Settings › Mods, rechargement à chaud. Taille M.
4. **Sujet 3** : serveur MCP de pilotage (lecture, préparation et lancement, outils de mods), garde-fous, Activity. Taille M. *Fait avant l'étape 3, après qu'un agent a modifié le code de Squadron pour déplacer un bouton : outils `overview`, `read_agent`, `start_agent`, `create_card`, `move_card`, `get_layout`, `set_layout`. Restent les outils de mods et le journal Activity.*
5. **Sujet 2B** : panneaux web isolés avec leur API. Taille M à L.

Le pilotage passe avant les panneaux web : l'IA sait déjà écrire des mods déclaratifs, et l'API d'actions est éprouvée avant d'être ouverte au code des mods.

## Décisions

1. **Code dans les mods** : du déclaratif, plus des panneaux web isolés. Pas de JavaScript libre dans l'application.
2. **Mods** : personnels seulement, dans le dossier de config. L'installation depuis GitHub viendra plus tard, elle n'est pas prévue pour l'instant.
3. **Pilotage** : uniquement par les agents de Squadron.
4. **Autonomie de l'IA** : elle prépare et lance des agents et des cartes, comme l'utilisateur l'aurait fait. Elle ne valide jamais rien et n'agit jamais à la place de l'utilisateur sur les agents. Pas d'autopilote.
5. **Personnalisation** : les deux, le mode Customize dans l'interface et l'onglet Settings › Layout.
