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
- **Extensions Zed**, le modèle retenu pour la logique et la distribution :
  - du Rust compilé en WebAssembly (`wasm32-wasip2`), exécuté par Wasmtime derrière une API typée et versionnée ;
  - un manifeste qui déclare les contributions et des permissions fines (par exemple `process:exec` limité à une commande et à ses arguments) ;
  - un registre central, le dépôt `zed-industries/extensions`, où l'on entre par une PR relue ;
  - « Install Dev Extension » pour charger une extension depuis un dossier local ;
  - installation à chaud ;
  - en revanche, aucune interface : ni panneau, ni bouton, ni barre d'état. Le RFC #53403 le demande et l'équipe l'a repoussé (« not something we're likely to get to in the near future »). C'est ce que Squadron ajoute.

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
- Pas de code de mod avec tous les droits : la logique tourne en WASM, et les panneaux dans un iframe isolé. Aucun JavaScript de mod dans la fenêtre principale.
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

Un mod est un dossier qui contient un manifeste `mod.json`, ses fichiers et, s'il a de la logique, un module WASM. On peut le développer en local, l'installer depuis la marketplace et le désinstaller, toujours sans redémarrer Squadron.

### Ce qu'un mod peut ajouter

Le but est d'ouvrir le plus de zones possible. Chaque zone de l'interface devient un emplacement, et chaque emplacement accepte des éléments de mod.

- **Éléments dans les zones de Squadron** : boutons (outils de l'agent), entrées de menu (menu ⋯ de l'agent, menu Squadron), lignes du panneau d'info, badges, boutons des cartes Kanban, onglet de Settings, raccourcis clavier.
- **Surfaces propres au mod** : panneau latéral (`SidePanel`), panneau d'info sous l'en-tête, carte dans la conversation.
- **Comportements** :
  - réactions aux événements : agent qui a fini, qui attend, qui a appelé un outil, carte déplacée ;
  - actions de Squadron, celles du registre ;
  - modèles de prompt ;
  - outils proposés aux agents, servis par le serveur MCP de Squadron.
- **Thème** : surcharge des variables de couleur de Cuivre.

Les identifiants des éléments sont préfixés par le mod (`lando.logs`). Ils ne peuvent donc jamais entrer en collision avec un élément intégré, et ils s'ajoutent à `layout.json` comme les autres : `arrange()` ajoute déjà les éléments nouveaux et retire ceux qui ont disparu.

### Design

- **Dans les zones de Squadron, aucune liberté.** Le mod fournit un libellé `en` et `fr`, une icône, un état et l'action du clic, et Squadron dessine l'élément en Cuivre. L'interface reste cohérente, et une refonte de Squadron ne casse aucun mod.
- **Dans ses propres panneaux, liberté totale.** Une page HTML/CSS/JS au choix, dans un iframe isolé, servie par un schéma `mod://<id>/` sur le modèle du schéma `artifact:`. Squadron fournit une `mod-kit.css` (variables Cuivre, boutons, champs) et pousse les variables du thème quand elles changent. Son usage est facultatif.
- **Jamais hors de son rectangle** : un mod n'ajoute pas de popup par-dessus l'application et n'injecte pas de CSS dans Squadron.

### Logique : Rust compilé en WASM

- Le code tourne côté Rust dans Wasmtime, une instance par mod, avec des plafonds de mémoire et de temps de calcul. Il continue de tourner quand ses panneaux sont fermés : réactions aux événements, état, outils.
- L'API est décrite en WIT et publiée en crate `squadron_mod_api`. Elle est versionnée (`"api": 1` dans le manifeste).
- Le panneau d'un mod (JS dans l'iframe) parle à son WASM par messages relayés par Squadron. Il n'accède jamais à l'IPC Tauri.
- Un mod sans logique (boutons vers des actions, thème, prompts) n'a pas de WASM.
- Une panique, un dépassement de plafond ou une erreur de manifeste désactive le mod et s'affiche dans Settings › Mods, sans effet sur Squadron.

### Permissions

Elles sont déclarées dans le manifeste, montrées à l'installation et acceptées une par une. Une mise à jour qui les change redemande l'accord.

