# Nouveautés

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
