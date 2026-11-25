# Nouveautés

## 0.64.0 · 2026-11-25

### Réglages
- Les réglages se recherchent : les lignes correspondantes de toutes les pages s’affichent dans une seule vue, groupées par page, et un index dans la barre latérale mène à chaque page.

### Agents
- Le modèle et l’effort peuvent être réglés sur Auto : Haiku les choisit d’après le premier message, et vous modifiez ses instructions dans les réglages.
- Les agents sauvegardés avant que Haiku réponde se chargent avec des options concrètes.
- Les agents démarrent sans six outils intégrés rarement utilisés ni les connecteurs claude.ai, chargés à la demande, et un agent peut attendre les checks ou la fusion d’une pull request.

### Prise en main
- Chaque étape de la visite guidée ouvre ce dont elle parle.

### Extensions
- Une extension peut passer les secrets de ses comptes à un programme qu’elle lance, sur son entrée standard, jamais dans ses arguments ni son environnement, une fois que vous l’avez autorisée : la demande d’autorisation indique quel programme reçoit des secrets. Les champs d’un compte lié à un site ne vont jamais à un programme.
- Une extension peut garder les jetons qu’un service lui renvoie, comme un jeton de renouvellement Google, dans le trousseau du système comme champ d’un de ses comptes : elle ne les revoit plus, et le formulaire du compte les indique reçus ou pas encore.
- Les secrets remplissent aussi les corps de requête encodés comme un formulaire, et sont masqués dans la réponse.
- Une extension peut ouvrir son formulaire de compte par-dessus Squadron, prérempli, pour ajouter ou modifier un compte.
- Les secrets peuvent être échappés pour JSON : un mot de passe avec guillemets, barres obliques inverses ou retours à la ligne ne casse plus le JSON qu’une extension passe à un programme.
- Une extension peut effacer un jeton qu’elle a gardé, pour vous faire vous reconnecter quand le service l’a révoqué, et savoir si un secret d’un compte est renseigné sans le lire.
- Les outils d’extension qui lancent une commande ne reçoivent plus de secrets dans leur environnement.
- Une extension peut vous connecter à un service dans votre navigateur (OAuth) : Squadron reçoit la réponse et garde le code dans le trousseau du système, hors de portée de l’extension.

## 0.63.0 · 2026-11-25

### Prise en main
- Au premier lancement, Squadron propose une visite guidée qui déroule un projet d’exemple, acme-shop, en 45 étapes. Rejouez-la, ou un seul chapitre, depuis Réglages › Squadron › Démarrage rapide.

### Extensions
- Ajoutez une extension depuis son dépôt GitHub, GitLab (auto-hébergé compris) ou Bitbucket avec Depuis un dépôt dans Extensions : Squadron installe sa dernière release et propose chaque nouvelle en mise à jour.
- Les marketplaces peuvent être sur GitHub, GitLab, Bitbucket ou à n’importe quelle URL .json.
- Les extensions s’installent depuis la release de leur propre dépôt : une nouvelle version est disponible dès sa publication.

## 0.62.0 · 2026-11-24

### Usage
- Une page Usage montre combien de tokens chaque session Claude Code de cette machine a consommé par jour, sur 7, 30 ou 90 jours, et ce qui les a consommés : sorties d’outils, réponses, votre configuration, hooks, serveurs MCP, plugins, advisor.
- Elle pointe ce qui pèse le plus, chaque fois avec une action : compacter plus tôt, voir les conversations les plus longues et les grosses sorties relues tour après tour, désactiver les plugins chargés mais jamais appelés.
- Ouvrez-la avec le bouton Usage de l’en-tête ou en cliquant les limites Session et Semaine de la barre d’état. Réglages › Préférences › Page Usage la désactive.

### Agents
- Le champ de la fenêtre Nouvel agent grandit avec son texte et s’agrandit en fenêtre pleine hauteur. Effacer demande un second clic, et Échap ferme par étapes en gardant votre brouillon.

## 0.61.0 · 2026-11-24

### Artifacts

- Les agents peuvent publier des artifacts claude.ai : présentations Slides, Docs, Sheets et les autres types de votre compte.
- Publier sur claude.ai, dans le menu Partager d’un artifact, accepte aussi les artifacts html et svg, en page claude.ai.
- Publier en présentation, nouvelle entrée du menu Partager, fait de tout artifact une présentation Slides sur claude.ai.
- Le panneau Artifacts liste ce que les agents publient sur claude.ai.
- Les artifacts html ont leur propre style et gardent le même rendu une fois publiés sur claude.ai.

## 0.60.0 · 2026-11-24

### Conversation

- Les images de la conversation se copient comme images et s’enregistrent, depuis le clic droit et la visionneuse plein écran.

### Mises à jour

- Quand tout est à jour, Mises à jour affiche un sceau animé et les extensions installées ; Gérer les extensions devient un bouton.

### Navigateur

- Les pages d’un onglet que le panneau Navigateur n’affiche pas continuent de tourner : les formulaires Turbo et les autres pages qui attendent une image ne restent plus bloqués.

## 0.59.0 · 2026-11-24

### Réglages

- Les Réglages sont réorganisés écran par écran : les options s’enregistrent dès que vous les changez, et le menu rouvre le dernier onglet visité.
- Projets › Tous les projets réunit les valeurs par défaut, les règles de permission de chaque projet, la CI et les consignes de revue ; les réglages des agents passent dans Projets › Agents.
- Interface › Disposition réunit les panneaux, le Diff et Ouvrir dans, chaque emplacement en ligne à glisser et à activer ; Ouvrir dans liste les applications trouvées sur votre machine.
- Toutes les extensions, sous Squadron, liste chaque extension avec son état et ouvre sa page.
- À propos affiche l’état des mises à jour et l’historique des versions, une ligne repliée par version, à la place de l’onglet Nouveautés.
- Claude Code affiche la valeur actuelle de chaque réglage, Serveurs MCP montre une seule liste où chaque serveur tourne dans chaque session ou à la demande, et Raccourcis liste les raccourcis fixes.
- Ouverture rapide devient un raccourci réassignable, et Spotlight trouve les règles de permission d’un projet.

### Agents

- Les agents peuvent reprendre seuls une session arrêtée sur une erreur, après 30 secondes et jusqu’à 3 fois de suite (Projets › Agents).
- Les lignes d’agent montrent les puces de chaque extension à tour de rôle ; Préférences › Liste des agents règle leur rotation.

### Kanban

- Sélectionnez des cartes par Ctrl ou Maj clic ou en traçant un cadre, et archivez-les ensemble. Archiver un agent retire sa carte du tableau.

### Pull requests

- La liste ne montre que les pull requests qui vous concernent ; un interrupteur dans Réglages › Pull requests ramène les autres du dépôt.
- La revue liste les nouvelles remarques en tête des Fils, chaque carte mène à son fil, la colonne se redimensionne, Fusionner revient et Demander des changements n’exige plus de commentaire.

### Navigateur

- Les agents règlent la largeur de la page pour tester le responsive, affichée en puce que vous pouvez réinitialiser, lisent les erreurs de console et les requêtes en échec, et capturent la page entière.
- Les onglets et leur historique reviennent après un redémarrage, et un nouvel onglet s’affiche une fois sa première page chargée.
- Entrée ajoute une note au message et continue l’annotation, la colonne des notes s’élargit, et Ajouter au message ramène à la conversation.

### Correctifs

- Le style de réponse choisi dans Réglages › Claude Code atteint désormais les agents.
- Les pages servies sur le port 5173 s’ouvrent dans le navigateur de l’app installée.

## 0.58.0 · 2026-11-21

### Projets

- Les règles de permission passent dans Réglages › Projets, dans une section repliée de la page de chaque projet, avec une liste compacte à déplier. Les Réglages n’ont plus de groupe Agents.

### Correctifs

- Coller une capture d’écran, copier du texte et ouvrir un lien fonctionnent de nouveau une fois qu’un agent a ouvert un onglet du navigateur.
- Le menu d’affichage du panneau Navigateur s’affiche au-dessus de la page au lieu de passer dessous.

## 0.57.0 · 2026-11-21

### Navigateur

- Les agents ont un navigateur intégré, en alpha et sous Linux pour l’instant : le bouton Navigateur d’un agent ouvre ses onglets dans un panneau, où vous naviguez comme dans n’importe quel navigateur et répondez aux dialogues de la page.
- Avec le stylo, cliquez un élément ou entourez une zone de la page et écrivez un commentaire par note, puis Ajouter au message : l’agent reçoit vos notes numérotées avec le détail de chaque élément et une capture annotée, et la conversation montre vos commentaires, le détail replié dessous.
- Les agents ouvrent, lisent et utilisent leurs onglets avec des outils qui dépensent peu de tokens : ils remplissent des formulaires, cliquent, envoient des fichiers, lisent la page et font des captures, sans jamais voir un mot de passe. Le réglage « Les agents peuvent utiliser le navigateur intégré » le désactive.

### Pull requests

- Les pull requests sont regroupées selon qui doit agir, chacune qui peut être fusionnée affiche Fusionner, et l’onglet Relectures est toujours là.
- Une pull request peut être ignorée, pour de bon ou jusqu’à ses prochains commits.
- Une relecture s’ouvre dans le panneau des pull requests, élargi à 1120px, avec une flèche de retour vers la liste. Les fils de discussion sont dans une colonne repliable à côté du diff, et un commentaire en brouillon s’affiche en Markdown et se modifie sur place.
- Ctrl+Entrée ajoute un commentaire de ligne ou de pull request à la relecture ; le publier tout de suite ou l’envoyer à un agent passe dans un menu. La barre de relecture garde Demander à un agent, Abandonner et Terminer la relecture, où vous choisissez Approuver ou Demander des modifications.
- La fusion utilise la méthode de fusion choisie dans les Réglages, et la forge supprime ou non la branche distante selon ses propres réglages.
- La branche poussée d’un agent sans pull request a un bouton Ouvrir une pull request, avec un titre et une description que Haiku rédige à partir des commits.
- Un projet peut faire relire par un agent chaque pull request dont on vous demande la relecture, et ses nouveaux commits. Désactivé par défaut, dans Réglages › Projets.
- L’agent qui corrige une relecture de votre pull request y pousse ses commits, jamais en force, et rebase quand la branche a bougé.
- Un agent qui attend seulement la fusion de sa pull request s’affiche comme en attente de vous, et Squadron vérifie cette pull request toutes les 20 secondes. Son projet affiche l’anneau d’attente et compte dans la Vue d’ensemble.
- Les merge requests GitLab et les pull requests et pipelines Azure DevOps viennent désormais des extensions GitLab et Azure DevOps, à installer depuis Découvrir : chaque forge est désormais une extension.

### Projets

- Réglages › Projets montre un projet sur une seule page. Son entrée Tous les projets réunit les réglages par défaut des nouveaux agents, conseiller compris, s’enregistre aussitôt et liste les projets qui ont leurs propres valeurs.
- Retirer un projet le sort de Squadron au lieu de l’archiver, et propose de supprimer ses agents, archivés compris. Les Réglages n’ont plus d’écran Agents archivés, et les projets archivés auparavant reviennent dans la liste.

### Agents

- Les questions et demandes de permission d’un agent en contrôle à distance arrivent sur claude.ai, où vous pouvez y répondre.
- Le premier message d’un nouvel agent prend le focus une fois que Haiku l’a rempli, pas pendant qu’il écrit.

### Extensions