- `process:exec` : une commande et ses arguments autorisés, comme dans Zed.
- `network` : une liste de domaines.
- `agents:read`, `agents:send`, `cards:write`, `project:read` : l'accès aux données et aux actions de Squadron.
- `tools` : outils proposés aux agents.

Le sandbox ne protège que ce que le mod ne peut pas faire. Une commande autorisée tourne avec tous les droits de l'utilisateur. Avec une marketplace, la vraie barrière est l'écran de permissions, et il doit le dire clairement.

### Services externes et jetons

- Le mod déclare ses réglages. Les secrets vont dans le keyring.
- Les requêtes HTTP partent de Rust, seulement vers les domaines déclarés, et Squadron y ajoute lui-même le jeton. Les commandes reçoivent le jeton en variable d'environnement.
- Le WASM et le panneau ne lisent jamais un secret.

### Exemple de manifeste

```json
{
  "id": "lando",
  "version": "1.0.0",
  "api": 1,
  "name": { "en": "Lando", "fr": "Lando" },
  "wasm": "lando.wasm",
  "contributes": {
    "buttons": [
      { "slot": "agent.tools", "id": "logs", "label": { "en": "Logs", "fr": "Logs" }, "icon": "scroll", "action": { "terminal.run": "lando logs -f" } },
      { "slot": "agent.tools", "id": "info", "label": { "en": "Lando", "fr": "Lando" }, "action": { "panel.open": "info" } }
    ],
    "panels": [{ "id": "info", "title": { "en": "Lando", "fr": "Lando" }, "page": "panel/index.html" }],
    "keys": [{ "button": "logs", "key": "Ctrl+L" }],
    "tools": [{ "name": "lando_logs", "description": "Last 200 lines of the Lando logs" }],
    "settings": [{ "id": "token", "label": { "en": "API token", "fr": "Jeton d'API" }, "secret": true }]
  },
  "permissions": {
    "process:exec": [{ "command": "lando", "args": ["logs", "**"] }],
    "network": ["api.lando.dev"],
    "tools": true
  }
}
```

### Ce que fait l'API 1 aujourd'hui (étape 5)

- **Où** : `<config>/mods/<dossier>/mod.json`, ou un dossier ajouté depuis Settings › Mods (« Ajouter un dossier… »). Les dossiers ajoutés et les mods désactivés sont dans `<config>/mods.json`. Un changement de `mod.json` se recharge en moins d'une seconde.
- **Manifeste** : `id` (minuscules, chiffres, tirets), `version`, `api: 1`, `name` et `description` en `{ "en": "…", "fr": "…" }`, l'anglais étant obligatoire.
- **`contributes.items`** : `slot` parmi `agent.tools`, `agent.menu` et `menu.squadron`, `id`, `label`, `hint` facultatif, `icon` facultative, prise dans les icônes de Squadron, et une `action` parmi :
  - `{ "terminal.run": "lando logs -f" }` : tape la commande dans le terminal de l'agent sans la lancer, l'utilisateur appuie sur Entrée. Une seule ligne ;
  - `{ "prompt.insert": "…" }` : ajoute le texte à la zone de saisie de l'agent ;
  - `{ "url.open": "https://…" }` : ouvre le lien dans le navigateur ;
  - `{ "squadron": "kanban" }` : une action du registre (`newAgent`, `kanban`, `editor`, `terminal`, `diff`…).
  `terminal.run` et `prompt.insert` demandent un agent, donc pas de `menu.squadron`.
