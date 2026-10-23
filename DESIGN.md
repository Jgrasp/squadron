# Squadron — Design

Interface en anglais. Source : page « Cuivre » du canvas Claude Design « Squadron — 10 directions » (écrans C1 à C25). Thème sombre unique, brun chaud et cuivre. Les valeurs ci-dessous sont relevées dans la maquette ; elles vivent en variables CSS dans `src/app.css`.

## Couleurs

### Fonds

| Token | Valeur | Usage |
|---|---|---|
| `--bg-app` | `#141110` | Fenêtre, en-tête |
| `--bg-sidebar` | `#171311` | Liste des agents, panneau Diff |
| `--bg-main` | `#1C1816` | Zone de conversation, dialogues |
| `--bg-rail` | `#191513` | Colonne de repères |
| `--bg-tabs` | `#1E1A18` | Conteneur des onglets de projet |
| `--surface-1` | `#221D1A` | Bloc de réponse de l'agent, encart d'infos |
| `--surface-2` | `#26211E` | Champs, barre de saisie, boutons secondaires |
| `--surface-3` | `#2E2724` | Code en ligne, puces, pastilles |
| `--surface-4` | `#332C28` | Boutons icône |
| `--surface-5` | `#433A35` | Bouton actif ou ouvert |
| `--menu` | `#2A2421` | Menus, info-bulles |
| `--menu-hover` | `#36302C` | Élément de menu survolé |
| `--overlay` | `rgba(12,9,8,0.74)` | Voile sous un dialogue |

### Bordures

| Token | Valeur |
|---|---|
| `--border` | `#2B2522` (séparateurs de zones) |
| `--border-strong` | `#3D3531` (boutons fantômes, menus) |
| `--border-dashed` | `#4A3F39` (« + Agent », dépôt de dossier) |
| `--border-focus` | `#8E7A6E` (champ actif, 2px intérieur) |

### Texte

| Token | Valeur | Usage |
|---|---|---|
| `--text` | `#EDE6E1` | Texte courant |
| `--text-strong` | `#FFFFFF` | Onglet actif, titres sur fond coloré |
| `--text-2` | `#D8CDC6` | Texte des boutons fantômes |
| `--text-3` | `#CFC5BE` | Chemins, texte secondaire lisible |
| `--text-muted` | `#B3A79F` | Résumés repliés, onglets inactifs |
| `--text-dim` | `#A89C94` | Métadonnées, légendes |
| `--text-faint` | `#6E625B` | Repères inactifs, icônes éteintes |

### Projets

Chaque projet reçoit à l'ajout la couleur la moins utilisée de cette palette, gardée dans `projects.json` : `#A78BFA`, `#2DD4BF`, `#F472B6`, `#60A5FA`, `#A3E635`, `#FACC15`. Elle colore le carré du projet (onglets, liste des agents, Overview, Kanban, dialogue New agent), le fond et le contour de l'onglet actif, et l'effet de l'onglet qui travaille (C14).

### Accent cuivre et « t'attend »

Le cuivre signale ce qui attend l'utilisateur : permission, question, plan, badge.

| Token | Valeur | Usage |
|---|---|---|
| `--accent` | `#E08A5B` | Bouton principal d'une carte, pastille d'attente, badge, repère actif |
| `--accent-ink` | `#24120A` | Texte sur `--accent` |
| `--accent-soft` | `#F7C7AA` | Texte d'état « Attend… », anneau de sélection |
| `--accent-link` | `#F0A57A` | Liens (`#F7C7AA` au survol) |
| `--wait-bg` | `#3B2216` | Carte en attente, agent en attente dans la liste |
| `--wait-border` | `#8E4E2E` | Contour 1px de ces cartes |
| `--wait-text` | `#FFF3EC` | Titre dans une carte en attente |
| `--wait-text-2` | `#F7DCCB` | Détail (commande, chemin) |
| `--wait-muted` | `#E7C3AE` | Aide, descriptions d'options |
| `--wait-btn` | `#57301F` | Bouton secondaire (« Toujours ») |
| `--wait-btn-text` | `#FFE0CF` | Texte de ce bouton |
| `--wait-note` | `#2E1B12` | Encart « Toujours », groupe de question |
| `--option` | `#4A2B1C` | Option de question |
| `--option-on` | `#6B3A24` | Option choisie (+ anneau 2px `--accent-soft`) |
| `--option-off` | `#3A2318` | Option non choisie après réponse |