- API d’extension 1.13.0 : merge prend une méthode et le choix de supprimer la branche, et une pull request indique ses méthodes de fusion permises, la suppression de branche et sa protection.
- API d’extension 1.14.0 : une forge peut ouvrir une pull request avec l’appel create.
- Un champ de compte ou un secret qui renvoie vers une page affiche un bouton Ouvrir, dans un bloc avec ses valeurs à copier.

### Correctifs

- L’aperçu d’un bloc markdown et un fichier ouvert dans sa propre fenêtre grandissent avec la fenêtre au lieu de s’arrêter à 520px.
- Un agent arrêté pendant qu’il posait une question ne s’affiche plus comme attendant une réponse après le redémarrage de Squadron.
- Une réponse diffusée sans son identifiant de message, comme avec le contrôle à distance, s’affiche une fois au lieu de deux.
- Fermer Squadron réduit sous Windows ne le rouvre plus hors de l’écran.
- Squadron ne reste plus agrandi quand la taille de fenêtre enregistrée dépasse l’écran.

## 0.56.0 · 2026-11-20

### Extensions

- API d’extension 1.12.0 : une extension peut envoyer plusieurs requêtes web à la fois, jusqu’à 8 en même temps, pour qu’une extension de forge charge une longue liste de pull requests dans le temps qui lui est imparti.

## 0.55.0 · 2026-11-19

### Agents

- Un agent lancé depuis le brouillon de nouvel agent d’une extension, comme un ticket Jira, garde son lien : le logo de l’extension et la clé s’affichent dans sa ligne de la liste des agents, avec le titre au survol, et un clic ouvre la page.

## 0.54.0 · 2026-11-19

### Pull requests

- Les pull requests Bitbucket et les exécutions Pipelines viennent désormais de l’extension Bitbucket, à installer depuis Découvrir.
- Le bouton Pipelines de l’en-tête d’agent affiche un seul compteur, et son panneau montre une carte par exécution, les vérifications de pull request à part des pipelines.
- Une fois une pull request fusionnée, Squadron supprime le worktree et la branche de son agent dès que l’agent dort, sans le réveiller, et supprime la branche sur origin.
- La liste des pull requests affiche un anneau de chargement pendant que Squadron interroge la forge, et l’actualiser vérifie aussi quelle forge couvre le projet.
- Les cartes de pull request n’attendent plus l’actualisation en arrière-plan des pull requests et des exécutions.

### Projets

- Squadron rouvre le projet sur lequel vous étiez à sa fermeture.
- Les projets cachés derrière l’onglet +N pulsent dans son menu et sur le bouton pendant que leurs agents travaillent.

### Conversation

- Les cartes de fichier ont des boutons Ouvrir et Dossier, Dossier montrant le fichier dans son dossier.
- Une commande shell collée avec un ! en tête après le ! de la zone de saisie s’exécute sans ce ! de trop, de même qu’une commande lancée depuis un bloc.
- Rouvrir la fenêtre de nouvel agent avec un brouillon place le curseur à la fin du texte.

### Extensions

- API d’extension 1.10.0 et 1.11.0 : un champ de compte peut lister des valeurs à copier en un clic, comme les droits qu’un jeton demande, et une exécution de forge peut nommer la pull request qu’elle vérifie.
- Deux extensions de même id ont chacune leur page de réglages, et désinstaller la copie installée garde les comptes et les choix du dossier local.

### Correctifs

- Squadron ne reste plus sur un écran noir quand l’onglet +N affiche des projets au travail.
- La fenêtre de Squadron ne se rouvre plus à la taille de tout l’écran : quitter le plein écran lui rend sa propre taille.
- Sous Linux, un texte copié pendant que la fenêtre de Squadron n’a pas le focus arrive dans le presse-papiers dès qu’elle le reprend.

## 0.53.4 · 2026-11-18

### Pull requests

- Relire depuis une carte de pull request ou depuis la liste des pull requests montre la pull request telle qu’elle est maintenant.
- Une relecture ouverte de votre pull request met à jour ses vérifications et son bouton de fusion dès qu’ils changent.

### Extensions

- Les extensions reçoivent toute la sortie des programmes qu’elles lancent, jusqu’à 8 Mo.

### Correctifs

- Les réglages s’ouvrent de nouveau quand une extension est à la fois installée et liée depuis un dossier.
- Les réglages et les autres fenêtres de dialogue s’ouvrent de nouveau après que la fenêtre les a fermés d’elle-même.

## 0.53.3 · 2026-11-18

### Extensions

- Les extensions pas encore installées affichent leur logo dans le catalogue, et non plus seulement la première lettre de leur nom.

## 0.53.2 · 2026-11-18

### Correctifs

- Les Nouveautés rangent de nouveau les changements de la 0.53.0 sous la 0.53.0, et la 0.53.1 comme un correctif de construction.

## 0.53.1 · 2026-11-18

### Correctifs

- Aucun changement visible : cette version corrige la construction de Squadron.

## 0.53.0 · 2026-11-18

### Pull requests

- Une pull request relue s’ouvre sur ses fils : vos tours de relecture à gauche, et à droite chaque fil selon son état (encore ouvert, répondu, corrigé, résolu, obsolète) avec ce qu’en a trouvé l’agent relecteur, vos brouillons à part.
- Quand l’auteur pousse de nouveaux commits, « Faire relire par l’agent » envoie à l’agent relecteur les nouveaux commits et les fils encore ouverts, et il dit pour chacun s’il est corrigé, répondu ou encore ouvert.
- Terminer un tour résout les fils que vous cochez, sur Bitbucket, GitLab, Azure DevOps et GitHub. Ceux que l’agent a jugés corrigés sont déjà cochés, et chaque fil peut aussi être résolu ou rouvert seul.
- Chaque tour de relecture garde ses brouillons et se publie sur le commit relu : les commits poussés entre-temps ne comptent plus comme relus.
- Les pull requests et les runs Actions de GitHub viennent maintenant de l’extension GitHub, à installer depuis « Découvrir des extensions ». Réglages › Pull requests liste chaque forge avec ses comptes et propose l’extension dont un projet a besoin, comme la vue des pull requests d’un projet qu’aucune forge ne couvre.
- Les pull requests et les liens d’une extension de forge s’affichent avec son nom et son logo, et seulement les actions de relecture qu’elle gère.

### Agents

- Les agents de relecture ont leur propre onglet Relectures dans la liste des agents, groupés en À suivre, En cours et Terminées. Une pastille cuivre sur un onglet compte ses agents qui vous attendent, et un point cuivre signale un agent qui a fini et que vous n’avez pas ouvert.

### Conversation

- Les fichiers CSV et TSV s’affichent en tableau dans les cartes de fichier et dans leur fenêtre, avec un filtre de lignes, le tri, des colonnes redimensionnables et des en-têtes fixes, aussi hors du projet et joints à un message.
- Les cartes de fichier ont un bouton « Afficher dans le dossier ».
- Le panneau de contexte compte les images des résultats d’outils, comme les captures du navigateur, à leur coût réel.
- Afficher tous les plugins dans le panneau de contexte ne le ferme plus.

### Nouvel agent

- La fenêtre Nouvel agent garde son brouillon par projet quand vous changez de projet ou la fermez, avec un bouton « Effacer le brouillon », et montre Haiku qui prépare les instructions dans le champ du message.
- La fenêtre Nouvel agent ouverte par une extension démarre avec les réglages repliés.
- La fenêtre de lancement d’un agent par un autre est plus large, montre davantage du premier message et garde les onglets des projets sur une ligne.

### Projets

- Un projet retiré de la barre d’onglets revient dès qu’un de ses agents démarre ou reprend le travail.
- Les onglets des projets passent dans le menu de débordement avant que le bouton Pull requests ne pousse les boutons de la fenêtre hors de l’écran, et ce bouton garde son libellé sur une ligne.

### Plugins

- Un plugin peut suivre des groupes de projets : les projets ajoutés à un groupe suivi le reçoivent et ceux qu’on en retire le perdent. Son interrupteur est dans le bloc Projets, et un plugin en pause nomme les projets où il reviendra.
- Le menu Squadron a une entrée Plugins, avant Extensions.

### Extensions

- « Découvrir des extensions » liste les extensions en lignes, avec l’interrupteur, les réglages et la mise à jour pour celles installées, et Installer pour les autres. L’entrée Extensions du menu l’ouvre, et « Ajouter une extension locale » est à côté de « Ajouter une marketplace ».
- API d’extension 1.8.0 : une extension peut fournir les pull requests et les pipelines d’une forge, fils résolus compris, et les extensions qui lisent l’activité des agents voient lesquels ne sont pas lus.
- API d’extension 1.9.0 : une extension peut ouvrir un de ses panneaux, ou le ramener au premier plan, sans clic, dès le lancement de Squadron ou depuis n’importe lequel de ses événements.

## 0.52.0 · 2026-11-17

### Extensions

- Une extension peut montrer l’activité de Squadron : vos projets et groupes, l’état de chaque agent, ce qu’il attend et l’outil qu’il lance, les limites de 5 heures et de la semaine de Claude, et si l’API de Claude répond. Chacun demande une permission que vous accordez, et l’extension ne voit jamais une conversation, un dossier ni un secret.
- Un panneau d’extension peut mettre l’un de vos agents au premier plan quand vous cliquez dedans.
- Un panneau d’extension peut s’ouvrir dans sa propre fenêtre, flottante, agrandie ou en plein écran. Une fenêtre flottante revient à sa dernière taille et à sa dernière position.
- API d’extension 1.7.0 : `projects`, `groups`, `all-agents`, `usage` et `show-agent`, avec leurs événements, derrière les permissions `agents: ["activity", "show"]`, `projects: ["read"]` et `usage: ["read"]`, et `"window"` sur les panneaux.
- Une extension qui ne lit pas vos agents et ne lance aucun programme ne reçoit plus le dossier des agents, le texte d’une demande de permission ni le chemin de votre dossier personnel. Les prompts et les brouillons de nouvel agent envoyés par une extension n’atteignent que l’agent à l’écran.
- La fenêtre d’ajout d’une extension locale se lit naturellement en français, et son bouton de dossier ne déborde plus.

### Conversation

- La proposition de compacter une conversation dont le cache a expiré suit la vraie durée du cache, 5 minutes ou 1 heure, à partir d’une taille réglée dans Réglages.

## 0.51.0 · 2026-11-17

### Projets

- Un projet sans agent affiche le même en-tête qu’un agent : son nom, son dossier et sa branche, puis tous les outils à droite, outils d’extension comme Jira compris, pour ouvrir le terminal, l’éditeur ou le sprint Jira avant de lancer un agent.

## 0.50.0 · 2026-11-16

### Conversation

- Un clic droit sur une carte de fichier dans une conversation propose « Télécharger… » pour en enregistrer une copie.

### Projets

- Un clic droit sur un onglet de projet propose « Retirer de la barre d’onglets », qui sort le projet de la barre sans l’archiver.

### Pull requests

- Vos nouvelles pull requests GitHub proposent « Fusionner » dans leur diff tout de suite, avant que GitHub ne les liste dans sa recherche.

### Mises à jour

- La mise à jour propose toujours la version la plus récente, même quand une release plus ancienne finit de se construire après elle.

## 0.49.0 · 2026-11-16

### Extensions

- Une extension peut tester un compte avant que vous l’enregistriez : « Tester la connexion » dit si le site, l’e-mail et le jeton marchent, et marque le champ à corriger. Chaque compte de la page de l’extension montre s’il marche, avec « Vérifier à nouveau », et les Réglages marquent l’extension d’un point rouge quand l’un d’eux échoue.
- API d’extension 1.6.0 : `check-account` teste un compte, enregistré ou non, sans que les secrets n’atteignent jamais l’extension.