- **`contributes.theme`** : des variables de Cuivre et des couleurs, par exemple `{ "accent": "#4fa3e0" }`.
- **Clés des éléments** : `<mod>.<élément>`, par exemple `lando.logs`. Les éléments se rangent, se masquent et se pilotent par MCP (`get_layout`, `set_layout`) comme les éléments intégrés. Dans le menu de l'agent, ils s'insèrent avant Archiver.
- **Erreurs** : chaque problème du manifeste s'affiche, traduit, sous le mod dans Settings › Mods. Un mod en erreur n'ajoute rien.
- **`contributes.tools`** (étape 6a) : `[{ "name": "logs", "description": "…", "command": "lando logs --tail 200" }]`. Les agents voient l'outil `<mod>_<nom>` sur le serveur MCP `squadron-mods`, présent chez chaque agent. La commande tourne dans le dossier de l'agent, avec 120 s au plus, et l'agent reçoit les 30 000 derniers caractères de sa sortie. Claude Code demande la permission à chaque appel, comme pour tout outil. Un outil ajouté, retiré ou autorisé apparaît chez les agents déjà lancés, sans redémarrage : notification `tools/list_changed`, envoyée par `subscriptions/listen` depuis le protocole MCP 2026-07-28, et par la session pour les clients plus anciens.
- **Accord sur les commandes** (étape 6a) : dès qu'un mod arrive avec des commandes pas encore autorisées (`terminal.run` et outils), une popup les liste avec Autoriser ou Plus tard. « Plus tard » vaut pour la session : un clic sur un bouton du mod rouvre la popup, la demande reste dans Settings › Mods, et elle revient au prochain lancement. Avant l'accord, les outils restent cachés aux agents. Après, le bouton lance la commande dans le terminal visible et les outils sont proposés. Une commande nouvelle ou modifiée redemande l'accord (`approved` dans `mods.json`).

### Logique en WASM (étape 6b)

Un mod écrit sa logique en Rust avec le crate `squadron_mod_api` (`mods/api`), compilé en composant WebAssembly. L'exemple complet est `mods/examples/hello`.

```rust
use squadron_mod_api::{Agent, Mod, host, squadron_mod};

struct Hello;

impl Mod for Hello {
    fn run(action: String, agent: Option<Agent>) -> Result<(), String> {
        host::notify("Hello", &format!("{action} clicked"));
        Ok(())
    }
}

squadron_mod!(Hello);
```