### États

| Token | Valeur | Usage |
|---|---|---|
| `--ok` | `#9AD6CB` | Agent qui travaille (anneau tournant), lignes ajoutées |
| `--ok-text` | `#BFE7DF` | Texte ajouté, « Toujours autorisé » |
| `--ok-line` | `#6E9E95` | Numéros des lignes ajoutées |
| `--ok-bg` | `rgba(154,214,203,0.10)` | Fond d'une ligne ajoutée (`0.07` dans le panneau Diff) |
| `--done` | `#B8D98C` | Coche d'un agent fini, `code 0` (`#C9E3A8`) |
| `--err` | `#F59E8C` | Croix d'erreur, lignes supprimées |
| `--err-text` | `#F8BDB1` | Texte d'erreur |
| `--err-muted` | `#E7C3BA` | Explication d'une erreur |
| `--err-bg` | `#2B1A17` | Carte d'erreur, appel d'outil en échec |
| `--err-border` | `#8A4A40` | Contour d'erreur (`#6B3A33` pour un outil) |
| `--err-ink` | `#2A0D08` | Texte sur `--err` |
| `--err-pre` | `#1A1210` | Sortie d'erreur |
| `--del-bg` | `rgba(245,158,140,0.10)` | Fond d'une ligne supprimée |
| `--del-line` | `#A7756C` | Numéros des lignes supprimées |

### Bulle utilisateur et bouton principal

Fond clair `--text` (`#EDE6E1`), texte `#1A1513`. La bulle a le coin bas droit pincé : `border-radius: 6px 6px 2px 6px`.

## Typographie