### Mises à jour

- « Rechercher des mises à jour » vérifie aussi les extensions et ouvre une seule fenêtre Mises à jour pour Squadron et ses extensions, et la barre d’état les montre dans une seule pastille en deux parties.

### Agents

- Quand un agent en lance un autre en mode accept edits ou auto, la demande montre les deux projets, le nom de l’agent, ses options, le mode expliqué et son premier message.

## 0.48.0 · 2026-11-16

### Projets

- Dans le panneau Projets, un projet se glisse à sa place dans son groupe : une ligne cuivre montre où il atterrit, et le déposer sur un projet d’un autre groupe l’y place à cet endroit.

### Conversation

- Un long lien dans le champ de message s’affiche en puce coupée à la largeur du champ au lieu de faire défiler le champ de côté.

### Agents

- Un agent qui relit une pull request affiche son indicateur d’activité dans la liste des agents, à côté du numéro de la pull request.

### Mises à jour

- La barre d’état prévient quand le Claude Code installé n’est pas la version avec laquelle ce Squadron a été testé, désormais 2.1.310, et son infobulle indique la version testée.

## 0.47.0 · 2026-11-16

### Notifications

- Réglages › Notifications : notifications du bureau quand un agent pose une question, demande une permission, a un plan prêt, demande un mot de passe, finit un tour, atteint son objectif « Done when » ou s’arrête sur une erreur, et pour les pipelines, les pull requests qui vous attendent, les mises à jour et les extensions. Chacune a son interrupteur, les événements d’agent leur carillon, et un clic ouvre l’agent ou la page concernée.
- Par défaut, elles s’affichent quand Squadron est en arrière-plan. Les notifications de pipelines rejoignent cette page : si elles étaient coupées, elles le restent.
- Une section Contrôle à distance active le contrôle à distance pour chaque nouvel agent, notifie votre téléphone quand une action est requise ou quand Claude le décide, copie chaque conversation sur claude.ai et choisit si les agents sous contrôle à distance notifient encore le bureau.
- « Envoyer une notification de test » l’envoie 5 secondes plus tard, et un clic dessus ouvre l’agent affiché.

### Projets

- Le panneau Projets redevient un arbre : chaque groupe de premier niveau est une carte qui contient ses sous-groupes et ses projets, avec repli, recherche, glisser-déposer et menus comme avant, et les projets sans groupe en dessous.
- Un bouton Nouveau groupe se trouve à côté d’Ajouter un projet, et un groupe ou un sous-groupe montre un + au survol qui crée un groupe à l’intérieur. Supprimer un groupe depuis son menu n’échoue plus.

### Conversation

- Un message envoyé pendant que l’agent lance une longue commande indique en dessous qu’il sera lu à la fin de l’action, avec un lien qui arrête la commande et le fait lire tout de suite. Les tâches de fond continuent.
- La carte de permission montre en entier un message adressé à un autre agent.
- Un worktree, un fork ou un retour en arrière demandé par l’agent est conservé quand Claude Code ne démarre pas, et Réessayer repart au bon endroit.
- La relecture d’une pull request récupère son dernier commit et prévient quand il est introuvable.

### Permissions et sécurité

- Une règle « Toujours » sur une commande Bash qui en lance une autre (timeout, env, sudo, bash -c…) ne couvre plus que cette commande exacte, et les anciennes règles larges de ce genre redemandent.
- Les commandes de Squadron ne répondent qu’à ses propres pages et les liens sont interceptés dans toutes les fenêtres : une page écrite par un agent ne peut jamais piloter l’app. Les artifacts ne peuvent plus envoyer de requêtes hors de Squadron.
- Les agents qui relisent une pull request ne chargent plus les réglages, hooks et serveurs MCP de la branche.
- La carte de mot de passe montre la commande Bash que l’agent lance vraiment.
- L’éditeur externe n’ouvre plus les fichiers que le système exécuterait.
- Rouvrir Squadron ramène la fenêtre déjà ouverte au lieu de lancer une seconde app sur les mêmes données.

### Extensions

- API d’extension 1.5 : host::open-new-agent ouvre la fenêtre Nouvel agent remplie depuis un brouillon (nom, premier message, Done when, un lien, des cases à cocher, un bandeau de préparation), et host::update-new-agent la complète sans écraser ce que vous avez tapé.
- Les extensions peuvent afficher leur propre logo SVG sur leur page, dans les Réglages, Gérer les extensions, les mises à jour, Nouvel agent et sur leurs boutons.
- Les options qu’une extension passe à Claude Code doivent correspondre à celles que vous avez approuvées et ne peuvent plus changer les permissions.

### Mises à jour

- Mettre à jour pendant qu’apt ou dpkg tient le verrou des paquets le signale et attend, au lieu de demander deux fois le mot de passe.
- Le fichier de réglages de Squadron est écrit de façon atomique et mis de côté en .bak s’il est illisible, au lieu d’être perdu.
- Les captures jointes aux cartes Kanban ne sont plus effacées au bout d’une semaine.

## 0.46.2 · 2026-11-13

### Extensions

- Un champ d’un compte ou d’un secret d’extension peut pointer vers la page où on le crée, affichée sous le champ et ouverte dans le navigateur.

### Conversation

- La flèche du haut ne ramène un message précédent que si la zone de saisie est vide ou montre encore celui rappelé.

### Mises à jour

- La 0.46.1 n’a pas pu être construite : cette version reprend ses changements.

## 0.46.1 · 2026-11-13

### Mises à jour

- Une nouvelle version arrive sur Linux, macOS et Windows dès que leur build est prêt, sans attendre le plus lent.

## 0.46.0 · 2026-11-13

### Extensions

- Settings a un groupe Extensions : + ouvre Découvrir, l’engrenage ouvre Gérer, et chaque extension activée a sa page avec ses comptes, ses secrets et ses options, une fenêtre À propos pour ses permissions et un bouton Désinstaller.
- Un bouton de la barre d’état compte les extensions qui ont une mise à jour : les mettre à jour une par une ou toutes d’un coup, lire les nouveautés de leur changelog, et voir lesquelles demanderont de nouveaux accès.
- Les extensions déclarent les versions de Squadron sur lesquelles elles tournent : Découvrir et les mises à jour proposent la plus récente qui convient, et une extension installée qui ne convient plus est désactivée avec la version qu’elle demande.
- Les extensions peuvent déclarer des options que Squadron dessine sur leur page de réglages, et ajouter une page de réglages à elles.
- L’intégration Jira intégrée laisse la place à l’extension Jira : plus d’onglet Tickets, de sources de tickets, de tableau Jira dans le Kanban ni de raccourci J, et un ticket cité dans un message s’ouvre toujours dans Jira depuis sa puce.

### Sécurité

- Chaque agent joint Squadron avec sa propre clé : une commande qu’il lance ne peut plus se faire passer pour un autre agent, et overview et read_agent ne voient que son projet, sauf si tous les projets sont permis.
- « Toujours » sur une modification de fichier ne couvre que le dossier de l’agent hors .git, .claude et .mcp.json, s’arrête au redémarrage de Squadron et s’affiche « accept edits » sur la pastille du mode. Les blocs html d’une conversation ne chargent plus rien depuis le réseau.

### Projets

- Déplacer un projet emmène ses agents et leurs conversations.

### Corrections

- Envoyer une revue de pull request à la conversation qui l’a ouverte remet bien ses commentaires, et le bouton s’appelle Envoyer à la conversation.
- Un titre de fenêtre trop long s’arrête après deux lignes, et un mot trop long dans un message passe à la ligne au lieu de faire défiler la fenêtre de côté.
- Une puce de lien dont le site cache son favicon derrière une connexion montre celui du domaine parent.
- La mise à jour télécharge une release depuis son vrai tag.

## 0.45.1 · 2026-11-13

### Agents

- Un agent qui repart après un tour en échec, de lui-même ou par Réessayer, ne s’affiche plus en attente de vous, et seule la dernière carte d’erreur garde ses boutons : une ancienne ne peut plus renvoyer votre message.

### Extensions

- Une extension installée dont le catalogue a une version plus récente montre un bouton Mettre à jour sur sa carte.

## 0.45.0 · 2026-11-13

### Artifacts

- Le partage par une extension se fait dans la conversation de l’artifact : le skill de l’extension y tourne en sous-agent sur son propre modèle (Sonnet pour Google Workspace). Ses étapes animent un bandeau dans le panneau Artifacts, et ses demandes de permission s’ouvrent dans sa carte de la conversation.
- Les artifacts restés sans changement sont nettoyés : 30 jours par défaut dans Settings, plus longtemps pour ce qu’une extension partage, et chaque artifact fixe son propre délai, Jamais compris, depuis son menu ⋯. Le partage s’arrête avant que l’artifact parte, et supprimer un artifact ou un agent arrête aussi ses partages.

### Extensions

- API 1.3.0 : des comptes que vous ajoutez dans Settings › Extensions, avec un hôte que l’extension peut joindre et des secrets gardés dans le trousseau ; {{account.field}} et {{base64:a:b}} dans les en-têtes de requête pour l’authentification Basic ; fetch-bytes, download et open-file ; exec-input avec le Claude Code de Squadron.
- artifact.share prend cleanup.days, un prompt de partage va dans la conversation de l’artifact, et set_progress prend artifact.

### Agents

- add_project : un agent ajoute un dossier comme projet par la carte de permission, sans changer votre vue ; un projet archivé revient avec ses agents.
- Un skill lancé en sous-agent forké s’affiche en une seule carte dans la conversation, ses demandes de permission dedans.

### Mises à jour

- Une vérification des mises à jour qui échoue dit que GitHub ne répond pas, avec son logo, et les releases restent en brouillon tant que toutes les versions ne sont pas envoyées.
- Un lien qui chargerait un site dans une fenêtre de Squadron s’ouvre dans le navigateur.

## 0.44.0 · 2026-11-12

### Artifacts

- Le panneau Artifacts a un menu Partager : Publier sur claude.ai en est la première destination, et les extensions ajoutent les leurs. Une fois partagée, une destination propose Ouvrir, Copier le lien, Mettre à jour quand l’artifact a changé depuis, et Arrêter le partage ; un bandeau s’affiche pendant le travail.
- Copier la source, Ouvrir le dossier (ou Enregistrer dans un fichier) et Supprimer passent dans un menu ⋯, l’en-tête du panneau tient à sa largeur par défaut.

### Extensions

- Un élément du nouvel emplacement artifact.actions partage l’artifact ouvert avec artifact.share : une confirmation, puis des étapes écrites en prompt, menées par un agent dédié, ou en appel au WebAssembly de l’extension. Le premier clic demande les accès en attente de l’extension.
- contributes.skills donne des skills à tes agents une fois que tu les autorises.
- Chaque extension a un dossier de données gardé aux mises à jour (host::data_dir, API 1.2.0). Les extensions compilées en 1.1.0 se chargent toujours.

## 0.43.0 · 2026-11-12

### Pull requests

- La page Pull requests liste toutes les pull requests ouvertes du projet : celles que tu relis, celles que tes agents relisent sans toi, les autres en revue (sans les brouillons), et les tiennes.
- Le bouton d’agent relecteur d’une pull request dit où en est son agent, affiche le démarrage sur lui-même pendant que « Review » attend grisé, et devient « Relire la suite » quand des commits sont arrivés après la relecture de l’agent. Ses boutons gardent une largeur fixe, les lignes restent alignées.

### Spotlight

