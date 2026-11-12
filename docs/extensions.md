# Squadron — Extensions et pilotage par IA (étude)

> Guide des auteurs d'extensions (en anglais, public) : https://github.com/Jgrasp/squadron-extensions/blob/main/docs/guide.md. Ce document-ci est l'étude et le journal des décisions.

Trois sujets :

1. remanier l'interface ;
2. y ajouter ses propres éléments ;
3. laisser une IA administrer Squadron, y compris écrire ces éléments.

## Ce qui existe déjà et servira

- **Un serveur MCP local** (rmcp, HTTP sur `127.0.0.1`, jeton par lancement), déjà branché sur chaque agent pour les permissions. Le serveur de pilotage peut tourner sur ce même serveur.
- **Le pont Rust ↔ fenêtre** en requête/réponse (permissions, fenêtre flottante du Kanban). Il permettra au serveur MCP d'agir sur l'état des agents, qui vit dans la fenêtre.
- **Un registre d'actions nommées** (`shortcuts.ts` : nouvel agent, Kanban, panneaux…). Il deviendra le vocabulaire commun des boutons, des raccourcis, des extensions et du MCP.
- **Un panneau latéral générique** (`SidePanel`), qui sert déjà au Diff, à l'éditeur et au terminal. Un panneau de extension s'y branchera de la même façon.
- **Des fichiers JSON simples** (`settings.json`, `projects.json` avec les cartes) qu'un agent peut lire et écrire.
- **Escouade**, la référence, expose un serveur MCP de pilotage sur un port fixe avec un jeton (`mcpPort`). Sa configuration ne montre rien sur des extensions.

## Ce que font Claude Code et l'app Claude, et ce qu'on en retient

On s'en inspire sans en dépendre : ces mécanismes changent d'une version à l'autre et restent peu fiables, c'est justement pour cela que Squadron existe.

- **Extensions Claude Code** (« function hooks », en accès anticipé). Exemple installé : `marco-extensions-aja-fork` (task-board, github-issues). Une extension est un module TypeScript `register(on)` qui :
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
- des outils d'extension proposés aux agents ;
- des panneaux isolés qui déclarent leurs capacités ;
- des commandes de validation et de test ;
- le rechargement à chaud ;
- l'activation et la désactivation de chaque extension.

**Ce qu'on fait autrement** :

- Squadron est l'hôte, donc l'API des extensions est la sienne : stable, versionnée (`"api": 1` dans le manifeste) et documentée par Squadron lui-même, sans dépendre des événements internes de Claude Code.
- Pas de code de extension avec tous les droits : la logique tourne en WASM, et les panneaux dans un iframe isolé. Aucun JavaScript de extension dans la fenêtre principale.
- Une erreur de extension se voit toujours (Settings › Extensions, Activity), jamais un échec silencieux.
- Aucune mise à jour automatique d'une extension : tout changement de ses permissions redemande l'accord.

## Le principe commun : des emplacements et des actions

- **Emplacement** : un endroit nommé de l'interface qui contient une liste ordonnée d'éléments.
  - Premiers emplacements : `agent.tools` (Editor, Terminal, Diff, Kanban), `menu.squadron`, `composer.actions`, `kanban.card`.
  - Plus tard : en-tête, onglets de projet, liste des agents.
- **Élément** : un bouton, une entrée de menu, un texte ou un panneau, avec un identifiant stable. Il est intégré à Squadron (`editor`, `terminal`…) ou apporté par une extension (`lando.logs`).
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

## Sujet 2 — Ajouter ses éléments : les extensions

Une extension est un dossier qui contient un manifeste `extension.json`, ses fichiers et, s'il a de la logique, un module WASM. On peut le développer en local, l'installer depuis la marketplace et le désinstaller, toujours sans redémarrer Squadron.

### Ce qu'une extension peut ajouter

Le but est d'ouvrir le plus de zones possible. Chaque zone de l'interface devient un emplacement, et chaque emplacement accepte des éléments de extension.

- **Éléments dans les zones de Squadron** : boutons (outils de l'agent), entrées de menu (menu ⋯ de l'agent, menu Squadron), lignes du panneau d'info, badges, boutons des cartes Kanban, onglet de Settings, raccourcis clavier.
- **Surfaces propres à l'extension** : panneau latéral (`SidePanel`), panneau d'info sous l'en-tête, carte dans la conversation.
- **Comportements** :
  - réactions aux événements : agent qui a fini, qui attend, qui a appelé un outil, carte déplacée ;
  - actions de Squadron, celles du registre ;
  - modèles de prompt ;
  - outils proposés aux agents, servis par le serveur MCP de Squadron.