1. `cargo build --release --target wasm32-wasip2` (la cible s'ajoute avec `rustup target add wasm32-wasip2`).
2. Dans `mod.json`, `"wasm": "target/wasm32-wasip2/release/<nom>.wasm"`, un chemin qui reste dans le dossier du mod.
3. Ajouter le dossier depuis Settings › Mods. Chaque build recharge le mod.

Ce que le manifeste ajoute :

- l'action `{ "mod": "<action>" }` pour un élément : Squadron appelle `run` ;
- un outil sans `command`, avec un `input` facultatif (schéma JSON de type object) : Squadron appelle `call-tool` avec l'entrée en JSON ;
- `"permissions": { "exec": ["git"] }` : les programmes que `host::exec` peut lancer, avec n'importe quels arguments, sans shell. La popup d'accord les présente ainsi (« git …, avec n'importe quels arguments »).

Comment ça tourne :

- une instance par mod, sur son propre fil ;
- 5 milliards d'unités de carburant par appel et 64 Mo de mémoire au plus ;
- WASI sans fichiers, sans réseau et sans variables d'environnement ;
- l'état de `state-set` va dans `<config>/mods-state/<id>.json`, 1 Mo au plus ;
- un mod qui plante est signalé dans Settings › Mods et repart sur une instance neuve, en gardant son état ;
- les événements `agent.done` et `agent.waiting` partent quand l'état d'un agent change.

Le contrat (`mods/api/wit/squadron.wit`). On peut lui ajouter des imports sans casser les mods déjà compilés. Ajouter des exports, en revanche, les casserait : les événements passent donc par un nom et du JSON.

```wit
package squadron:api@1.0.0;

interface types {
  /// The agent a call is about.
  record agent {
    id: string,
    name: string,
    project: string,
    /// The folder the agent works in: its worktree, or its project.
    dir: string,
  }

  record output {
    /// -1 when the program was stopped or could not report one.
    code: s32,
    stdout: string,
    stderr: string,
  }
}

interface host {
  use types.{output};

  /// Writes a line to Squadron's log.
  log: func(message: string);
  /// Shows a notification.
  notify: func(title: string, body: string);
  /// Adds text to the message the user is writing to an agent.
  insert-prompt: func(agent: string, text: string);
  /// Shows a short badge on one of the mod's items, or removes it.
  set-badge: func(item: string, badge: option<string>);
  /// Replaces the label of one of the mod's items, or restores the manifest's.
  set-label: func(item: string, label: option<string>);
  /// The mod's own storage, kept across restarts.
  state-get: func(key: string) -> option<string>;
  state-set: func(key: string, value: option<string>);
  /// Runs a program the user allowed in `permissions.exec`, without a shell, for at most 120 s.
  exec: func(program: string, args: list<string>, dir: option<string>) -> result<output, string>;
}

world squadron-mod {
  use types.{agent};
  import host;

  /// Something happened: `started` once the mod is loaded, `agent.done` when an agent finished its turn, `agent.waiting` when it waits for the user. `data` is JSON.
  export on-event: func(name: string, agent: option<agent>, data: string);
  /// One of the mod's items with the action `{ "mod": "<action>" }` was clicked.
  export run: func(action: string, agent: option<agent>) -> result<_, string>;
  /// An agent called one of the mod's tools that has no `command`. `input` is JSON.
  export call-tool: func(name: string, input: string, agent: option<agent>) -> result<string, string>;
}
```

### Distribution

- **Dossier** : `~/.config/dev.squadron.desktop/mods/<id>/`, surveillé à chaud.
- **Marketplace** : le dépôt `squadron-mods`, curé.
  - Un mod y entre par une PR que tu relis : un sous-module git et une entrée dans l'index.
  - La CI compile le WASM, empaquette le mod, calcule son sha256 et publie l'archive (GitHub Releases) et `index.json` (GitHub Pages).
  - Squadron lit l'index et affiche le catalogue dans Settings › Mods.
- **Installer** :
  1. télécharger l'archive ;
  2. vérifier le sha256 de l'index ;
  3. extraire dans un dossier neuf en refusant tout chemin qui en sort ;
  4. valider le manifeste ;
  5. montrer les permissions ;
  6. déplacer le dossier dans `mods/`. Le chargement à chaud fait le reste.
- **Désinstaller** : fermer ses panneaux, retirer ses éléments et ses outils, arrêter son instance WASM, supprimer le dossier.
- **Mises à jour** : notifiées, installées sur un clic. Jamais d'installation automatique.
- **Développer** : « Installer un mod local » charge un dossier. Le mod est rechargé à chaque build et prend la place de la version publiée.

### Outils des mods chez les agents

Le serveur MCP de Squadron sert les outils des mods. Pour qu'un agent déjà lancé les voie sans redémarrer, il notifie `tools/list_changed`.

### Vérifications faites (2026-11-06)

1. **IPC Tauri depuis un iframe isolé : bloqué.** Testé dans l'iframe des Artifacts de la release (Tauri 2.12.4).
   - `__TAURI_INTERNALS__`, `__TAURI__` et `isTauri` sont absents de l'iframe.
   - Accéder au parent (`parent.__TAURI_INTERNALS__`, `top.document`) lève une `SecurityError`.
   - `fetch` vers `ipc://` et `http://ipc.localhost` est bloqué.
   - Point d'attention : `window.webkit.messageHandlers.ipc` reste joignable depuis l'iframe (WebKitGTK l'expose à toutes les frames). Seule la clé d'invocation protège ce canal : un aléa de 128 bits injecté dans la seule fenêtre principale, vérifié par `on_message` pour toute requête IPC. Il ne faut donc jamais la transmettre à une frame, et la CSP de `mod://` n'autorise ni `ipc:` ni `http://ipc.localhost`.
2. **`tools/list_changed` : pris en compte.** Testé avec Claude Code 2.1.310 en `stream-json` et un serveur MCP HTTP. Un outil ajouté pendant la session est vu et appelé au tour suivant, que la notification passe par le flux GET ou dans la réponse POST. Sans notification, il reste invisible. Le serveur rmcp de Squadron est déjà en session stateful : il reste à déclarer `listChanged` et à garder les sessions pour les notifier.
3. **Wasmtime 50 avec WASI** : +21,5 Mo sur le binaire strippé avec les features par défaut, +16,8 Mo en version allégée (`runtime`, `component-model`, `cranelift`), pour un Squadron de 36,5 Mo. Compter 1 à 1,5 min de compilation à froid, une seule fois, puisque la dépendance reste ensuite en cache.

Taille : L au total, découpée dans l'ordre ci-dessous.

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
2. il compile le WASM (`cargo build --target wasm32-wasip2`) ;
3. il vérifie son travail avec `mod_check`, qui contrôle le manifeste, les emplacements, les actions, les permissions et l'API du module ;
4. Squadron recharge le mod à chaud, et le résultat est visible tout de suite.

### Technique et risques

- **Technique** : l'état des agents vit dans la fenêtre (Svelte). Le serveur MCP (Rust) relaie chaque appel à la fenêtre et attend sa réponse, comme pour les permissions. Rust gère directement les cartes, les réglages, le layout et les mods, qui sont des fichiers.
- **Risque principal : l'injection de prompt.** Un agent qui lit une page piégée pourrait utiliser le pilotage. Le risque reste borné : il ne peut que lancer des agents dont les demandes passent par l'utilisateur, et tout ce qui élargit les droits demande une confirmation.

Taille : M pour la lecture et l'action, S pour les outils de mods.

## Ordre proposé

1. **Socle** : registre des emplacements et des actions. Les boutons, menus et raccourcis existants y passent. Taille S. *Fait.*
2. **Sujet 1** : `layout.json`, mode Customize et Settings › Layout. Taille S à M. *Fait : outils de l'agent, menu ⋯ de l'agent, menu Squadron.*
3. **Sujet 3** : serveur MCP de pilotage (lecture, préparation et lancement, outils de mods), garde-fous, Activity. Taille M. *Fait en partie, après qu'un agent a modifié le code de Squadron pour déplacer un bouton : outils `overview`, `read_agent`, `start_agent`, `create_card`, `move_card`, `get_layout`, `set_layout`. Restent les outils de mods et le journal Activity.*
4. **Vérifications** : IPC depuis l'iframe, `tools/list_changed`, poids de Wasmtime. Taille S. *Fait.*
5. **Mods sans logique** : `mod.json`, éléments dans les emplacements, thème, prompts, Settings › Mods, installation d'un dossier local, chargement à chaud. Taille M. *Fait. Les prompts sont des éléments avec l'action `prompt.insert`. Les raccourcis clavier des mods restent à faire.*
6. **Logique WASM**, découpée en 6a (accord sur les commandes, commandes lancées, outils pour les agents : *fait*), 6b (Wasmtime, API, événements) et 6c (jetons et requêtes HTTP). *6b fait : Wasmtime 50, contrat WIT `squadron:api@1.0.0`, crate `squadron_mod_api`, mod d'exemple `hello`.* Wasmtime, API WIT et crate `squadron_mod_api`, événements, permissions, secrets, outils pour les agents. Taille L.
7. **Panneaux** : schéma `mod://`, iframe isolé, `mod-kit.css`, messages entre le panneau et le WASM. Taille M.
8. **Marketplace** : dépôt `squadron-mods`, CI de build et de publication, catalogue et installation en un clic, notification des mises à jour. Taille M.

L'API passe avant la marketplace : la marketplace n'est qu'une couche de distribution, et elle figerait une API qui n'a jamais servi. Il faut d'abord éprouver l'API v1 sur 2 ou 3 vrais mods.

## Décisions

1. **Code dans les mods** : la logique en Rust compilé en WASM, exécutée par Wasmtime, sur le modèle de Zed. Les panneaux en HTML/CSS/JS dans un iframe isolé. Pas de JavaScript de mod dans la fenêtre principale.
2. **Distribution** : un index curé (dépôt `squadron-mods`, entrée par PR relue, sha256 fixé par version), installation et désinstallation à chaud, et installation d'un dossier local pour développer.
3. **Pilotage** : uniquement par les agents de Squadron.
4. **Autonomie de l'IA** : elle prépare et lance des agents et des cartes, comme l'utilisateur l'aurait fait. Elle ne valide jamais rien et n'agit jamais à la place de l'utilisateur sur les agents. Pas d'autopilote.
5. **Personnalisation** : les deux, le mode Customize dans l'interface et l'onglet Settings › Layout.
6. **Design** : aucune liberté dans les zones de Squadron, que Squadron dessine lui-même ; liberté totale dans les panneaux du mod, dans son rectangle ; thème possible par les variables de Cuivre.
7. **Mises à jour des mods** : notifiées, installées sur un clic, avec un nouvel accord si les permissions changent.
8. **Jetons** : gardés dans le keyring et ajoutés par Squadron aux requêtes et aux commandes. Le mod ne les lit jamais.