- Tab garde un projet, une action ou un agent en puce et liste ce que tu peux en faire ; chat lance une conversation sans dossier.
- Le spotlight trouve une pull request par son titre ou son numéro et liste celles qui t’attendent dans tous les projets ; Tab sur l’une d’elles propose la relecture, la relecture par un agent, la page de l’hébergeur, approuver, demander des changements et fusionner.
- Les entrées des extensions apparaissent sous leur libellé avec le nom de l’extension, trouvées par leurs mots optionnels, sauf si elles mettent spotlight à false.

### Mots de passe

- Squadron ne transmet jamais un mot de passe tapé à un agent : run_sudo lance une commande root après que tu as tapé le mot de passe dans une carte, redemandé à chaque commande, et add_ssh_key déverrouille une clé SSH dans Squadron et la charge dans l’agent SSH. L’assistant de mot de passe laissé dans l’environnement de l’agent ne répond que oui ou non.

### Jira

- Les tickets s’ouvrent dans une fenêtre avec les filtres à gauche et des colonnes de statut de largeur fixe : masquer un statut n’élargit plus les autres.

### Fenêtre

- Quitter Squadron pendant que des agents travaillent demande confirmation.
- Le réglage Projets dans la barre d’onglets propose Toujours, qui garde tous les projets dans la barre.

### Corrections

- La conversation affichée n’est plus déchargée quand son projet n’a pas d’agent courant enregistré.

## 0.42.0 · 2026-11-10

### Mémoire

- Un agent qui a fini arrête son processus Claude Code au bout de 15 minutes au lieu d’une heure, avec ses serveurs MCP et les navigateurs qu’ils ont ouverts ; ton prochain message le relance. Une boucle arrêtée, un réveil passé ou un cron supprimé ne le gardent plus en vie.
- La conversation d’un agent arrêté que tu n’as pas ouvert depuis 15 minutes quitte la mémoire, et se recharge quand tu l’ouvres.
- Archiver un agent de revue supprime le checkout de sa pull request, sauf s’il contient des modifications ou des commits non poussés ; le restaurer remet en place le commit relu. Les checkouts orphelins sont supprimés au démarrage.

### Serveurs MCP

- Serveurs MCP à la demande : Réglages › Serveurs MCP sort un serveur de la configuration de Claude Code, pour qu’il ne démarre dans aucune session. Un agent le démarre pour lui seul depuis /mcp, ou de lui-même avec l’outil use_mcp quand sa tâche en a besoin, et le retrouve après une relance jusqu’à ce qu’il l’arrête.

### Conversation

- Les notes d’un agent entre ses étapes s’affichent en blocs de réponse avec leur heure.
- Une réponse plus haute qu’une fois et demie la conversation se termine par un bouton « Revenir en haut ».
- Le champ de réponse à la question d’un agent passe à la ligne et grandit avec le texte ; Maj+Entrée crée une nouvelle ligne.

### Archives

- La liste des agents archivés s’ouvre vite : elle ne lit que l’en-tête de chaque agent, et la fenêtre d’archives charge l’historique de l’agent affiché.

## 0.41.0 · 2026-11-10

### Conversation

- Un lien dans votre message s’affiche en puce avec l’icône du site, et une carte sous le message donne le titre, la description et l’image de la page. Les liens tapés dans la zone de saisie deviennent aussi des puces, que Ctrl+clic ouvre.
- Une commande en arrière-plan reste affichée dans la conversation pendant que vous parlez à l’agent, jusqu’à ce qu’elle se termine.
- Les artefacts d’un agent se choisissent dans une liste au lieu d’une rangée d’onglets.

### Recherche

- La fenêtre de recherche s’ouvre vite même avec beaucoup d’agents archivés, et ses filtres par projet et agents archivés apparaissent dès qu’on tape.

## 0.40.0 · 2026-11-10

### Conversation

- Les notes qu’un agent écrit entre ses étapes restent visibles dans la conversation, avec un liseré clair à gauche, au lieu de se replier dans la ligne des étapes.

### Pull requests

- Sur une pull request à relire, le bouton de l’agent est une icône carrée à côté de Relire, avec l’état de l’agent de relecture dans son coin, et les boutons Relire s’alignent d’une ligne à l’autre.

## 0.39.1 · 2026-11-10

### Pull requests

- Sur une pull request à relire, le bouton qui demande un agent ou montre sa relecture est à côté de Relire, et non plus dans son menu.

## 0.39.0 · 2026-11-10

### Pull requests

- Un agent peut changer le titre et la description d’une pull request avec update_pull, avec votre compte, sur Bitbucket, GitHub, GitLab et Azure DevOps.
- Une pull request fusionnée ou fermée ne propose que des commentaires quand vous terminez sa relecture, et l’agent de relecture porte le nom de la pull request.

### Extensions

- Une extension peut voir vos agents et ce qu’ils font, archiver ceux qui ne travaillent pas, et se réveiller avec un minuteur, une fois que vous l’autorisez (API d’extension 1.1).

### Conversation

- Quand la réponse d’un agent cite plus de trois fichiers, seules les deux premières cartes s’affichent, et une barre liste les autres jusqu’à ce que vous demandiez à toutes les voir.

## 0.38.1 · 2026-11-09

### Correctifs

- Le détail du contexte s’affiche dès l’ouverture de la jauge, même quand l’agent n’a reçu aucun message depuis le démarrage de Squadron.

## 0.38.0 · 2026-11-09

### Contexte

- La jauge de contexte ouvre le détail de tout ce qu’il contient, au démarrage comme depuis la conversation : la part de chaque plugin comptée par Claude Code, les instructions d’un skill de plugin à part de vos propres messages, et les instructions de compaction. Les réglages ont un onglet pour cela, et la pastille des plugins disparaît.
- Un skill de plugin lancé vous-même avec sa commande slash compte dans l’usage des plugins de l’agent, au coût que le plugin déclare pour lui.

### Extensions

- Une extension peut lancer Claude Code avec ses propres options, comme --chrome, une fois que vous les autorisez.

### Pull requests

- Une pull request fusionnée se rouvre depuis la pastille de relecture, avec ses changements et vos remarques en brouillon à publier.

### Conversation

- Déplier le contenu d’une carte de fichier fait défiler la conversation jusqu’à placer la carte juste sous l’en-tête de l’agent.

### Correctifs

- Un agent ne s’arrête plus avec « mcp__squadron__permission not found » après une seconde ouverture de Squadron.
- Les coûts des plugins se chargent sans passer d’abord par les réglages Plugins.
- Le champ de message ne garde plus une bande vide sous son texte indicatif après l’apparition d’une suggestion, et reprend sa hauteur quand la suggestion change alors qu’il n’a pas le focus.

## 0.37.0 · 2026-11-09

### Plugins

- Les réglages ont un onglet Plugins : installez des plugins Claude Code et activez chacun pour tous les projets ou seulement ceux que vous cochez.
- Chaque agent montre les plugins qu’il a chargés et ce qu’ils coûtent, et la conversation marque l’endroit où un plugin est intervenu.

### Projets

- Le panneau Projets affiche les groupes en onglets, les sous-groupes en pastilles et les projets en cartes avec leurs agents ouverts, questions, erreurs et agents terminés.
- Il se trie par activité, attention, nom, agents ouverts ou à la main, s’élargit sur plusieurs colonnes en tirant ou double-cliquant son bord, et l’effet d’une carte de projet au travail (serpent, bande animée ou pulsation) se choisit dans les Préférences.

### Zone de saisie

- Un fichier ou un dossier cité avec @ s’affiche en pastille, avec une icône colorée selon le type de fichier, ou pour un dossier son chemin parent et son nombre de fichiers. Les messages envoyés montrent les mêmes pastilles.

### Conversation

- Un gros fichier s’ouvre vite dans sa carte : l’aperçu montre ses 300 premières lignes et charge la suite quand vous défilez.
- Les cartes de fichier et les pièces jointes affichent une icône du type de fichier au lieu du texte de l’extension.
- Le contenu d’une carte de fichier se déplie vers le bas en glissant, et la conversation défile juste assez pour le montrer.

### Tickets

- Un ticket Jira s’ouvre dans une fenêtre entière : description, pièces jointes et commentaires à gauche, champs et liens à droite, depuis le Kanban comme depuis une conversation.

### Pull requests

- Un formulaire de commentaire ou de réponse s’ouvre en glissant sous sa ligne, et le diff défile juste assez pour le montrer.
- Terminer la relecture envoie à l’agent les commentaires ajoutés pour lui en plus du brouillon de relecture, et les compte.

### Agents

- Un agent peut se renommer, ou renommer un autre agent de son projet, avec l’outil set_title.
- Chaque agent lit le DESIGN.md du projet avant un artifact, un bloc html ou une retouche de style, et un artifact html peut lier /__squadron/theme.css pour avoir les couleurs, classes et polices de Squadron.

### Correctifs

- Cliquer sur Mettre à jour recherche à nouveau la dernière version, si bien qu’une version publiée depuis la dernière vérification est celle qui s’installe.
- Sous Windows, un terminal ou une commande ! sur un projet sous \\wsl$ s’ouvre dans sa distribution WSL et son dossier.
- Une pull request ne reste plus bloquée sur le chargement quand son commit relu change pendant qu’elle charge, ou quand deux brouillons de commentaire partagent une ligne.
- Le second commentaire d’un agent de relecture sur une ligne remplace le premier au lieu de le dupliquer.
- L’onglet Plugins ne reste plus sur Chargement quand un plugin est installé dans deux portées du même projet.

## 0.36.1 · 2026-11-09

### Correctifs

- Les cartes de fichier d’une réponse s’empilent une par ligne au lieu de se ranger côte à côte.
- Le premier champ de message d’un nouvel agent a le focus à l’ouverture de la fenêtre, et le curseur reste visible en début de ligne.
- La barre d’état affiche la version de Squadron à gauche.
- L’en-tête du panneau de ticket Jira tient sur une ligne : type et sprint se raccourcissent, Tout voir et Ouvrir dans Jira ne gardent que leur icône quand le panneau est étroit.

## 0.36.0 · 2026-11-06

### Extensions

- Squadron s’étend sans être recompilé. Les extensions ajoutent des boutons, des entrées de menu, des panneaux, des couleurs et des outils pour vos agents, avec un extension.json et, pour leur logique, du Rust compilé en WebAssembly.
- Réglages › Extensions, aussi dans le menu Squadron, est un catalogue avec recherche : installation, mise à jour et désinstallation en un clic, depuis la source officielle ou les dépôts GitHub que vous ajoutez, ou chargement d’un dossier en cours d’écriture, rechargé à chaque build.
- Une extension demande avant de lancer une commande ou de joindre un site. Ses jetons restent dans le trousseau du système et ses panneaux sont isolés.
- Écrire la vôtre avec le guide : https://github.com/Jgrasp/squadron-extensions/blob/main/docs/guide.md

### Zone de saisie

- Les champs de message sont des éditeurs de code : un bloc de code délimité s’affiche en police mono et en couleurs pendant la frappe, dans la conversation et la fenêtre de nouvel agent.
- Le code collé arrive dans un bloc délimité avec son langage, pris dans VS Code ou deviné. Ctrl+Maj+V colle le texte brut.
- L’avertissement sur une ancienne conversation accepte des instructions de compaction, passées à /compact avant l’envoi du message.

### Conversation