- **Thème** : surcharge des variables de couleur de Cuivre.

Les identifiants des éléments sont préfixés par l'extension (`lando.logs`). Ils ne peuvent donc jamais entrer en collision avec un élément intégré, et ils s'ajoutent à `layout.json` comme les autres : `arrange()` ajoute déjà les éléments nouveaux et retire ceux qui ont disparu.

### Design

- **Dans les zones de Squadron, aucune liberté.** L'extension fournit un libellé `en` et `fr`, une icône, un état et l'action du clic, et Squadron dessine l'élément en Cuivre. L'interface reste cohérente, et une refonte de Squadron ne casse aucune extension.
- **Dans ses propres panneaux, liberté totale.** Une page HTML/CSS/JS au choix, dans un iframe isolé, servie par un schéma `extension://<id>/` sur le modèle du schéma `artifact:`. Squadron fournit une `extension-kit.css` (variables Cuivre, boutons, champs) et pousse les variables du thème quand elles changent. Son usage est facultatif.
- **Jamais hors de son rectangle** : une extension n'ajoute pas de popup par-dessus l'application et n'injecte pas de CSS dans Squadron.

### Logique : Rust compilé en WASM

- Le code tourne côté Rust dans Wasmtime, une instance par extension, avec des plafonds de mémoire et de temps de calcul. Il continue de tourner quand ses panneaux sont fermés : réactions aux événements, état, outils.
- L'API est décrite en WIT et publiée en crate `squadron_extension_api`. Elle est versionnée (`"api": 1` dans le manifeste).
- Le panneau d'une extension (JS dans l'iframe) parle à son WASM par messages relayés par Squadron. Il n'accède jamais à l'IPC Tauri.
- Une extension sans logique (boutons vers des actions, thème, prompts) n'a pas de WASM.
- Une panique, un dépassement de plafond ou une erreur de manifeste désactive l'extension et s'affiche dans Settings › Extensions, sans effet sur Squadron.

### Permissions

Elles sont déclarées dans le manifeste, montrées à l'installation et acceptées une par une. Une mise à jour qui les change redemande l'accord.

- `exec` : les programmes que le WASM peut lancer, avec n'importe quels arguments, sans shell.
- `network` : les noms d'hôte que `host::fetch` peut joindre, en HTTPS.
- `agents` : `read` donne les agents ouverts et ce qu'ils font (`host::agents`), `archive` permet d'en archiver (`host::archive-agent`). *Fait en 1.1.0.*
- `agents:send`, `cards:write`, `project:read` : prévus, pour les autres données et actions de Squadron.
- `tools` : outils proposés aux agents.

Le sandbox ne protège que ce que l'extension ne peut pas faire. Une commande autorisée tourne avec tous les droits de l'utilisateur. Avec une marketplace, la vraie barrière est l'écran de permissions, et il doit le dire clairement.

### Services externes et jetons

- L'extension déclare ses secrets. Ils vont dans le keyring.
- Les requêtes HTTP partent de Rust, seulement vers les domaines autorisés, et Squadron y ajoute lui-même le jeton. Les outils à `command` fixe reçoivent le jeton en variable d'environnement.
- Le WASM et le panneau ne lisent jamais un secret. Détail en étape 6c.

### Exemple de manifeste

Voir `extensions/examples/hello/extension.json`, et la référence complète dans le guide des auteurs.

### Ce que fait l'API 1 aujourd'hui (étape 5)

- **Où** : `<config>/extensions/<dossier>/extension.json`, ou un dossier ajouté depuis Settings › Extensions (« Ajouter un dossier… »). Les dossiers ajoutés et les extensions désactivées sont dans `<config>/extensions.json`. Un changement de `extension.json` se recharge en moins d'une seconde.
- **Manifeste** : `id` (minuscules, chiffres, tirets), `version`, `api: 1`, `name` et `description` en `{ "en": "…", "fr": "…" }`, l'anglais étant obligatoire.
- **`contributes.items`** : `slot` parmi `agent.tools`, `agent.menu` et `menu.squadron`, `id`, `label`, `hint` facultatif, `icon` facultative, prise dans les icônes de Squadron, et une `action` parmi :
  - `{ "terminal.run": "lando logs -f" }` : tape la commande dans le terminal de l'agent sans la lancer, l'utilisateur appuie sur Entrée. Une seule ligne ;
  - `{ "prompt.insert": "…" }` : ajoute le texte à la zone de saisie de l'agent ;
  - `{ "url.open": "https://…" }` : ouvre le lien dans le navigateur ;
  - `{ "squadron": "kanban" }` : une action du registre (`newAgent`, `kanban`, `editor`, `terminal`, `diff`…).
  `terminal.run` et `prompt.insert` demandent un agent, donc pas de `menu.squadron`.