- **DM Sans** pour le texte, **DM Mono** pour le code, les chemins, les commandes, les coûts et les compteurs. Polices embarquées, pas de Google Fonts.
- Taille de base 13,5px.
- Échelle : 11 (légendes de menu, compteurs), 11,5 (métadonnées en mono, badges), 12 (sous-titres, infos), 12,5 (états, résumés, aides), 13 (texte d'erreur, code en carte), 13,5 (base), 15 (titre d'agent, question), 15,5 (titres Markdown), 18 (titre de dialogue), 22–24 (états vides).
- Graisses : 500 (boutons secondaires), 600 (titres, noms), 700 (boutons principaux, en-têtes de dialogue).
- Interlignage : 1,5 (bulles, aides), 1,6 (prose), 1,65 (Markdown de l'agent), 20px fixe pour les lignes de diff.
- Marque « SQUADRON » : 12px, 700, capitales, `letter-spacing: 0.12em`, précédée d'un triangle cuivre de 14px.

## Formes et espacements

- Rayons : 2px (barres, petites pastilles), 3px (code en ligne, badges), 4px (boutons, champs, lignes), 6px (cartes, blocs, conteneur d'onglets), 8px (dialogues).
- Contours en `box-shadow` (intérieur 1px), pas en `border`, sauf séparateurs de zones.
- Espacements : 4, 6, 8, 10, 12, 14, 16, 18, 20, 24px.
- Ombres : menus `0 14px 36px rgba(0,0,0,0.5)`, dialogues `0 30px 80px rgba(0,0,0,0.6)`, agent sélectionné `0 8px 24px rgba(201,105,58,0.18)`.

## Disposition

- Fenêtre sans décoration système, largeur minimale 1000px.
- **En-tête** (54px, `--bg-app`, grille `1fr auto 1fr`) : menu Squadron à gauche, onglets de projet au centre, boutons de fenêtre (24px) à droite. L'en-tête sert de poignée de déplacement.
- **Liste des agents** (304px, `--bg-sidebar`, padding 12px).
- **Zone principale** (`--bg-main`) : en-tête d'agent 60px, conversation, barre de saisie.
- **Conversation** : colonne centrée de 740px max, padding 20px, espacement 14px entre blocs.
- **Colonne de repères** : 40px à gauche de la conversation, `--bg-rail`.
- **Panneau Diff** : 680px à droite, la conversation garde 360px minimum.

## Composants

### Onglets de projet
Conteneur `--bg-tabs`, padding 4px, rayon 6px, espacement 4px. Onglet 30px de haut, rayon 4px, texte `--text-muted`. Onglet actif : texte blanc 600, fond et halo cuivre (`rgba(224,138,91,0.16)`, contour intérieur 0.75, halo 18px 0.28). Badge du nombre d'agents qui attendent : 20px, `--accent` sur `--accent-ink`, 11,5px 700. Au-delà de la place disponible, un bouton « +N » ouvre un menu avec recherche. Bouton « + » pour ajouter un projet. Projet dont le dossier a disparu : icône d'alerte dans l'onglet.

### Liste des agents
- En-tête : nom du projet (700) et « N agents · N t'attend » (12px `--text-dim`), bouton « + Agent » (32px, contour pointillé).
- Agent : padding 12px 14px, rayon 6px, espacement 6px. Ligne 1 : icône d'état, nom (600), coût (mono 12px). Ligne 2 : état ou dernière action (mono 11,5px, une ligne, ellipse).
- Icônes d'état : anneau `--ok` qui tourne (travaille), point `--accent` avec halo 4px (attend), coche `--done` (fini), croix `--err` (erreur).
- Agent qui attend : fond `--wait-bg`, contour `--wait-border`, état en `--accent-soft` 600. Agent sélectionné : anneau 2px `--accent-soft` et ombre cuivrée. Agent en erreur sélectionné : fond `--err-bg`, anneau `--err`.
- Pied : encart `--surface-1` avec le chemin (mono), « Git · branche » ou « Pas de dépôt Git détecté », coût du projet.

### En-tête d'agent
Nom 15px 600, puis « modèle · effort · mode · coût · tokens » en 12px `--text-dim` (coût et tokens en mono). À droite, le bouton Diff (32px, contour `--border-strong`, compteur `+N` en `--ok`, `−N` en `--err`).

### Conversation
- **Message utilisateur** : bulle claire alignée à droite, 540px max, padding 12px 16px.
- **Réponse de l'agent** : bloc `--surface-1`, padding 16px 18px, rayon 6px, espacement 12px. Markdown en interlignage 1,65 ; code en ligne `--surface-3`, mono 12,5px, padding 1px 6px.
- **Résumé replié** (réflexion, groupe d'actions) : 12,5px `--text-muted`, chevron dans un carré 18px `--surface-3`. « A réfléchi N s ». « N actions · 2 lectures, 1 modification, 1 commande ».
- **Appel d'outil** : ligne 30px, nom de l'outil sur 40px en `--text-dim`, argument principal en mono, statut à droite. Edit et Write ne se déplient pas : un clic ouvre le panneau Diff sur le fichier (« Diff → », `+N −N`). Bash se déplie sur sa sortie. Outil en échec : fond `--err-bg`, contour `--err-border`, sortie `--err-text` sur `--err-pre`.
- **Fin de tour** : ligne mono 11,5px `--text-dim`, « Tour N terminé · $x · N k entrée · N k sortie · durée ». Pendant le tour : « Tour en cours · … ».
- **Repères d'historique** (permission répondue) : ligne 12,5px, padding 9px 14px. Autorisé : fond `#1F2321`, contour `#33403B`, libellé `--ok-text`. Refusé : fond `#241917`, contour `#4A2E29`, libellé `--err-text`. Détail en dessous (règle ajoutée, consigne de refus) en 12px `--text-dim`.
- **Interruption** : encart `--surface-2`, contour `#4E443E`, « Interrompu par toi · heure », puis « Tour interrompu ». **Rechargement** : séparateur horizontal « Session rechargée · heure · conversation conservée ».
- **Bouton « Dernier message »** : pilule claire flottante en bas de la conversation, rayon 18px, quand on a remonté.
- **Colonne de repères** : un trait par message de l'utilisateur, les traits groupés et centrés verticalement dans la colonne (ils se resserrent quand il y en a beaucoup). Inactif : 10×2px `--text-faint` ; actif (message affiché) : 18×3px `--accent` avec halo. Au survol, info-bulle `--menu` de 300px à hauteur du trait : « Message N · heure » puis le texte. Au clic, défilement jusqu'au message.

### Cartes qui attendent l'utilisateur
Fond `--wait-bg`, contour `--wait-border`, padding 16–18px, rayon 6px, espacement 12px. Icône 30px `--accent` sur `--accent-ink`. Titre `--wait-text` 600, détail en mono `--wait-text-2`.
- **Permission** : « Exécuter une commande ? », « Modifier un fichier ? » (avec le diff, 200px max), « Utiliser un outil ? » (paramètres en liste clé/valeur). Boutons 36–38px : Autoriser (`--accent`), Toujours (`--wait-btn`), Refuser (`--surface-3`). Encart `--wait-note` qui dit ce que « Toujours » autorise.
- **Question** : options en boutons pleine largeur (fond `--option`, touche numérotée 22px, titre 600, description `--wait-muted`, badge « Recommandé »). Plusieurs questions : un groupe `--wait-note` par question, le groupe en cours en `--option` avec contour `#A55E37` ; choix multiples en cases à cocher sur deux colonnes ; « Envoyer les réponses » actif quand tout est répondu.
- **Plan** : plan en Markdown dans un encart `--bg-main`, boutons « Lancer · accepter les modifications », « Lancer · demander à chaque modification », « Continuer à planifier ».

### Carte d'erreur
Fond `--err-bg`, contour `--err-border`, icône `--err`. Titre blanc, contexte `--err-muted`, message brut en `pre` mono 12px `--err-text` sur `--err-pre`. Actions : Recharger (bouton clair), Copier l'erreur.

### Barre de saisie
Conteneur `--surface-2`, rayon 6px, padding 8px 8px 8px 18px, colonne de 740px. Champ sans bordure, boutons icône 38px (Interrompre, menu ⋯), bouton Envoyer clair. Le menu ⋯ (270px, `--menu`) contient Recharger et Archiver, chacun avec une ligne d'explication en 12px.

### Panneau Diff
680px, `--bg-sidebar`, en-tête 60px (« Diff », « Modifications de X · N fichiers », fermer). Un onglet par fichier modifié par l'agent (28px, mono 12px, `+N`). Fichier : en-tête `--surface-1` avec badge NOUVEAU/MODIFIÉ, chemin, « créé par Write · N lignes ». Lignes en grille (numéro 40px, signe 16px, texte), mono 12px, interligne 20px. Pied : « Ouvrir dans l'éditeur ».

### Dialogues
Voile `--overlay`, boîte `--bg-main`, rayon 8px, contour `--border-strong`. Titre 18px 700, aide 13px `--text-dim`. Champs 42px `--surface-2`, contour 1px `--border-strong`, 2px `--border-focus` au focus ; chemins en mono. Boutons en bas à droite (Annuler `--surface-2`, action principale claire), séparés par un trait `--border`. Erreur de champ : texte `--err-text` avec icône, bouton principal désactivé.
- **Ajouter un projet** : dossier (lecture seule + « Parcourir… »), « Dépôt Git détecté · branche X » en `#C9E3A8`, nom prérempli.
- **Nouvel agent** : nom, modèle (Opus/Sonnet/Haiku) et effort (low, medium, high, xhigh, max) en segments, mode de permission en 4 cartes (Demander tout, Accepter les modifications, Plan, Auto), premier message. « Ctrl ↵ pour lancer ».

### États vides
Centrés, 440–460px. Aucun projet : triangle cuivre, titre 24px, texte `--text-muted`, bouton « Ajouter un projet », zone de dépôt en pointillé. Projet sans agent : « Lance le premier agent de X », bouton « Nouvel agent » et raccourci `Ctrl N`. Dossier introuvable : carte d'alerte avec « Choisir le nouvel emplacement… », « Réessayer », « Retirer le projet ».

## Mouvement
Anneau de l'agent qui travaille : rotation 1,4s linéaire. Curseur de streaming : clignotement 1s. Tout s'arrête sous `prefers-reduced-motion`.

## Accessibilité
Vrais `button`, `a`, `input` et `label`. `aria-label` sur chaque bouton icône. Focus clavier visible : anneau 2px `--accent-soft`. Les cartes d'attente sont des `section` étiquetées, les erreurs des `role="alert"`.

## Hors V1
Présents dans la maquette, non implémentés : Overview, Forker, progression « N/M tâches », menu Squadron au-delà de « Claude Code » et « Quitter », « Dossier proche » (C24), « Réordonner les onglets » (C25).