- Un chemin de fichier écrit en code dans une réponse a sa carte de fichier. Les cartes de fichier prennent toute la largeur de la réponse, et plusieurs à la suite se rangent en colonnes.
- Une carte à une seule question a son propre champ de réponse libre, qui ressemble désormais à un champ.
- Les tableaux Markdown gardent des colonnes lisibles : plus de retour à la ligne lettre par lettre, et le code d’une cellule reste sur une ligne.
- Chercher dans une longue conversation ne fige plus Squadron dès la première lettre, et la barre de recherche ne se rouvre plus dans la conversation suivante.

### Panneaux

- Un clic hors d’un panneau en superposition le ferme ; les panneaux à côté, en bas et en plein écran restent ouverts.
- Les panneaux glissent sans à-coups, et un redimensionnement ne déplace que le panneau jusqu’au relâchement.
- Le ticket Jira ouvert depuis le Kanban couvre le tableau sous son en-tête.

### Correctifs

- La branche et la pull request d’un agent suivent son worktree après un checkout ou un switch.
- Sous Linux, une copie faite juste après être revenu dans la fenêtre remplace bien le contenu du presse-papiers.

## 0.35.3 · 2026-11-05

### Correctifs

- Un Ctrl+clic ou Maj+clic sur un lien d’une conversation l’ouvre une seule fois dans le navigateur, au lieu de deux.

## 0.35.1 · 2026-11-05

### Correctifs

- Sur Windows, les fichiers que Claude donne dans une conversation s’ouvrent de nouveau : leur carte ne dit plus que le fichier n’existe plus quand Claude écrit son chemin en /c/Users/… ou C:\…
- Quand un fichier ne s’ouvre pas, l’infobulle de sa carte montre toujours son chemin.

## 0.35.0 · 2026-11-05

### Nouvel agent

- Quand le premier message commence par /, le dialogue de nouvel agent propose les skills avec leurs arguments, comme le champ de saisie.

### Conversation

- La conversation et le champ de saisie sont centrés sur la même colonne : le rail des messages descend à côté du champ de saisie.
- Arrêter un agent remet le curseur dans son champ de saisie.

### Interface

- Squadron installé se rouvre à la taille qu’il avait, et agrandi s’il l’était.

## 0.34.0 · 2026-11-05

### Conversation

- Après une interruption, ↑ dans le champ de saisie vide reprend le dernier message : il quitte la conversation et la session de Claude, qui repart juste avant lui, et revient dans le champ avec ses fichiers et ses commentaires.
- Chercher une lettre courante dans une conversation ne fige plus Squadron : au-delà de mille occurrences, seule l’occurrence courante est surlignée.
- Une tâche de fond qui ne fait qu’attendre ne propose plus Voir la sortie, qui ouvrait un terminal vide.

### Panneaux

- Dans le panneau Pull requests, Actualiser a une icône de flèche circulaire qui tourne pendant l’actualisation, et a la même hauteur que le menu de placement et le bouton de fermeture à côté.

## 0.33.0 · 2026-11-04

### Panneaux

- **À côté de la discussion** : un panneau peut s’ouvrir à côté de la discussion, qui se resserre pour lui faire place, au lieu de passer par-dessus. Les boutons de placement de l’en-tête d’un panneau deviennent un menu de quatre placements dessinés en vignettes : par-dessus, à côté, dessous et plein écran.
- Réglages › Panneaux présente chaque panneau sur une ligne de placements, avec une taille Petit, Moyen ou Grand (un tiers, la moitié ou deux tiers de la place). Un panneau redimensionné par son bord affiche Sur mesure.
- Le panneau d’un ticket Jira est un panneau comme les autres : il se redimensionne par son bord, change de placement depuis son en-tête et a sa ligne dans Réglages › Panneaux.

### Conversation

- Dans une carte de plusieurs questions, cliquer de nouveau sur le choix retenu le désélectionne, pour répondre à la question par un texte libre seulement.
- Pendant l’écriture d’un bloc html, son emplacement montre le triangle de Squadron, dont le contour est parcouru d’un trait cuivré, au-dessus de « Création du rendu… ».
- Dans la fenêtre Nouvel agent, Annuler et Lancer l’agent affichent leur raccourci, Échap et Ctrl ↵, dans le bouton.

## 0.32.0 · 2026-11-04

### Conversation

- **Tickets Jira dans les messages envoyés** : un ticket joint à un message s’affiche en puce avec sa clé et son titre. Un clic ouvre le panneau du ticket à côté de la conversation, avec sa description, ses commentaires et ses liens.
- Le panneau du ticket s’ouvre sous l’en-tête de l’agent au lieu de passer dessous.

### Hébergement du code

- Réglages › Hébergement du code présente Bitbucket, GitLab, Azure DevOps et GitHub en cartes avec leur logo et le compte. Les étapes de connexion d’un hébergeur s’ouvrent sous sa ligne au clic sur Connecter.

### Pull requests

- Demander à un agent et Relire les nouveautés affichent un indicateur de chargement sur le bouton de la ligne pendant le lancement de l’agent, au lieu de sembler ne rien faire.

### Corrections

- Les tokens, dont celui de GitHub utilisé par gh, sont lus dans le trousseau du système une fois par lancement au lieu de chaque rafraîchissement des pull requests : un trousseau redémarré verrouillé ne ramène plus la demande de déverrouillage toute la journée.
- Le modèle et l’effort dans la fenêtre Nouvel agent et sur une carte redeviennent de simples libellés, avec l’advisor sur une ligne.

## 0.31.0 · 2026-11-04

### Hébergement du code

- **GitLab et Azure DevOps** : Réglages › Hébergement du code se connecte à GitLab (gitlab.com ou une instance auto-hébergée) et à Azure DevOps avec un token gardé dans le trousseau du système. La pull request de la branche d’un agent, les listes de pull requests à relire et des vôtres, les liens dans les conversations, les commentaires et réponses, la relecture avec ses commentaires sur ligne, l’approbation, le merge et les pipelines y fonctionnent comme sur Bitbucket et GitHub.
- Demander des changements n’est pas encore disponible sur GitLab : les commentaires sont publiés et Squadron indique de le faire sur GitLab.

## 0.30.0 · 2026-11-04

### Conversation

- **Chercher dans la conversation** : Ctrl+F ouvre une barre de recherche dans la conversation, avec le nombre de résultats et précédent et suivant.
- **Connecteurs MCP** : une puce de la barre de conversation liste les connecteurs MCP de l’agent, avec un filtre. Elle apparaît dès que l’agent a au moins un serveur MCP et peut être masquée dans les réglages de disposition.

### Pull requests

- **Demander des changements** : une pull request peut être renvoyée depuis son menu, à côté d’Approuver.

### Nouvel agent

- Le modèle et l’effort se changent directement dans le résumé de la fenêtre Nouvel agent et d’une carte du Kanban, sans ouvrir les réglages. Les réglages restent fermés à chaque ouverture de la fenêtre.

## 0.29.1 · 2026-11-04

### Sécurité

- L’app livrée embarque une Content Security Policy : les scripts ne se chargent que depuis l’app, une chaîne injectée ne peut plus exécuter ni télécharger de code.
- Le jeton MCP n’est plus visible de tous les processus de la machine : Claude Code le lit dans un fichier privé.
- Un mot de passe sudo, ssh ou git retenu est lié à la commande à laquelle il a répondu, et la carte affiche cette commande : une autre commande lancée par un agent ne peut plus le rejouer en silence.
- Un lien symbolique cassé dans un projet ne peut plus faire écrire l’éditeur ou ask_values hors du projet ; un texte de tâche qui commence par un tiret ne peut plus être lu comme une option de Claude Code.

### Corrections

- Squadron redémarre depuis son AppImage sur les distributions autres qu’Ubuntu, et un lien ou un fichier que GIO ne sait pas ouvrir, comme sur Fedora, s’ouvre via xdg-open.
- Un fichier déposé sur la fenêtre Nouvel agent rejoint ses pièces jointes au lieu de la conversation derrière. Un fichier copié dans le gestionnaire de fichiers puis collé est joint au lieu d’être collé comme chemin, dans la conversation aussi.
- Une pull request GitHub sur laquelle on vient de vous demander une review apparaît tout de suite dans la liste des reviews, sans attendre l’index de recherche de GitHub.
- Un agent qui attend une commande en arrière-plan ne reste plus bloqué indéfiniment.
- Un projet Bitbucket dont le remote pointe vers ssh.bitbucket.org est toujours reconnu.
- ask_values indique à l’agent que les valeurs ont été écrites dans le fichier sans lui être renvoyées.

## 0.29.0 · 2026-11-03

### Démarrage

- **Écran d’ouverture** : pendant que Squadron démarre, une escadrille se forme au milieu de la fenêtre. Les ailiers rejoignent la formation à mesure que les agents reviennent, sous l’étape en cours et le nombre d’agents ; puis elle vous salue par votre prénom et s’envole pour dévoiler l’app. Un clic ou une touche la passe.
- Squadron est prêt en une demi-seconde même avec beaucoup de longues conversations : l’historique d’un agent n’est lu qu’à l’ouverture de sa conversation ou quand une recherche en a besoin, et la fenêtre ne se fige plus pendant le retour des agents.

## 0.28.1 · 2026-11-03

### Corrections

- La puce de contrôle à distance d’une conversation l’active d’un clic, sans menu ; le menu reste pour quand il est actif. La puce ne grandit plus à l’activation.
- Toutes les façons d’ouvrir un panneau le font glisser comme son bouton : le bouton Pull requests de la barre de titre, la Vue d’ensemble, la palette de commandes, une carte de pull request, le bouton Kanban d’une tâche. Ouvrir la liste des pull requests ferme le panneau ouvert dans la conversation, qui ne revient plus quand la liste se ferme.
- Les menus de la barre de titre et de la barre de conversation, comme Pull requests à relire ou Actions et pipelines, s’ouvrent au-dessus du panneau Kanban ou Pull requests au lieu de passer dessous.

## 0.28.0 · 2026-11-03

### Agents

- **Liste de tâches dans l’en-tête** : la liste de tâches d’un agent passe du bas de la conversation à une puce de son en-tête qui l’ouvre. Une liste dont toutes les tâches sont faites laisse place à une nouvelle au message ou à la tâche suivante, et la précédente reste à portée dans le menu.

### Pull requests

- Review sur une carte de pull request ou sur la puce de review d’une conversation ouvre seulement le Diff de cette pull request, sans la liste des pull requests.
- Le panneau de review glisse en se fermant, comme les autres panneaux.

## 0.27.0 · 2026-11-03

### Questions

- **Votre propre réponse par question** : dans une carte à plusieurs questions, chaque question a son champ, pour une réponse libre ou un commentaire sur l’option choisie, envoyé avec elle. Un × l’efface. Le cadre ne saute plus à la question suivante non répondue.

### Agents

- À la réouverture de Squadron, chaque projet affiche la dernière conversation visitée au lieu de la première.
- Un agent dont l’objectif a été envoyé pendant que Squadron était fermé ne reste plus en cours indéfiniment après la fin de ses tours.
- Le panneau sous l’en-tête d’un agent ne clignote plus en blanc en passant de la Vue d’ensemble à un projet.

### Pull requests

- Le Diff d’une pull request ouverte depuis le panneau Pull requests couvre le panneau depuis son haut, en-tête compris.

## 0.26.0 · 2026-11-02

### Panneaux

- **Placement par panneau** : Terminal, Éditeur, Diff, Sous-agents, Artifacts, Kanban et Pull requests s’ouvrent chacun par-dessus la discussion, en plein écran, ou sous la discussion, qui se réduit alors. Les boutons de l’en-tête du panneau et Réglages › Panneaux le règlent panneau par panneau, avec sa largeur ou sa hauteur.
- **Transitions** : un panneau qui s’ouvre de la même façon et au moins aussi grand passe par-dessus celui qui est ouvert ; sinon, celui-ci se rabat pendant que le nouveau s’ouvre. Un clic sur un agent dans la liste ferme le panneau ouvert et affiche sa conversation.