- **Spotlight** : chaque élément, quel que soit son `slot`, sort aussi dans le spotlight sous son libellé, avec le nom de l'extension en étiquette. `words` facultatif ajoute des mots qui le trouvent (`["sentry", "errors"]`), `"spotlight": false` l'en retire. Un élément qui demande un agent (`terminal.run`, `prompt.insert`, `panel.open`) vise l'agent affiché, ou celui de la puce agent ; sans agent, il n'est pas proposé.
- **`contributes.theme`** : des variables de Cuivre et des couleurs, par exemple `{ "accent": "#4fa3e0" }`.
- **Clés des éléments** : `<extension>.<élément>`, par exemple `lando.logs`. Les éléments se rangent, se masquent et se pilotent par MCP (`get_layout`, `set_layout`) comme les éléments intégrés. Dans le menu de l'agent, ils s'insèrent avant Archiver.
- **Erreurs** : chaque problème du manifeste s'affiche, traduit, sous l'extension dans Settings › Extensions. Une extension en erreur n'ajoute rien.
- **`contributes.tools`** (étape 6a) : `[{ "name": "logs", "description": "…", "command": "lando logs --tail 200" }]`. Les agents voient l'outil `<extension>_<nom>` sur le serveur MCP `squadron-extensions`, présent chez chaque agent. La commande tourne dans le dossier de l'agent, avec 120 s au plus, et l'agent reçoit les 30 000 derniers caractères de sa sortie. Claude Code demande la permission à chaque appel, comme pour tout outil. Un outil ajouté, retiré ou autorisé apparaît chez les agents déjà lancés, sans redémarrage : notification `tools/list_changed`, envoyée par `subscriptions/listen` depuis le protocole MCP 2026-07-28, et par la session pour les clients plus anciens.
- **`contributes.args`** : `["--chrome"]`, des options ajoutées à la ligne de commande de Claude Code pour chaque agent, après accord (« Lance Claude Code avec --chrome »). Une option avec sa valeur s'écrit en deux éléments, `["--add-dir", "/chemin"]`. Les options que Squadron règle lui-même (`--model`, `--permission-mode`, `--resume`, `--mcp-config`, `--dangerously-skip-permissions`…) sont refusées. Elles valent pour les agents lancés ou relancés après l'accord, pas pour ceux qui tournent déjà.
- **Accord sur les commandes** (étape 6a) : dès qu'une extension arrive avec des commandes pas encore autorisées (`terminal.run` et outils), une popup les liste avec Autoriser ou Plus tard. « Plus tard » vaut pour la session : un clic sur un bouton de l'extension rouvre la popup, la demande reste dans Settings › Extensions, et elle revient au prochain lancement. Avant l'accord, les outils restent cachés aux agents. Après, le bouton lance la commande dans le terminal visible et les outils sont proposés. Une commande nouvelle ou modifiée redemande l'accord (`approved` dans `extensions.json`).

### Logique en WASM (étape 6b)

Une extension écrit sa logique en Rust avec le crate `squadron_extension_api` (`extensions/api`), compilé en composant WebAssembly. L'exemple complet est `extensions/examples/hello`.

```rust
use squadron_extension_api::{Agent, Extension, host, squadron_extension};

struct Hello;

impl Extension for Hello {
    fn run(action: String, agent: Option<Agent>) -> Result<(), String> {
        host::notify("Hello", &format!("{action} clicked"));
        Ok(())
    }
}

squadron_extension!(Hello);
```