### Pull requests

- **Terminer la revue** : sur une pull request que vous relisez, Squadron demande le verdict, Commenter, Approuver ou Demander des changements, et l’envoie sur GitHub ou Bitbucket. Une pull request déjà relue par un agent de revue affiche cet agent ; une pull request qui attend votre revue propose Demander à un agent dans son menu Revue. Les commentaires d’un agent de revue ne tombent que sur des lignes que le diff affiche, et les refus de GitHub disent pourquoi.
- **Vos commentaires** affichent votre avatar cerclé de cuivre. Un commentaire sur une ligne supprimée au long numéro n’est plus coupé dans l’arbre des fichiers, la barre de revue passe à la ligne quand le panneau est étroit, et sur Bitbucket une approbation qui échoue ne poste plus le résumé deux fois en réessayant.

### Agents

- **Jauge de contexte** : les nouveaux tokens tombent un à un dans l’anneau et le niveau monte à chacun.
- **Agents épinglés** : ils gardent l’ordre que vous leur avez donné, quel que soit le tri. Pendant un tri, seuls les agents épinglés se déplacent, et les déplacer ne coupe plus le tri.
- Une carte de tâche dans une conversation propose Aller à l’agent et Voir la tâche.
- Un agent qui reprend après ses tâches de fond relance le chronomètre du tour et le compte des tokens.
- Dans les Réglages, Enregistrer reste grisé tant que rien n’a changé.

## 0.25.0 · 2026-11-02

### Pull requests

- **Commentaires dans la revue** : chaque commentaire s’affiche sous son fichier dans l’arbre, avec sa ligne, l’avatar et le nom de son auteur et les avatars de ceux qui ont répondu. Un clic amène à la ligne, son fil mis en avant. « ‹ 2 of 4 › » dans la barre de revue, N et Maj N passent d’un commentaire à l’autre dans toute la pull request. Vos commentaires s’affichent sous leur ligne au lieu d’une icône dessus, et chaque message d’un fil montre l’avatar de son auteur.
- **Revoir les nouveaux changements** : quand l’auteur pousse après votre revue, Review new changes envoie les nouveaux commits à l’agent de revue ouvert depuis Pull requests, dans sa propre conversation ; See review agent y mène. Une pull request approuvée le reste quand de nouveaux commits arrivent et ne vous attend plus. Les agents de revue affichent le numéro de leur pull request et le verdict posté dans la liste des agents.
- **Fusionner ou relire** : la pull request d’un agent ne propose Merge que si vous l’avez écrite, sinon Approve ou Request changes. Une pull request ouverte depuis le diff d’un agent retrouve sa barre de revue, même avant tout commentaire.

### Agents

- **Conseiller** : chaque agent peut tourner avec un conseiller, Sonnet, Opus ou Fable, que Claude consulte aux moments clés. La fenêtre New agent montre le message avec le modèle, l’effort et le conseiller dessous ; Settings déplie le nom, Done when, les permissions et l’agent. La conversation montre chaque consultation en lilas, puis son avis, avec ses tokens. L’en-tête et le menu de l’agent changent le conseiller, et /advisor tapé dans un agent ne change que cet agent.
- **En-tête de l’agent** : un seul bouton montre le modèle, son effort en barres de la couleur du niveau et le conseiller ; le mode de permission est un badge à sa couleur Claude Code. Le contexte est un anneau qui se remplit, vert, orange puis rouge, avec le pourcentage, les tokens ou les deux.
- **Tâches en arrière-plan** : un agent dont le tour s’est terminé pendant qu’une commande lancée en arrière-plan tourne encore reste au travail. Sa ligne dit ce qu’il attend, la carte Kanban le montre, et la conversation affiche une carte In the background avec la durée de chaque tâche, un lien View output et un bouton Stop.

### Conversation

- **Chemins de fichier** : un chemin de fichier écrit en code dans une réponse, seul ou avec son :ligne, ouvre le fichier dans l’éditeur au clic quand il existe dans le projet.

## 0.24.0 · 2026-10-30

### Éditeur

- **Chercher dans le fichier** : Ctrl F ouvre une bulle sur le code, en haut à droite, avec casse, mot entier et expression régulière, le nombre d’occurrences et une ligne Remplacer derrière la flèche.
- **Rechercher** : Ctrl Maj F ou le bouton Rechercher ouvre une palette. On tape un bout de nom pour ouvrir un fichier, :42 pour arriver à la ligne 42. Tab passe en Texte, qui cherche dans tous les fichiers du projet, montre les lignes autour du résultat choisi et ouvre le fichier à cette ligne, la recherche déjà remplie.

## 0.23.1 · 2026-10-30

### Corrections

- **Réflexions en direct** : pendant que Claude travaille, chaque réflexion du tour s’affiche en entier dans son propre bloc entre les étapes, la réflexion en cours marquée par l’accent et un point qui pulse, au lieu d’une fenêtre défilante. Quand la réponse arrive, elles se replient dans les étapes.

## 0.23.0 · 2026-10-30

### Conversation

- **Réflexions en direct** : pendant que Claude travaille, les réflexions du tour s’affichent sous les étapes dans une fenêtre défilante qui suit la dernière, séparées par leur durée et les actions faites entre deux. Remonter fait apparaître un bouton Suivre, et la fenêtre s’estompe aux bords quand le texte continue. À la fin du tour, les réflexions se replient dans les étapes.

## 0.22.0 · 2026-10-30

### Interface

- **Panneaux par-dessus la conversation** : Terminal, Éditeur, Diff, Kanban et Pull requests glissent depuis la droite par-dessus la conversation, sous la barre d’outils de l’agent. Ils ont une seule largeur ; le grand écran s’arrête à la liste des agents ; changer d’outil remplace le panneau et le fermer le fait glisser vers la droite.
- **Liste des agents** : elle ne se replie plus en avatars et le réglage Rail disparaît. Ctrl B masque la liste ; l’épingle de son en-tête la remet.
- **Pull requests** : le bouton de la barre de titre affiche Pull requests à côté de son compteur.

### Conversation

- **Cartes de fichier** : un lien vers un fichier du projet écrit en chemin absolu devient aussi une carte.

### Corrections

- **Barre des agents masquée** : elle s’ouvre dès que la souris bouge sur le bord gauche, même si elle y était déjà.

## 0.21.0 · 2026-10-30

### Interface

- **Largeur de la conversation** : Réglages › Préférences la règle sur étroite, normale, large, toute la place, ou une largeur personnalisée en pixels.
- **Barre des agents** : toujours visible, masquée jusqu’à ce que la souris touche le bord gauche, ou en rail d’avatars qui ouvre la liste au survol. L’épingle de son en-tête passe de visible à masquée.
- **Menu ⋯ à côté d’Envoyer** : changer le modèle, l’effort et les permissions dans des sous-menus, chacun avec sa description.

### Conversation

- **Fichiers du projet** : un lien vers un fichier du projet dans la réponse de l’agent devient une carte : l’ouvrir dans le panneau Éditeur, déplier son contenu dans la conversation, ou l’ouvrir dans une fenêtre à part.
- **Menu des messages** : un clic droit sur une réponse de Claude propose Copier ; le menu montre des icônes.

### Tickets

- **Jira** : un bouton Jira ouvre les tickets du sprint dans leurs colonnes de statut, un onglet par projet Jira, avec des filtres rapides par personne, statut, type, priorité et epic. Le panneau du ticket montre son statut, son assigné et sa priorité.

### Artifacts

- **Fenêtre à part** : le panneau Artifacts s’ouvre dans une fenêtre séparée ; y pointer un élément le cite dans le message de l’agent.

### Pull requests

- **Relecture depuis l’agent** : Approuver et Demander des changements marchent aussi depuis la vue d’un agent.

### Corrections

- **Permissions** : les fenêtres Fichier et Kanban ne refusent plus les demandes de permission d’un agent tant qu’elles sont ouvertes.

## 0.20.0 · 2026-10-30

### Claude Code

- **/config et /mcp** : dans la conversation, /config et /mcp répondent en carte : les réglages de Claude Code à chercher et poser en un clic, et les serveurs MCP avec leur statut à autoriser, reconnecter, activer ou désactiver. Les autres sorties de commande gardent leurs retours à la ligne.

### Corrections

- **Revue de pull request** : Approve envoie la revue tout de suite, sans devoir ouvrir Finish review d'abord.

## 0.19.1 · 2026-10-30

### Corrections

- **Build de la release** : le build de la release tourne sous Node 24, sans avertissement de dépréciation.

## 0.19.0 · 2026-10-30

### Claude Code

- **/config** : dans la conversation, /config liste les réglages de Claude Code avec leur description, puis les valeurs de celui choisi. Tous les réglages ouvre Réglages › Claude Code.
- **/mcp** : /mcp liste les serveurs MCP de l’agent avec leur statut. Choisir un serveur qui demande une autorisation l’ouvre dans le navigateur, choisir un serveur en échec le reconnecte.
- **Réglages › Claude Code** : tous les réglages de Claude Code, à chercher et appliquer, ceux du terminal seul mis à part.

### Windows

- **Windows** : Squadron existe aussi pour Windows, en installeur qui se met à jour seul. Claude Code doit y être installé en natif (claude.exe).

## 0.18.0 · 2026-10-29

### Projets

- **Groupes** : les projets se rangent en groupes et sous-groupes. Un panneau Projets (Ctrl P) flotte au-dessus de l’application et les liste tous en arbre, avec glisser-déposer et sélection au Ctrl-clic.
- **Barre d’onglets** : elle garde les projets épinglés et ceux dont les agents ont été actifs dans un délai réglable. Un bouton de l’en-tête rassemble les pull requests à relire.
- **Réglages de projet** : une liste de projets avec une fiche par projet (Général, Agents, CI, Revues, Tickets). Les réglages du projet, depuis le menu d’un onglet ou du panneau Projets, ouvrent la fiche de ce projet.
- **Valeurs par défaut par projet** : un projet peut fixer le modèle, l’effort et le mode de permission de ses nouveaux agents. Les nouveaux agents, les tâches du Kanban, le pilote, les imports et les revues de pull request les utilisent et retombent sur les valeurs globales sinon.

### Recherche

- **Actions** : un verbe et un projet, comme « pr uh » ou « new nova », ouvrent ses pull requests ou le formulaire de nouvel agent. Un verbe seul liste tous les projets et Tab le complète. Kanban, Diff, Terminal, Éditeur, Archiver, Dupliquer, Interrompre, Épingler, Vue d’ensemble, chaque onglet des réglages, Personnaliser la disposition et Rechercher des mises à jour y sont aussi. La recherche d’agents et de conversations ne change pas.

### Corrections

- **Texte flou** : le texte n’est plus flou sous Linux.
- **PATH sous Linux** : Squadron lit aussi le PATH du shell de connexion, si bien que le claude de ~/.local/bin se lance et non un plus ancien.
- **Mentions @** : dans un dossier qui n’est pas un dépôt Git, @ liste les fichiers sans les dossiers cachés, de dépendances et de build, jusqu’à 20 000 chemins.
- **Lignes Bash** : une ligne Bash se déplie toujours et montre toute la commande, et survoler une ligne d’outil montre tout son argument.

## 0.17.0 · 2026-10-29

### Mode agent

- **Démarrer en agent** : une nouvelle session peut tourner en tant qu’agent de Claude Code (projet, utilisateur ou plugin), choisi dans le formulaire de création. Son modèle et son effort sont préremplis et restent modifiables. L’agent est fixé pour toute la session.
- **Pastille d’agent** : l’agent s’affiche à côté du titre de la session et dans la liste des agents. Un clic ouvre sa définition, en lecture seule : description, modèle, effort, skills, outils classés selon ce qu’ils permettent, et prompt avec un sommaire.
- **Pilote** : start_agent accepte un agent, et overview indique l’agent de chaque session.

## 0.16.3 · 2026-10-28

### Correctifs

- **Liste des tâches** : la liste des tâches d’un agent reste ouverte tant qu’il en reste à faire et se replie quand elles sont toutes terminées, au lieu de s’ouvrir et se replier à chaque tour, ce qui faisait bouger la conversation.
- **Maquettes** : un bloc html s’ouvre directement sur son aperçu une fois écrit, au lieu d’afficher un instant son HTML.
- **Messages d’une autre session** : ils s’affichent en retrait, avec un trait d’accent et l’étiquette « Depuis une autre session ».

## 0.16.2 · 2026-10-28

### Correctifs

- **Avis de Squadron** : l’avis qu’un agent reçoit quand vous mergez sa pull request ou publiez une relecture s’affiche en carte discrète de Squadron, et non plus comme si vous l’aviez écrit.

## 0.16.1 · 2026-10-28

### Kanban

- **Tâches dans les réponses** : un agent cite une tâche par un lien vers sa carte, qui s’affiche en puce dans une phrase ou en carte en tête d’un élément de liste, pour toutes les tâches, avec ou sans code. Une tâche dont le titre commence par un code (T52) est aussi reconnue quand l’agent écrit le code seul. La puce porte la couleur de la colonne de la tâche et ouvre sa fenêtre.

## 0.16.0 · 2026-10-28

### Artifacts

- **Artifacts en fichiers** : chaque agent écrit ses artifacts dans un dossier à lui, servi à part de l’app. Les pages exécutent leurs scripts, se rechargent à chaque sauvegarde et permettent de pointer un élément pour le citer ; elles partent avec leur agent.
- **Nettoyage** : les artifacts d’un agent archivé depuis un mois sont supprimés.

### Kanban

- **Tâches dans les réponses** : une tâche du Kanban qu’un agent cite en tête d’un élément de liste s’affiche en carte, avec sa colonne, son agent, sa pull request et son ticket Jira. Un clic ouvre une fenêtre avec son objectif, sa description, ses pièces jointes et des liens vers le Kanban, l’agent et la pull request.

### Correctifs

- **Messages entre agents** : un message envoyé par un autre agent s’affiche en carte à gauche avec son expéditeur, et non plus comme si vous l’aviez écrit.
- **Pull request d’un agent** : retrouvée depuis la branche qu’il a poussée ou le lien qu’il a donné.

## 0.15.0 · 2026-10-28

### Tickets

- **Jira** : connexions Jira dans Réglages › Tickets, projets Jira liés à chaque projet avec des suggestions tirées de git, une colonne Tickets dans le Kanban avec le sprint en cours, un panneau de ticket en lecture seule avec ses commentaires en fils, et une tâche créée depuis un ticket dont Haiku prépare les instructions.
- **Ticket dans l’en-tête** : un agent qui travaille sur une tâche créée depuis un ticket Jira affiche le ticket dans son en-tête, où il s’ouvre pour être relu.

### Pull requests

- **Retour à l’agent relecteur** : une pull request relue par un agent propose d’aller à cet agent depuis son menu, pour reprendre la conversation.
- **Relecture confiée à un agent** : une relecture en cours part à l’agent qui travaille sur la branche, ou à un nouvel agent dans un checkout de la pull request, au lieu d’être seulement publiée sur Bitbucket ou GitHub.
- **Commentaires lus par les agents** : les agents lisent les commentaires d’une pull request, en fils avec leurs réponses, avec read_pull_comments.

### Agents

- **Worktree supprimé** : un agent dont le worktree a été supprimé s’ouvre dans le dossier de son projet au démarrage suivant, au lieu d’afficher une session arrêtée.
- **Réponse prévue** : elle s’affiche sans flèche après elle.

## 0.14.0 · 2026-10-28

### Agents

- **Du plan au mode auto** : un plan peut être approuvé directement en mode auto. Une fois son plan approuvé, l’agent n’affiche plus le mode plan.
- **Réponse prévue** : après chaque tour, le champ de message vide affiche la réponse que Claude Code attend de vous. La flèche droite la reprend.
- **Messages entre agents** : le message d’un autre agent s’affiche comme une carte à gauche, avec son Markdown rendu et son expéditeur en haut.

### Pull requests

- Publier une review, approuver ou merger une pull request prévient les agents en cours qui travaillent dessus.

### Artifacts

- Le panneau liste les Claude Docs qu’un agent crée sur claude.ai. Un artifact Markdown peut y être publié pour le partager, avec son lien à ouvrir ou copier.

### Correctifs

- Les agents coupés par un redémarrage de mise à jour reprennent de nouveau au démarrage suivant.

## 0.13.1 · 2026-10-28

### Agents

- **Valeurs et secrets** : l'agent ne modifie plus le fichier pour le préparer. Squadron écrit chaque valeur à l'emplacement de son repère, ou ajoute la ligne en fin de fichier, et crée le fichier s'il n'existe pas. Le mode auto ne bloque plus la demande.

## 0.13.0 · 2026-10-28

### Agents

- **Reprise après un redémarrage** : un agent coupé par la fermeture ou la mise à jour de Squadron reprend là où il en était au démarrage suivant, avec les messages qu'il n'avait pas encore lus. Il ne repart pas s'il attendait votre réponse.
- **Agents inactifs** : le processus Claude Code d'un agent inactif depuis une heure s'arrête, et repart avec sa conversation au message suivant. Un travail en arrière-plan, un /loop ou Remote Control le garde actif.
- **Artefacts** : un agent peut publier un rapport, un plan ou une maquette dans un panneau à côté de sa conversation, où on peut le copier, l'enregistrer en fichier ou le supprimer.
- **Valeurs et secrets** : un agent peut demander des clés ou des secrets dans une carte de sa conversation, et Squadron les écrit dans le fichier sans que l'agent les voie.
- Une session arrêtée propose Continuer, qui demande à l'agent de reprendre là où il s'est arrêté.
- Le pilote peut épingler ou désépingler un agent avec set_pinned, et overview indique quels agents sont épinglés.

### Pull requests

- L'agent qui relit une pull request indique laquelle, et ses instructions se règlent pour tous les projets, par projet, ou via le pilote.

### Actions et pipelines

- Chaque action ou pipeline affiche le visage de la personne qui l'a lancé, avec son état dans une pastille sur l'avatar.
- Changer le dépôt d'un projet recherche aussitôt ses actions et pipelines.
- Une capture que vous avez jointe s'ouvre aussi dans la visionneuse de Squadron.

## 0.12.0 · 2026-10-27

### Actions et pipelines

- **Dans les outils de l'agent** : les actions et pipelines passent dans un bouton à droite des outils de l'agent, qui ouvre leur liste. Le marqueur sur l'onglet du projet disparaît.
- **Notification à la fin** : Squadron peut envoyer une notification système quand une action ou un pipeline se termine, à activer dans Réglages › Hébergement du code et à couper par projet dans Réglages › Projets.

## 0.11.0 · 2026-10-27

### Conversation

- **Commandes depuis la conversation** : une commande tapée après ! s'exécute dans le dossier de l'agent et s'affiche dans une carte bordée de vert ou de rouge, et Claude voit sa sortie. Un bloc bash dans une réponse a un bouton Lancer qui fait de même.
- **Mots de passe** : quand sudo, ssh ou git demande un mot de passe dans une commande d'un agent, Squadron le demande dans une fenêtre qui montre l'agent et la commande, et Claude ne le voit jamais.
- **Fichiers avec @** : taper @ dans un message ou dans le premier message d'un nouvel agent propose les fichiers et dossiers du projet.
- **Conversations sans dossier**, rangées dans un projet Conversations à part, accessible depuis Ajouter un projet et depuis l'écran vide.
- Une capture d'écran s'ouvre dans la visionneuse de Squadron, qui ne parcourt que les captures de la conversation.
- Claude met chaque commande terminal dans son propre bloc, et un bloc html affiche un état de chargement pendant son écriture.

### Projets

- **Actions et pipelines** : les GitHub Actions et Bitbucket Pipelines d'un projet s'affichent au-dessus de ses agents pendant qu'elles tournent, sur une ligne avec un panneau au survol, et une exécution terminée reste visible, en vert ou en rouge, le temps que vous choisissez.
- On peut coller ou joindre des captures et des fichiers en créant une tâche ou un nouvel agent.
- Un agent lancé depuis une tâche reçoit le titre de la tâche en tête de son prompt.
- Un agent terminé et pas encore lu ressort dans la liste avec un bord vert.
- La liste des agents ne se termine plus par le dossier du projet et sa branche Git.

### Squadron

- **Nouveautés** dans les Réglages liste les fonctions et correctifs de chaque version dans la langue de l'interface, et une notification l'affiche une fois après une mise à jour.

## 0.10.0 · 2026-10-27

### Conversation

- **Fichiers en cartes** : les fichiers joints s’affichent au-dessus du message avec leur type et leur taille, et un fichier que Claude vous remet apparaît sous la même carte dans sa réponse.
- **Propositions visuelles** : un petit bloc html dans une réponse est dessiné dans la conversation avec les polices et les couleurs de Squadron, sans qu’aucun script n’y tourne.
- Le message que Claude écrit avant de poser une question reste affiché au-dessus, au lieu d’être replié avec les étapes.
- Une pull request citée dans une phrase a sa carte complète sous le paragraphe, et la phrase garde une courte pastille #numéro.
- Un fichier déposé sur une conversation est seulement joint : Ajouter un projet s’ouvre pour un dossier déposé hors d’une conversation.

### Pull requests

- **Fichiers vus** dans une relecture : un fichier coché se replie, la liste des fichiers compte ceux qui sont vus, et un fichier modifié ensuite revient en Modifié.
- La relecture propose Fusionner sur vos propres pull requests et Approuver sur celles des autres ; Fusionner n’est plus proposé sur les pull requests que vous relisez seulement.
- Relire passe avant Fusionner sur vos pull requests.

### Squadron

- Les questions et les avis, comme pour une mise à jour, une fusion ou une suppression, s’affichent dans une boîte de dialogue Squadron.
- Changer de projet ramène à ses conversations.
- Les onglets de projet ont un contour plus net, et un onglet qui sonne pour vous un contour plein qui luit.
- Un sous-agent relancé a sa propre progression et sa propre fin.

## 0.9.0 · 2026-10-27

### Pull requests

- **Cartes de pull request** : une pull request GitHub ou Bitbucket donnée en lien dans la réponse d’un agent s’affiche en carte avec son état en direct, ses checks, ses approbations, ses commentaires et sa taille, et les boutons Relire, Ouvrir avec Squadron et Ouvrir sur GitHub ou Bitbucket. Un lien au milieu d’une phrase devient une pastille qui montre la carte au survol.
- **Relire vos propres pull requests**, depuis Relire ou sous la flèche de Fusionner ; la relecture ne fait que commenter, puisque les hébergeurs refusent qu’on approuve la sienne.
- **Vue de la branche** dans le panneau Diff de l’agent : tout ce que la branche change depuis qu’elle a quitté la branche principale, commits et travail non commité ensemble, à relire et commenter avant une pull request.
- Ouvrir un commentaire dans une pull request ne fait défiler que le diff, et plus toute la fenêtre.