1. `cargo build --release --target wasm32-wasip2` (la cible s'ajoute avec `rustup target add wasm32-wasip2`).
2. Dans `extension.json`, `"wasm": "target/wasm32-wasip2/release/<nom>.wasm"`, un chemin qui reste dans le dossier de l'extension.
3. Ajouter le dossier depuis Settings › Extensions. Chaque build recharge l'extension.

Ce que le manifeste ajoute :

- l'action `{ "run": "<action>" }` pour un élément : Squadron appelle `run` ;
- un outil sans `command`, avec un `input` facultatif (schéma JSON de type object) : Squadron appelle `call-tool` avec l'entrée en JSON ;
- `"permissions": { "exec": ["git"] }` : les programmes que `host::exec` peut lancer, avec n'importe quels arguments, sans shell. La popup d'accord les présente ainsi (« git …, avec n'importe quels arguments »).
- `"permissions": { "agents": ["read", "archive"] }` : voir les agents ouverts, en archiver. La popup d'accord les présente ainsi (« Voit vos agents et ce qu'ils font », « Archive vos agents qui ne travaillent pas »).

Comment ça tourne :

- une instance par extension, sur son propre fil ;
- 5 milliards d'unités de carburant par appel et 64 Mo de mémoire au plus ;
- WASI sans fichiers, sans réseau et sans variables d'environnement ;
- l'état de `state-set` va dans `<config>/extensions-state/<id>.json`, 1 Mo au plus ;
- une extension qui plante est signalée dans Settings › Extensions et repart sur une instance neuve, en gardant son état ;
- les événements `agent.done` et `agent.waiting` partent quand l'état d'un agent change ;
- `host::agents` et `host::archive-agent` interrogent la fenêtre principale, qui répond par `extensions_wasm_answer` en 10 s au plus. L'archivage suit la règle de l'archivage automatique (`archivable` dans `agents.svelte.ts`) : agent qui ne travaille pas, sans question en attente, ni épinglé, ni distant, ni affiché ;
- `host::set-timer` envoie l'événement `timer` après 10 s au moins, depuis le fil de l'extension, sans fil en plus. Un nouvel appel remplace le minuteur en cours. Il survit à un plantage, pas au rechargement ni au redémarrage : l'extension le réarme sur `started`.

Le contrat (`extensions/api/wit/squadron.wit`), en 1.1.0. On peut lui ajouter des imports sans casser les extensions déjà compilées : une extension compilée contre 1.0.0 se charge dans un Squadron 1.1.0, vérifié avec l'exemple `hello`. Une extension qui appelle une fonction 1.1.0 ne se charge pas dans un Squadron plus ancien. Ajouter des exports, en revanche, les casserait : les événements passent donc par un nom et du JSON.

```wit
package squadron:api@1.1.0;

interface types {
  /// The agent a call is about.
  record agent {
    id: string,
    name: string,
    project: string,
    /// The folder the agent works in: its worktree, or its project.
    dir: string,
  }

  /// An agent open in Squadron, with what it is doing. Since 1.1.0.
  record agent-state {
    agent: agent,
    /// `working`, `waiting`, `done`, `interrupted` or `error`.
    status: string,
    /// When the status last changed, in milliseconds since 1970.
    since: u64,
    pinned: bool,
    remote: bool,
    /// Open on screen.
    shown: bool,
    /// The branch of its worktree, when it has one.
    branch: option<string>,
    /// `archive-agent` would archive it now.
    archivable: bool,
  }

  record output {
    /// -1 when the program was stopped or could not report one.
    code: s32,
    stdout: string,
    stderr: string,
  }

  record request {
    method: string,
    url: string,
    /// `{{name}}` in a value is replaced by the extension's secret `name`, which the extension never reads.
    headers: list<tuple<string, string>>,
    body: option<string>,
  }

  record response {
    status: u16,
    headers: list<tuple<string, string>>,
    body: string,
  }
}

interface host {
  use types.{agent-state, output, request, response};

  /// Writes a line to Squadron's log.
  log: func(message: string);
  /// Shows a notification.
  notify: func(title: string, body: string);
  /// Adds text to the message the user is writing to an agent.
  insert-prompt: func(agent: string, text: string);
  /// Shows a short badge on one of the extension's items, or removes it.
  set-badge: func(item: string, badge: option<string>);
  /// Replaces the label of one of the extension's items, or restores the manifest's.
  set-label: func(item: string, label: option<string>);
  /// The extension's own storage, kept across restarts.
  state-get: func(key: string) -> option<string>;
  state-set: func(key: string, value: option<string>);
  /// Runs a program the user allowed in `permissions.exec`, without a shell, for at most 120 s.
  exec: func(program: string, args: list<string>, dir: option<string>) -> result<output, string>;
  /// Sends an HTTPS request to a host the user allowed in `permissions.network`, for at most 30 s and 8 MB.
  fetch: func(request: request) -> result<response, string>;
  /// Sends a JSON message to one of the extension's open panels, which receives it in `squadron.on`.
  panel-post: func(panel: string, message: string);
  /// The agents open in Squadron, archived ones aside. Needs `agents: ["read"]` in `permissions`. Since 1.1.0.
  agents: func() -> result<list<agent-state>, string>;
  /// Archives an agent, as the user would. Refused while it works, waits for the user, is pinned, remote or shown. Needs `agents: ["archive"]` in `permissions`. Since 1.1.0.
  archive-agent: func(agent: string) -> result<_, string>;
  /// Sends the event `timer` in `seconds`, 10 at least, and replaces the one pending. Since 1.1.0.
  set-timer: func(seconds: u32);
}

world squadron-extension {
  use types.{agent};
  import host;

  /// Something happened: `started` once the extension is loaded, `agent.done` when an agent finished its turn, `agent.waiting` when it waits for the user, `panel.message` when one of its panels called `squadron.send`, with `{ "panel": "…", "message": … }`, `timer` when the delay of `set-timer` is over. `data` is JSON.
  export on-event: func(name: string, agent: option<agent>, data: string);
  /// One of the extension's items with the action `{ "run": "<action>" }` was clicked.
  export run: func(action: string, agent: option<agent>) -> result<_, string>;
  /// An agent called one of the extension's tools that has no `command`. `input` is JSON.
  export call-tool: func(name: string, input: string, agent: option<agent>) -> result<string, string>;
}
```

### Jetons et requêtes HTTP (étape 6c)

```json
"contributes": { "secrets": [{ "id": "token", "label": { "en": "GitHub token", "fr": "Jeton GitHub" }, "hint": { "en": "…" } }] },
"permissions": { "network": ["api.github.com"] }
```

- **Secrets** : chaque secret a son champ dans Settings › Extensions, sous l'extension. La valeur va dans le keyring du système (`extension.<id>.<secret>`). Squadron ne la renvoie jamais, ni à la fenêtre, ni à l'extension : il dit seulement si elle est renseignée.
- **Domaines** : `permissions.network` liste des noms d'hôte exacts. Ils passent par la même popup d'accord que les commandes (« Requêtes HTTPS vers api.github.com »). Un domaine nouveau redemande l'accord.
- **`host::fetch`** : HTTPS seulement, vers un hôte autorisé, 30 s et 8 Mo au plus. Une redirection vers un hôte non autorisé n'est pas suivie : l'extension reçoit la réponse 3xx.
- **Jeton dans une requête** : `{{token}}` dans la valeur d'un en-tête, par exemple `("Authorization", "Bearer {{token}}")`. Squadron le remplace par le secret, et le remplace par `[secret]` s'il revient dans la réponse. Seulement dans les en-têtes, pas dans l'URL ni le corps.
- **Jeton dans une commande** : un outil avec `command` reçoit chaque secret renseigné en variable `SQUADRON_<SECRET>`, par exemple `SQUADRON_TOKEN`. La commande est fixe et l'utilisateur l'a lue dans la popup.
- **Pas de jeton pour `host::exec`** : il autorise un programme avec n'importe quels arguments (`git -c alias.x='!env' x` lance un shell), donc le WASM pourrait relire la variable. Une extension en WASM passe par `host::fetch`.
- **Pas de jeton pour `terminal.run`** : la commande est tapée dans le terminal de l'agent, déjà lancé.

L'exemple `hello` le montre avec l'outil `github_user`, qui appelle `https://api.github.com/user` avec le jeton.

### Panneaux (étape 7)

```json
"contributes": {
  "panels": [{ "id": "main", "title": { "en": "Hello", "fr": "Hello" }, "page": "panel/index.html" }],
  "items": [{ "slot": "agent.tools", "id": "panel", "label": { "en": "Hello panel" }, "action": { "panel.open": "main" } }]
}
```

- **Ouverture** : l'action `panel.open` ouvre ou ferme le panneau à côté de la conversation, comme le terminal ou le diff. Elle demande un agent, donc pas de `menu.squadron`. Le panneau garde les modes et tailles des autres panneaux.
- **Page** : servie par le schéma `extension://localhost/<id>/<page>` (`http://extension.localhost/…` sous Windows), seulement pour une extension activée et seulement depuis son dossier. HTML, CSS, JS, images et polices au choix.
- **Isolement** : iframe `sandbox="allow-scripts"`, d'origine opaque. Pas d'IPC Tauri, pas d'accès à la fenêtre de Squadron, pas de popup (`alert`, `confirm`), pas de `localStorage`. La CSP interdit tout accès réseau (`connect-src 'none'`) : les requêtes passent par le WASM et `host::fetch`.
- **Pont** : Squadron ajoute `/_squadron/kit.js` en tête de chaque page HTML.
  - `squadron.send(valeur)` : le WASM de l'extension reçoit l'événement `panel.message` avec `{ "panel": "<id>", "message": <valeur> }`.
  - `squadron.on(fonction)` : reçoit ce que le WASM envoie par `host::panel_post("<id>", "<json>")`.
  - Un message envoyé avant le chargement de la page se perd : la page envoie d'abord (`ready`), le WASM répond.
- **Thème** : Squadron pousse dans la page les variables `--bg-main`, `--surface-1` à `--surface-3`, `--border`, `--border-focus`, `--text`, `--text-strong`, `--text-muted`, `--text-dim`, `--ink`, `--accent`, `--accent-link`, `--ok-text`, `--err-text`, `--sans` et `--mono`, au chargement et à chaque changement de thème. C'est la liste garantie par l'API. `/_squadron/kit.css`, facultative, s'en sert pour le fond, le texte, les boutons (`button`, `button.primary`) et les champs.
- **Limite** : modifier la page ne recharge pas le panneau. Il faut le fermer et le rouvrir.

L'exemple `hello` a un panneau qui affiche le compteur et l'augmente par un bouton, à travers son WASM.

### Distribution (étape 8)

Sur le modèle de Zed : chaque extension vit dans son dépôt, un dépôt central la recense, et c'est la CI de ce dépôt qui la compile.

- **Dépôt `Jgrasp/squadron-extensions`** :
  - `registry.json` : une entrée par extension, `{ "repository", "commit", "path", "version" }`. On publie ou on met à jour par une PR relue, une extension par PR. Changer le commit sans changer la version fait échouer la PR ;
  - `api/` : le crate `squadron_extension_api` et son WIT, recopiés depuis `extensions/api` de Squadron, qui reste la référence ;
  - `examples/hello` : l'extension d'exemple, publiée depuis ce dépôt ;
  - `index.json` : écrit par la CI, lu par Squadron.
- **CI** (`.github/workflows/registry.yml`) :
  - `plan` vérifie les entrées et liste celles qui n'ont pas encore de release `<id>-<version>` ;
  - `build` récupère chaque extension à son commit, vérifie que `extension.json` a le même id et la même version, compile le WASM (`cargo build --locked`) et empaquette `extension.json`, le WASM et les fichiers hors Rust. Ce job lance le code de l'extension (`build.rs`, macros), il n'a donc qu'un droit de lecture ;
  - `publish`, seulement après fusion sur `main`, ne lance aucun code d'extension : sha256 de l'archive, release `<id>-<version>`, puis `index.json` mis à jour et commité.
- **Catalogue dans Settings › Extensions** : Squadron lit `index.json` (depuis Rust, la CSP de la fenêtre bloque GitHub) et affiche nom, version, description et permissions demandées, avec Installer, Mettre à jour vers x ou Installée.
- **Installer** :
  1. l'archive ne peut venir que des releases de `squadron-extensions`, 50 Mo au plus ;
  2. son sha256 doit être celui de l'index ;
  3. extraction dans `extensions/.staging-<uuid>` : seulement des fichiers et des dossiers, aucun lien, aucun chemin qui sort, 2000 fichiers et 200 Mo au plus ;
  4. l'`extension.json` extrait doit porter l'id attendu ;
  5. l'ancienne version est mise de côté, la nouvelle prend sa place, l'ancienne est supprimée ;
  6. le chargement à chaud fait le reste, et la popup d'accord demande les commandes et les domaines.
- **Désinstaller** : retire ses secrets du trousseau, supprime son dossier et oublie ses accords. Un dossier local se retire, il ne se désinstalle pas.
- **Mises à jour** : vérifiées au lancement puis toutes les 6 heures. Une notification par version, puis un clic sur « Mettre à jour vers x ». Jamais d'installation automatique. Une mise à jour qui ajoute une commande ou un domaine redemande l'accord.
- **Sources** : `Jgrasp/squadron-extensions` est la source officielle, toujours présente. L'utilisateur peut ajouter d'autres dépôts GitHub `propriétaire/dépôt` organisés de la même façon (un fork du dépôt officiel suffit) ; Squadron vérifie à l'ajout qu'ils publient un `index.json`. Leurs extensions sont marquées « Non officielle ».
  - Une archive ne vient que des releases du dépôt qui l'a listée.
  - Si deux sources publient le même id, l'officielle est proposée d'abord.
  - Squadron retient la source de chaque extension installée (`origins` dans `extensions.json`) et ne propose ses mises à jour que depuis cette source : une autre source ne peut pas la remplacer.
- **Développer** : « Ajouter un dossier… » charge une extension locale, rechargée à chaque build. Un dossier local passe avant la version installée qui a le même id.
- **Limite** : une extension compilée avec un SDK plus récent que le Squadron installé ne se charge pas.

### Outils des extensions chez les agents

Le serveur MCP de Squadron sert les outils des extensions. Pour qu'un agent déjà lancé les voie sans redémarrer, il notifie `tools/list_changed`.

### Vérifications faites (2026-11-06)

1. **IPC Tauri depuis un iframe isolé : bloqué.** Testé dans l'iframe des Artifacts de la release (Tauri 2.12.4).
   - `__TAURI_INTERNALS__`, `__TAURI__` et `isTauri` sont absents de l'iframe.
   - Accéder au parent (`parent.__TAURI_INTERNALS__`, `top.document`) lève une `SecurityError`.
   - `fetch` vers `ipc://` et `http://ipc.localhost` est bloqué.
   - Point d'attention : `window.webkit.messageHandlers.ipc` reste joignable depuis l'iframe (WebKitGTK l'expose à toutes les frames). Seule la clé d'invocation protège ce canal : un aléa de 128 bits injecté dans la seule fenêtre principale, vérifié par `on_message` pour toute requête IPC. Il ne faut donc jamais la transmettre à une frame, et la CSP de `extension://` n'autorise ni `ipc:` ni `http://ipc.localhost`.
2. **`tools/list_changed` : pris en compte.** Testé avec Claude Code 2.1.310 en `stream-json` et un serveur MCP HTTP. Un outil ajouté pendant la session est vu et appelé au tour suivant, que la notification passe par le flux GET ou dans la réponse POST. Sans notification, il reste invisible. Le serveur rmcp de Squadron est déjà en session stateful : il reste à déclarer `listChanged` et à garder les sessions pour les notifier.
3. **Wasmtime 50 avec WASI** : +21,5 Mo sur le binaire strippé avec les features par défaut, +16,8 Mo en version allégée (`runtime`, `component-model`, `cranelift`), pour un Squadron de 36,5 Mo. Compter 1 à 1,5 min de compilation à froid, une seule fois, puisque la dépendance reste ensuite en cache.

Taille : L au total, découpée dans l'ordre ci-dessous.

## Sujet 3 — Piloter Squadron par une IA

### Le serveur MCP « squadron »

Il expose trois familles d'outils.

- **Lire** :
  - projets, agents (état, coût, ce qu'ils attendent) ;
  - conversation d'un agent (derniers messages), son diff ;
  - cartes Kanban, réglages, layout et extensions.
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
  - écrire une extension, le vérifier, le recharger ;
  - modifier le layout.

### Qui s'y connecte

Uniquement les agents lancés par Squadron, sur option (par exemple un agent « Admin »). Le serveur est ajouté à leur `--mcp-config`, et l'en-tête `x-squadron-agent` identifie l'agent qui agit. Il n'y a pas d'accès pour les clients externes.

### Garde-fous

- **Périmètre** : par défaut, un agent ne lit et ne lance que dans son propre projet. Le pilotage de tous les projets est une option.
- **Confirmation dans l'interface**, avec la même carte que les permissions, pour :
  - lancer un agent dans un mode qui saute des permissions (`auto`, `acceptEdits`) ;
  - activer une extension qui lance des commandes.
- **Pas d'autopilote** : aucun outil ne valide quoi que ce soit à la place de l'utilisateur.
- **Journal « Activity »** : tout ce que l'IA a fait, et par qui.
- **Plafonds** : nombre d'agents créés, coût du jour.

### Coder une extension à la place de l'utilisateur

Le dossier des extensions peut s'ajouter comme un projet Squadron. Un agent l'édite avec ses outils habituels :

1. il lit la documentation fournie par l'outil MCP `extensions_docs` ;
2. il compile le WASM (`cargo build --target wasm32-wasip2`) ;
3. il vérifie son travail avec `extension_check`, qui contrôle le manifeste, les emplacements, les actions, les permissions et l'API du module ;
4. Squadron recharge l'extension à chaud, et le résultat est visible tout de suite.

### Technique et risques

- **Technique** : l'état des agents vit dans la fenêtre (Svelte). Le serveur MCP (Rust) relaie chaque appel à la fenêtre et attend sa réponse, comme pour les permissions. Rust gère directement les cartes, les réglages, le layout et les extensions, qui sont des fichiers.
- **Risque principal : l'injection de prompt.** Un agent qui lit une page piégée pourrait utiliser le pilotage. Le risque reste borné : il ne peut que lancer des agents dont les demandes passent par l'utilisateur, et tout ce qui élargit les droits demande une confirmation.

Taille : M pour la lecture et l'action, S pour les outils de extensions.

## Ordre proposé

1. **Socle** : registre des emplacements et des actions. Les boutons, menus et raccourcis existants y passent. Taille S. *Fait.*
2. **Sujet 1** : `layout.json`, mode Customize et Settings › Layout. Taille S à M. *Fait : outils de l'agent, menu ⋯ de l'agent, menu Squadron.*
3. **Sujet 3** : serveur MCP de pilotage (lecture, préparation et lancement, outils de extensions), garde-fous, Activity. Taille M. *Fait en partie, après qu'un agent a modifié le code de Squadron pour déplacer un bouton : outils `overview`, `read_agent`, `start_agent`, `create_card`, `move_card`, `get_layout`, `set_layout`. Restent les outils de extensions et le journal Activity.*
4. **Vérifications** : IPC depuis l'iframe, `tools/list_changed`, poids de Wasmtime. Taille S. *Fait.*
5. **Extensions sans logique** : `extension.json`, éléments dans les emplacements, thème, prompts, Settings › Extensions, installation d'un dossier local, chargement à chaud. Taille M. *Fait. Les prompts sont des éléments avec l'action `prompt.insert`. Les raccourcis clavier des extensions restent à faire.*
6. **Logique WASM**, découpée en 6a (accord sur les commandes, commandes lancées, outils pour les agents : *fait*), 6b (Wasmtime, API, événements) et 6c (jetons et requêtes HTTP). *6b fait : Wasmtime 50, contrat WIT `squadron:api@1.0.0`, crate `squadron_extension_api`, extension d'exemple `hello`. 6c fait : secrets dans le keyring, `host::fetch` vers les domaines autorisés, jeton en en-tête et en variable d'environnement des outils.* Wasmtime, API WIT et crate `squadron_extension_api`, événements, permissions, secrets, outils pour les agents. Taille L.
7. **Panneaux** : schéma `extension://`, iframe isolé, `extension-kit.css`, messages entre le panneau et le WASM. Taille M. *Fait : `contributes.panels`, action `panel.open`, `kit.js` et `kit.css`, événement `panel.message` et `host::panel_post`.*
8. **Marketplace** : dépôt `squadron-extensions`, CI de build et de publication, catalogue et installation en un clic, notification des mises à jour. Taille M. *Fait dans Squadron ; le dépôt est prêt à publier.*

L'API passe avant la marketplace : la marketplace n'est qu'une couche de distribution, et elle figerait une API qui n'a jamais servi. Il faut d'abord éprouver l'API v1 sur 2 ou 3 vrais extensions.

## Décisions

1. **Code dans les extensions** : la logique en Rust compilé en WASM, exécutée par Wasmtime, sur le modèle de Zed. Les panneaux en HTML/CSS/JS dans un iframe isolé. Pas de JavaScript de extension dans la fenêtre principale.
2. **Distribution** : un index curé (dépôt `squadron-extensions`, entrée par PR relue, sha256 fixé par version), installation et désinstallation à chaud, et installation d'un dossier local pour développer.
3. **Pilotage** : uniquement par les agents de Squadron.
4. **Autonomie de l'IA** : elle prépare et lance des agents et des cartes, comme l'utilisateur l'aurait fait. Elle ne valide jamais rien et n'agit jamais à la place de l'utilisateur sur les agents. Pas d'autopilote.
5. **Personnalisation** : les deux, le mode Customize dans l'interface et l'onglet Settings › Layout.
6. **Design** : aucune liberté dans les zones de Squadron, que Squadron dessine lui-même ; liberté totale dans les panneaux de l'extension, dans son rectangle ; thème possible par les variables de Cuivre.
7. **Mises à jour des extensions** : notifiées, installées sur un clic, avec un nouvel accord si les permissions changent.
8. **Jetons** : gardés dans le keyring et ajoutés par Squadron aux en-têtes des requêtes et aux outils à commande fixe. L'extension ne les lit jamais.