### Captures d’écran

- Les captures d’écran renvoyées par les outils pendant qu’un agent travaille s’affichent en vignettes au bout de la ligne des étapes ; au-delà de quatre, +N les déplie toutes, et un clic les ouvre en grand.
- Les agents peuvent montrer une capture dans leur réponse depuis le dossier de captures de Squadron. Les captures sont supprimées au bout d’une semaine.

### Conversation

- Ce qui précède le dernier compactage est replié derrière une ligne, avec Afficher pour le déplier.
- Les questions auxquelles vous avez répondu restent dépliées ; les réponses d’une question s’empilent l’une sous l’autre.
- L’éditeur affiche les fichiers Markdown et HTML mis en forme, avec un bouton Aperçu.
- Un message venu d’une autre session Claude n’affiche que son texte.

### Onglets et liens

- Un onglet de projet perd sa marque rouge, et son compte d’attente, une fois que vous avez regardé ses agents en échec.
- Les liens web s’ouvrent dans un nouvel onglet du navigateur déjà ouvert, et plus dans une nouvelle fenêtre.
- Un agent qui entre dans un worktree avec cd affiche la branche de ce worktree.

## 0.8.0 · 2026-10-26

### Contexte et coût

- **Jauge de contexte** dans l’en-tête de l’agent, comme dans Claude Code : 65k / 1M · 7 %, l’endroit où démarre l’auto-compactage, et un bouton Compacter maintenant.
- **Taille de compactage** dans Réglages › Préférences : compacter vers 600k, 400k ou 200k au lieu des 967k de Claude Code.
- **Garde-fou des conversations froides** : écrire à une grosse conversation inactive depuis plus d’une heure propose de la compacter d’abord, puisque son cache a expiré.

### Agents

- **Sélectionner plusieurs agents** avec Ctrl et un balayage, puis les archiver, les épingler, les marquer comme lus ou non lus, ou les supprimer ensemble.
- **Tout chercher** avec Ctrl+Espace : agents, projets et conversations, archivés compris.
- **Badge du modèle** : Opus 5.5 · Extra, avec les niveaux d’effort nommés comme dans l’app Claude.
- Les échos des commandes slash, comme la sortie de /compact, ne s’affichent plus comme vos messages.
- Une session qui est seulement entrée dans un dossier avec cd ne passe plus pour un worktree.

### Pull requests

- La page occupe toute la largeur, avec un bouton par ligne et les autres actions sous sa flèche.
- La ligne de pull request d’une carte d’agent s’aligne sur les autres.
- Les cartes de question s’affichent proprement.

## 0.7.0 · 2026-10-26

### Squadron en français

- **L’interface parle votre langue** : elle suit la langue du système, ou celle choisie dans Réglages › Préférences. Le français est la première traduction, sur toute l’interface.

### Agents

- **Agents non lus** : un agent qui termine pendant que vous regardez ailleurs met une coche sur l’onglet de son projet et un point sur sa carte jusqu’à ce que vous l’ouvriez. Clic droit sur une carte pour la marquer comme lue ou non lue.
- **Un onglet qui sonne** : un onglet de projet tremble tant qu’un de ses agents vous attend, et peut aussi carillonner (Réglages › Préférences).
- **Les sous-agents en arrière-plan** sont listés et suivis, et leur agent reste au travail jusqu’à ce qu’ils finissent.
- **Progression et panneau** : un agent peut afficher une barre de progression sur sa carte, et remplir un panneau sous l’en-tête de sa conversation avec sa propre mise en page, rendue dans un cadre isolé que vous pouvez masquer.
- **Les agents épinglés** se placent au-dessus d’un séparateur dans la liste.
- **Les coûts disparaissent** de la liste des agents, de la conversation et de la vue d’ensemble.
- **Précédent et suivant** entre les vues avec les boutons de la souris ou Alt+← et Alt+→.

### Pull requests

- **Approuver et fusionner** une pull request depuis Squadron, quand son hébergeur le permet.
- **Déplier le diff** autour de chaque bloc, ou afficher le fichier entier.
- **Demander à un agent** de relire vous laisse sur les pull requests : un bandeau indique qu’il relit et ouvre sa conversation à la demande.
- Le rail de la conversation affiche une marque par message, regroupées quand elles ne tiennent plus.

## 0.6.0 · 2026-10-26

### Projets et onglets

- **Choisir la couleur en ajoutant un projet**, avec les mêmes pastilles que dans ses réglages.
- **Ordonner les onglets de projet** dans Réglages › Projets : à la main, par nom ou par dernière activité. Les projets épinglés restent en tête. Glissez un onglet dans l’en-tête pour le déplacer ; glisser pendant un tri par nom ou par activité passe à l’ordre à la main.
- **Un projet sans agent** ouvre l’Éditeur, le Terminal, le Diff, les Pull requests et le Kanban.

### Agents

- **Glisser pour réordonner** : les agents de la liste (tri « À la main »), les boutons d’outils et les pastilles d’un agent, sans passer par le mode personnalisation.
- **Les longues conversations s’ouvrent vite** : seuls les 20 derniers messages sont affichés, les précédents se chargent quand vous remontez. Le rail regroupe ses marques quand elles ne tiennent plus.
- **La réflexion dans la conversation** : Réglages › Préférences › Réflexion › « Dans la conversation » affiche les pensées de Claude en message plus discret entre ses actions.

### Correctifs

- Les animations serpent et pulsation sont synchronisées partout, onglets de l’en-tête compris.
- L’application s’appelle « Squadron » dans le menu des applications et sur macOS, et le menu Squadron indique macOS sur un Mac.

## 0.5.2 · 2026-10-23

### Correctifs

- Les conversations qui commencent par du texte collé apparaissent désormais dans Importer des conversations, et gardent ces messages une fois importées.

## 0.5.1 · 2026-10-23

### Correctifs

- La version macOS : la boîte de dialogue des réglages n’entre plus en conflit avec le module des réglages sur un disque insensible à la casse.

## 0.5.0 · 2026-10-23

### macOS

- **Squadron tourne sur macOS**, Apple Silicon et Intel. Installez-le avec le même script d’une ligne, qui le place dans `/Applications`, ou téléchargez le `.dmg`. Il n’est pas notarié par Apple : si macOS refuse d’ouvrir la version `.dmg`, cliquez une fois sur **Ouvrir quand même** dans Réglages Système › Confidentialité et sécurité.
- Sur macOS, le jeton Bitbucket va dans le trousseau, la barre d’état affiche le processeur et la mémoire, les éditeurs de /Applications apparaissent dans Ouvrir avec, et les conversations de l’app Claude peuvent être importées.
- Les mises à jour arrivent sur Mac de la même façon : **Mettre à jour** dans la barre d’état.

### Correctifs

- Un `cd` dans un sous-dossier ne passe plus pour un worktree : Squadron ne bascule que vers un dossier qui est un checkout git à part entière.

## 0.4.0 · 2026-10-23

### Pull requests

- **Relire page par page, fichier après fichier** : la relecture d’une pull request affiche tous les fichiers l’un après l’autre, chacun avec ses conversations sur leurs lignes. L’arborescence suit le défilement et saute à un fichier ; l’en-tête de chaque fichier reste en haut et se replie.
- **Nouveaux changements depuis votre relecture** : une pull request que vous avez relue remonte en tête quand sa branche bouge, et sa relecture s’ouvre sur ce qui a changé depuis, avec Tous les changements à un clic.
- **Statuts selon qui doit agir** : pleins quand c’est à vous (relire, relire à nouveau, corriger, fusionner), atténués quand vous attendez les autres. Chaque pull request affiche l’avatar de son auteur.
- Une relecture laissée en commentaire compte comme relue.
- Les relectures en brouillon survivent à un redémarrage de Squadron.

### Agents

- **Les agents peuvent écrire à d’autres agents** : avec le nouvel outil de pilotage `message_agent`, un agent demande à envoyer une instruction à un autre agent ; vous approuvez chaque message.
- Chaque agent garde son message non envoyé, ses pièces jointes et ses commentaires de diff quand vous changez de vue, et d’un redémarrage à l’autre.

### Correctifs

- Le menu de tri ne déborde plus de la fenêtre.
- Le bouton de repli de la liste des agents se place à côté du nom du projet.

## 0.3.0 · 2026-10-23

### Importer des conversations

- **Faire venir des conversations tenues ailleurs** : le bouton tout à droite de l’en-tête de la liste des agents liste les conversations du projet et de ses worktrees, depuis l’app Claude ou Claude Code, avec leur titre, leur date et leur taille. Filtrez-les, sélectionnez-les et importez-les.
- Chacune devient un agent qui poursuit la conversation dans une copie : l’originale reste intacte et continue de fonctionner dans l’app Claude.
- Les agents peuvent le faire aussi, avec le nouvel outil de pilotage `import_session`.

### Mises à jour

- **Rechercher des mises à jour** depuis le menu Squadron ou Réglages › À propos. Le menu affiche maintenant la vraie version.

### Interface

- L’en-tête de la liste des agents met le nom du projet sur sa propre ligne, avec le nombre d’agents et les boutons en dessous.

## 0.2.0 · 2026-10-23

### Pull requests

- **La pull request de chaque agent** : une pastille à côté de la branche montre son état, chaque relecteur avec son avatar et son verdict, combien ont approuvé, les builds et les commentaires. Bitbucket fonctionne avec un jeton d’API gardé dans le trousseau du système (Réglages › Hébergement du code), GitHub avec la CLI gh.
- **Relire dans le panneau Diff** : le diff de la pull request calculé depuis votre checkout, les fils des relecteurs sur leurs lignes, les réponses, et vos commentaires sur des lignes ou sur toute la pull request. Rassemblez-les dans une relecture, puis envoyez-la à l’agent ou publiez-la avec Commenter, Approuver ou Demander des changements.
- **Page Pull requests** pour chaque projet : celles qui attendent votre relecture et les vôtres, avec leur avancement. Un badge sur l’onglet du projet et un bouton dans la Vue d’ensemble signalent quand l’une a besoin de vous.
- **Demander à un agent** : un agent relit une pull request dans son propre worktree et dépose ses remarques dans votre brouillon de relecture. Rien n’est publié sans vous.

### Conversations

- **Captures d’écran et fichiers** : collez-les avec Ctrl+V, déposez-les ou joignez-les. Claude voit les images et lit les autres fichiers là où ils sont. Les captures collées disparaissent avec leur agent.
- **Nouvel agent** : tous les champs sont facultatifs. Écrivez le premier message dans la conversation, avec captures d’écran et fichiers.
- **Blocs de code** : coloration syntaxique pour tous les langages et un bouton Copier. Les blocs Markdown s’affichent rendus, avec leur source à un clic. Claude met le texte à copier dans un bloc plutôt que dans un fichier.
- Les avis de Claude Code s’affichent dans un bloc discret, et les agents au travail montrent une étape lisible au lieu des commandes brutes.

### Diff

- Les fichiers modifiés en arborescence de dossiers, avec un filtre.

## 0.1.2 · 2026-10-23

Ajoute un paquet .deb : il utilise le WebKit du système, ce qui corrige la fenêtre grise sur les Ubuntu récents.

## 0.1.1 · 2026-10-23

Compile sur Ubuntu 24.04 : corrige la fenêtre vide sur les systèmes récents.

## 0.1.0 · 2026-10-23

Première version.
