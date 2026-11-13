# Squadron — Design

Source : page « Cuivre » du canvas Claude Design « Squadron — 10 directions » (écrans C1 à C25), puis ce que l'app a livré depuis. Thème sombre unique, brun chaud et cuivre. Les valeurs vivent en variables CSS dans `src/app.css` : ce fichier fait foi, ce document dit à quoi elles servent. Interface traduisible (`src/lib/i18n`, `en` de base + `fr`) : toute nouvelle chaîne a ses deux clés.

## Règles

À relire avant toute maquette, artifact, bloc html ou retouche de style.

- Uniquement des tokens de `app.css`. Une couleur en dur n'est admise que si elle figure dans « Exceptions en dur » plus bas ; `scripts/design.test.ts` le vérifie.
- Le cuivre (`--accent`, `--wait-*`, `--option*`) signale ce qui attend l'utilisateur : permission, question, plan, badge. Jamais pour une action ordinaire.
- Bouton principal : clair, `--text` sur `--ink`, 700 (`.btn-primary`). Pas cuivre.
- `--background*` est le bleu des tâches de fond, pas un fond de surface. L'encre sur cuivre est `--accent-ink`.
- Rayons : 6px les cartes, 8px les dialogues et les cartes de conversation à barre de ton (PullCard, FileCard, LinkCard), 4px boutons et champs.
- Contours en `box-shadow: inset 0 0 0 1px …`, pas en `border`, sauf séparateurs de zones.
- Partir du CSS du composant existant le plus proche (fichier cité dans « Composants ») au lieu d'inventer. Réutiliser `StatusIcon`, `ModeIcon`, `Icon`, `Avatar`, `SidePanel`, `Dialog`, `Confirm`, `Select` plutôt que les redessiner.
- Utiliser les classes de `app.css` (`.btn*`, `.field`, `.option-*`, `.effect-*`, `.sr-only`) sans les redéfinir.

## Maquettes et artifacts

- **Bloc ```html dans la conversation** : rendu dans une iframe qui reçoit le `:root` et les `@font-face` de l'app (`Markdown.svelte`, `themeCss`). Les tokens et DM Sans / DM Mono sont là ; les classes (`.btn`, `.field`…) non : recopier leurs règles depuis `app.css`. Fond `--bg-main`, 13,5px/1,5.
- **Artifact** (`$SQUADRON_ARTIFACTS`, iframe `artifact://`) : `<link rel="stylesheet" href="/__squadron/theme.css">` dans le `<head>` donne tout `app.css` (tokens, classes, keyframes) et DM Sans / DM Mono, servis par `artifacts.rs`. Ne pas recopier les tokens à la main. Fond de page `--bg-app` par défaut, poser `--bg-main` sur la zone maquettée.
- Maquetter à la taille réelle : largeurs de « Disposition », conversation à 740px.
- Plusieurs variantes : les poser côte à côte sur la même page, chacune titrée, mêmes données.
- Chaînes de la maquette en anglais (langue de base de l'interface).

## Couleurs

### Fonds

| Token | Valeur | Usage |
|---|---|---|
| `--bg-app` | `#141110` | Fenêtre, en-tête, corps de l'éditeur |
| `--bg-sidebar` | `#171311` | Liste des agents, panneaux de droite, colonnes du Kanban, barre d'état |
| `--bg-main` | `#1C1816` | Conversation, dialogues, Kanban, PullCard, FileCard, LinkCard |
| `--bg-rail` | `#191513` | Colonne de repères |
| `--bg-tabs` | `#1E1A18` | Conteneur des onglets de projet |
| `--surface-1` | `#221D1A` | Bloc de réponse, carte Kanban, ligne de PR, agent sélectionné |
| `--surface-2` | `#26211E` | Champs, barre de saisie, boutons secondaires, segments |
| `--surface-3` | `#2E2724` | Code en ligne, puces, pastilles, boutons de carte |
| `--surface-4` | `#332C28` | Boutons icône, onglet ou segment actif |
| `--surface-5` | `#433A35` | Bouton actif ou ouvert |
| `--menu` | `#2A2421` | Menus, info-bulles, puces de contexte |
| `--menu-hover` | `#36302C` | Élément de menu survolé |
| `--overlay` | `rgba(12,9,8,0.74)` | Voile sous un dialogue |

### Bordures

| Token | Valeur |
|---|---|
| `--border` | `#2B2522` (séparateurs de zones) |
| `--border-strong` | `#3D3531` (boutons fantômes, menus, cartes) |
| `--border-dashed` | `#4A3F39` (« + Agent », « + Tâche », dépôt) |
| `--border-focus` | `#8E7A6E` (champ actif, 2px intérieur) |

### Texte

| Token | Valeur | Usage |
|---|---|---|
| `--text` | `#EDE6E1` | Texte courant, fond du bouton principal et de la bulle |
| `--text-strong` | `#FFFFFF` | Titres de carte, onglet actif |
| `--text-2` | `#D8CDC6` | Texte des boutons fantômes |
| `--text-3` | `#CFC5BE` | Chemins, texte secondaire lisible |
| `--text-muted` | `#B3A79F` | Résumés repliés, onglets inactifs |
| `--text-dim` | `#A89C94` | Métadonnées, légendes |
| `--text-faint` | `#6E625B` | Repères inactifs, icônes éteintes |
| `--ink` | `#1A1513` | Texte sur fond clair (bouton principal, bulle) |

### Projets

Chaque projet reçoit à l'ajout la couleur la moins utilisée de cette palette, gardée dans `projects.json` : `#A78BFA`, `#2DD4BF`, `#F472B6`, `#60A5FA`, `#A3E635`, `#FACC15`. Exposée en `--project` sur l'élément. Elle colore le carré du projet (onglets, liste, Overview, Kanban, New agent), l'onglet actif (fond 16 %, contour intérieur 75 %, halo 18px 28 %), le contour des onglets inactifs (55 %), le liseré de l'onglet actif de l'éditeur et l'effet de l'onglet qui travaille.

### Accent cuivre et « t'attend »

| Token | Valeur | Usage |
|---|---|---|
| `--accent` | `#E08A5B` | Bouton principal d'une carte d'attente, pastille d'attente, badge, repère actif, « Répondre » |
| `--accent-ink` | `#24120A` | Texte sur `--accent` |
| `--accent-soft` | `#F7C7AA` | Texte « Attend… », anneau de focus et de sélection |
| `--accent-link` | `#F0A57A` | Liens (`--accent-soft` au survol) |
| `--wait-bg` | `#3B2216` | Carte en attente, agent ou tâche en attente |
| `--wait-border` | `#8E4E2E` | Contour 1px de ces cartes, bouton Pull requests |
| `--wait-text` | `#FFF3EC` | Titre dans une carte en attente |
| `--wait-text-2` | `#F7DCCB` | Détail (commande, chemin) |
| `--wait-muted` | `#E7C3AE` | Aide, descriptions d'options |
| `--wait-btn` | `#57301F` | Bouton secondaire (« Toujours ») |
| `--wait-btn-text` | `#FFE0CF` | Texte de ce bouton |
| `--wait-note` | `#2E1B12` | Encart « Toujours », groupe de question |
| `--option` | `#4A2B1C` | Option de question |
| `--option-on` | `#6B3A24` | Option choisie (+ anneau 2px `--accent-soft`) |
| `--option-off` | `#3A2318` | Option non choisie, champ de ValuesCard |

### États

| Token | Valeur | Usage |
|---|---|---|
| `--ok` | `#9AD6CB` | Agent qui travaille, lignes ajoutées, PR ouverte, progression |
| `--ok-text` | `#BFE7DF` | Texte ajouté, « Toujours autorisé », PullTag |
| `--ok-line` | `#6E9E95` | Numéros des lignes ajoutées |
| `--ok-bg` | `rgba(154,214,203,0.10)` | Fond d'une ligne ajoutée, puce worktree |
| `--done` | `#B8D98C` | Coche d'un agent fini, colonne « Fini » |
| `--done-text` | `#C9E3A8` | `code 0`, « Dépôt Git détecté » |
| `--err` | `#F59E8C` | Croix d'erreur, lignes supprimées, PR en échec |
| `--err-text` | `#F8BDB1` | Texte d'erreur |
| `--err-muted` | `#E7C3BA` | Explication d'une erreur |
| `--err-bg` | `#2B1A17` | Carte d'erreur, outil en échec, agent en erreur |
| `--err-border` | `#8A4A40` | Contour d'erreur |
| `--err-ink` | `#2A0D08` | Texte sur `--err` |
| `--err-pre` | `#1A1210` | Sortie d'erreur |
| `--del-bg` | `rgba(245,158,140,0.10)` | Fond d'une ligne supprimée |
| `--del-line` | `#A7756C` | Numéros des lignes supprimées |

### Familles annexes

| Token | Valeur | Usage |
|---|---|---|
| `--background` | `#8FB3D9` | Anneau pointillé d'une tâche de fond (StatusIcon, BackgroundCard) |
| `--background-line` / `--background-bg` | `#5F7F9F` / `rgba(143,179,217,0.12)` | Contour et fond d'icône de BackgroundCard |
| `--sub-running` / `--sub-waiting` / `--sub-error` / `--sub-done` / `--sub-done-cell` / `--sub-stopped` | `#9AD6CB` / `#E08A5B` / `#F59E8C` / `#8BA36B` / `#5F7049` / `#4A3F39` | Cases et lignes des sous-agents |
| `--persona` / `--persona-bg` / `--persona-line` | `#E3D3B4` / 9 % / 30 % | PersonaChip (définition d'agent) |
| `--advisor` / `--advisor-text` / `--advisor-bg` / `--advisor-line` | `#C3B4F0` / `#DDD3FB` / 8 % / 32 % | Conseiller (AdvisorCard, option) |
| `--mode-default` / `--mode-accept` / `--mode-plan` / `--mode-auto` | `#999999` / `#AF87FF` / `#48968C` / `#FFC107` | Pilule du mode de permission |
| `--remote-dot` / `--remote-text` | `#7DB8FA` / `#B9D6F7` | PR en revue, session distante, verdict « revérifier » |
| `--merged` / `--merged-text` | `#C4A7F0` / `#CDBCF7` | PR fusionnée : ton de PullCard, texte de la puce PR (fond `--merged` 16 %) |
| `--jira` | `#2684FF` | Marque Jira |
| `--ticket-bg` / `--ticket-card` / `--ticket-card-hover` / `--ticket-border` / `--ticket-border-hover` / `--ticket-text` / `--ticket-muted` / `--ticket-button` | bleus nuit | Popup de ticket Jira et clés Jira (Kanban, agents), seulement là |

### Exceptions en dur

| Valeur | Usage |
|---|---|
| `#3A2420` / `#F8D5CC` | Bouton danger (ErrorCard, Kanban, Diff, Settings) |
| `#E3D7CF` | Texte de `.btn-secondary` et `.btn-icon` |
| `#110E0D` | Fond du terminal et de la sortie de ShellCard |
| `#E8DDD4` | Code dans la saisie et dans la bulle utilisateur |
| `#918579` | Numéros de ligne (éditeur, DiffView, QuickOpen) |
| `#F0C987` | Fichier modifié (FileTree, QuickOpen) |
| `#4A2E29` | Contour des encarts rouge-brun (« Refusé », PermissionCard, PlanCard) |
| `#4E443E` | Contour neutre (« Interrompu », session de terminal active) |
| `#5A3322` | Brun d'attente (progression d'un agent en attente) |
| `#D9B6A3` | Texte d'un bouton désactivé sur `--wait-btn` |
| `#4CBF85`, `#E5655A`, `#6EA8F0`, `#E0B65B`, `#B79CF0` | Tons de fichier : tableur, PDF, doc, archive, code |
| `#C7BCB4` | Texte des boutons de fenêtre |
| `#5DB5E7` / `#0D1B24` | Marque du build de dev |
| `#221A16` | Halo du splash |
| `#2A1F14` | Bandeau d'avertissement de l'éditeur |
| `#DDBBA8` | Détail d'un agent en attente |
| `#C99A90` | Repères dans la sortie d'erreur (ErrorCard) |
| `#CDB8FF` | Épopée Jira |
| `#1F8BFF`, `#FC6D26` | Logos Azure DevOps, GitLab |
| `#4FA3E0`, `#7CC47C`, `#E3B341`, `#6FA0D0`, `#3CC39A`, `#E07A9B`, `#A8A29E` | Couleurs au choix (extensions, palette de projet) |
| `#7FD1D9`, `#8DB4E8`, `#A8E3E8`, `#B3CDF0`, `#C9A6FF`, `#DCC5FF` | Palette ANSI du terminal |
| `#4ADE80`, `#FB923C`, `#F87171` | Niveaux de ContextGauge |

Les thèmes CodeMirror et xterm recopient des tokens en hexadécimal (objets JS) : garder les mêmes valeurs.

### Bulle utilisateur et bouton principal

Fond `--text`, texte `--ink`. La bulle a le coin bas droit pincé : `border-radius: 6px 6px 2px 6px`.

## Typographie

- **DM Sans** (`--sans`) pour le texte, **DM Mono** (`--mono`) pour le code, les chemins, les commandes, les coûts et les compteurs. Polices embarquées (`@fontsource`), pas de Google Fonts.
- Taille de base 13,5px.
- Échelle : 11 (légendes, en-têtes de colonne en capitales), 11,5 (métadonnées mono, badges), 12 (sous-titres, infos), 12,5 (états, résumés, aides, la plus courante), 13 (texte d'erreur, menus), 13,5 (base), 14 (titre de panneau), 15 (titre d'agent, titre de carte), 16 (titre de Confirm et de Settings), 18 (titre de dialogue), 22 (titre de page Kanban / Overview / PRs), 24 (états vides).
- Graisses : 500 (boutons secondaires), 600 (titres, noms), 700 (boutons principaux, titres de dialogue et de page, capitales).
- Capitales : 11px 700, `letter-spacing: 0.08em`, `--text-muted` ou `--text-dim`.
- Interlignage : 1,5 (bulles, aides), 1,6 (prose), 1,65 (Markdown de l'agent), 20px fixe pour les lignes de code et de diff.
- Marque « SQUADRON » : 12px, 700, capitales, `letter-spacing: 0.12em`, précédée d'un triangle cuivre de 14px.

## Formes et espacements

- Rayons : 2px (barres, petites pastilles), 3px (code en ligne, badges, puces), 4px (boutons, champs, lignes), 6px (cartes, blocs, menus), 8px (dialogues, colonnes Kanban, PullCard, FileCard, LinkCard), 50 % (points), 999px (pilules d'état).
- Espacements : 4, 6, 8, 10, 12, 14, 16, 18, 20, 24, 32px.
- Ombres : menus `0 14px 36px rgba(0,0,0,0.5)` + contour `0 0 0 1px --border-strong`, dialogues `0 30px 80px rgba(0,0,0,0.6)`, panneau glissant `-18px 0 40px rgba(0,0,0,0.45)`, agent en attente sélectionné `0 8px 24px rgba(201,105,58,0.18)`.
- Barre de ton : 3px à gauche d'une carte, couleur d'état ou de type (PullCard, FileCard, agent fini non lu). Liseré 2px pour AdvisorCard et le fil de commentaire à soi.

## Disposition

- Fenêtre sans décoration système, largeur minimale 1000px.
- **En-tête** 54px, `--bg-app`, grille `1fr auto 1fr`, trait bas `--border`.
- **Liste des agents** 304px par défaut, réglable de 240 à 560px.
- **Zone principale** `--bg-main` : en-tête d'agent (60px min), panneau d'infos optionnel, conversation, saisie, barre d'état 26px.
- **Conversation** : colonne centrée `--conversation-width` (740px, réglable), padding 20px, 14px entre blocs. Colonne de repères 40px `--bg-rail`.
- **Panneaux de droite** (`SidePanel`) : 420px min. Largeurs par défaut : diff 680 (côte à côte 1120), éditeur 780, terminal, sous-agents, artefacts 680. Modes glissant, côte à côte, en bas, plein écran, fenêtre.
- **Pages** (Kanban, Overview, PRs) : en-tête padding 24px 32px 16px, titre 22px 700 avec carré projet.

## Classes partagées (`app.css`)

- `.btn` 38px, padding 0 18px, rayon 4px, 500. `.btn-primary` `--text`/`--ink` 700. `.btn-secondary` `--surface-2`. `.btn-ghost` 32px, contour `--border-strong`, `--text-2`. `.btn-icon` 38px `--surface-4`. Désactivé : opacité 0,5.
- `.field` 42px `--surface-2`, contour 1px `--border-strong`, 2px `--border-focus` au focus, 2px `--err` si `aria-invalid`.
- `.option-row` / `.option-trigger` / `.option-menu` : ligne réglage 36px et son menu 280px qui s'ouvre vers le haut.
- `.effect-pulse`, `.effect-snake`, `.ring` : onglet ou carte qui travaille / qui attend, teintés par `--project`.
- Keyframes : `spin`, `snake`, `ping`, `pulse`, `ring`, `trace`, `blink`.
- `.jira-kind`, `.jira-urgency`, `.jira-status` : marques Jira. `.sr-only`, `.mono`.

## Composants

### Icônes et briques (à réutiliser)
- **StatusIcon** (10px) : travaille = anneau `--ok` qui tourne (1,4s), fond = anneau pointillé `--background` (6s), attend = point 8px `--accent` + halo 4px, fini = coche `--done`, interrompu = carré `--text-2`, erreur = croix `--err`.
- **Icon** : SVG 24 en contour 2px `currentColor`, 14px par défaut, noms dans `Icon.svelte`. À préférer à un SVG en ligne.
- **ModeIcon** : schéma du placement d'un panneau (slide, side, bottom, full).
- **Avatar** : rond 24px `--surface-4`, initiales 700 `--text-2`.
- **PersonaChip** : 22px, rayon 3px, `--persona-bg` + contour `--persona-line`, mono 500 12px `--persona`.
- **Select** : déclencheur 36px `--surface-2` + contour `--border-strong`, liste `--menu`, options 34px, choisie `--accent-soft` 600.

### En-tête
Marque à gauche, puis bouton 30px du panneau Projets (`--surface-5` actif). Onglets au centre : « Overview », séparateur 1px `--border-strong`, puis les projets. Conteneur `--bg-tabs`, padding 4px, rayon 6px, espacement 4px. Onglet 30px, rayon 4px, `--text-muted`, pastille projet 9px rayon 2px. Onglet actif : blanc 600, teinté par la couleur du projet. Onglet qui travaille : `.effect-pulse` ou `.effect-snake` (réglage). Onglet avec un agent qui attend : `.ring`. Badges 20px : attente `--accent`/`--accent-ink` 11,5px 700, finis non lus contour `--ok`, erreur `--err`/`--err-ink`. « +N » `--surface-5`. À droite : bouton « Pull requests » 30px (contour `--wait-border`, `--accent-soft` 13px 600), boutons de fenêtre 24px `--surface-2`. Build de dev : triangle et étiquette « dev » en `#5DB5E7`.

### Liste des agents
- En-tête : nom du projet (700), « N agents · N t'attend » (12px `--text-dim`), « + Agent » 32px pointillé.
- Agent : padding 12px 14px, rayon 6px, espacement 6px. Ligne 1 : StatusIcon (ou n° de PR en revue), point non lu 7px `--ok`, nom 600, épingle, verdict. Ligne 2 : état 12px, ticket Jira et PersonaChip à droite ; détail en mono 11,5px. Puis PullTag, compteurs de sous-agents, progression 4px.
- Sélectionné : `--surface-1` + contour `--border-strong`. En attente : `--wait-bg` + `--wait-border`, état `--accent-soft` 600 ; sélectionné : anneau 2px `--accent-soft` + ombre cuivrée. En erreur sélectionné : `--err-bg`, anneau `--err`. Fini non lu : fond `--ok` 9 % + barre de ton `--ok`. Sélection multiple : anneau intérieur 2px `--accent-soft` + barre d'actions.

### En-tête d'agent
60px min, padding 10px 20px, sur plusieurs lignes. Nom 15px 600 (bouton qui renomme). Bouton réglages 34px `--menu` + contour `--border-strong` : modèle 600, effort en marches, conseiller `--advisor-text`, pilule de mode 22px `--mode-*`. Outils en `.btn-ghost` (Éditeur, Terminal, Diff `+N` `--ok` `−N` `--err`, PRs, Kanban…), actif `--surface-5` + contour `--text-faint`, compteur 18px `--accent`. Puces de contexte 34px `--menu` (remote, MCP, tâches, ticket, branche, PR, artefacts, sous-agents) et ContextGauge (anneau 22px, `#4ADE80` / `#FB923C` / `#F87171`).

**Panneau de contexte** (`ContextPanel`, ouvert par ContextGauge) : 460px, `--bg-main`, rayon 6px, contour + ombre de menu. En tête « Context » 600 et segments Usage / Settings (`--surface-2`, actif `--text`/`--ink` 700). Usage : barre de progression 8px `--surface-3` remplie par source, trait 2px `--text-faint` au seuil d'auto-compaction ; deux tuiles `--surface-1` (mono 18px) ; sections en capitales 11px ; lignes 30px (26px imbriquées, filet gauche `--border-strong`), chevron 18px `--surface-3`, pastille 9px de la source, total mono 12,5px `--text-2`, « ≈ » devant une estimation. Couleurs des sources : Claude Code `--text-faint`, Squadron `--remote-dot`, configuration `--persona`, serveurs MCP `--background`, plugins et appels de plugins `--advisor`, conversation `--ok`, hooks et rappels `--ok-line`. Logo de plugin 16px rayon 4px. Bas : « Compaction » et un champ 36px `--surface-2` qui contient le bouton « Compact now ». Settings : lignes 48px séparées par `--border`, `Select` à droite.

### Conversation
- **Message utilisateur** : bulle claire à droite, 540px max, padding 12px 16px.
- **Réponse de l'agent** : bloc `--surface-1`, padding 16px 18px, rayon 6px, espacement 12px. Markdown 1,65 ; code en ligne `--surface-3`, mono 12,5px, padding 1px 6px.
- **Retour en haut** : sous une réponse plus haute qu'une fois et demie la conversation, filet `--border`, padding-top 12px, puis « Back to top » en `.btn-ghost` 28px aligné à droite, qui fait défiler jusqu'au début du bloc.
- **Résumé replié** (réflexion, groupe d'actions) : 12,5px `--text-muted`, chevron dans un carré 18px `--surface-3`.
- **Note** (texte de l'agent entre deux étapes) : toujours visible, hors du repli, dans un bloc de réponse, son heure en tête en mono 11,5px `--text-dim`. La réponse finale n'a pas d'heure.
- **Appel d'outil** (`ToolRow`) : ligne 30px, nom sur 40px `--text-dim`, argument en mono, statut à droite. Edit et Write ouvrent le Diff ; Bash se déplie. En échec : `--err-bg`, contour `#6B3A33`, sortie `--err-text` sur `--err-pre`.
- **Fin de tour** : mono 11,5px `--text-dim`, « Turn N done · N in · N out · durée ».
- **Repères d'historique** (permission répondue) : 12,5px, padding 9px 14px. Autorisé `#1F2321` / contour `#33403B` / `--ok-text`. Refusé `#241917` / contour `#4A2E29` / `--err-text`.
- **Interruption** : encart `--surface-2`, contour `#4E443E`. **Rechargement** : séparateur horizontal.
- **Bouton « Dernier message »** : pilule claire flottante, rayon 18px.
- **Colonne de repères** : un trait par message de l'utilisateur, groupés et centrés. Inactif 10×2px `--text-faint`, actif 18×3px `--accent` avec halo. Info-bulle `--menu` 300px.

### Cartes de conversation
- **FileCard** (`FileCard.svelte`) : `--bg-main`, rayon 8px, contour `--border-strong`, barre de ton 3px (tons de fichier). Vignette 30×36 rayon 5px, extension 700 9,5px. Nom 600 blanc, méta mono 11,5px. Actions sur `--surface-1`, boutons 28px. Au-delà de 3 cartes d'affilée, seules les 2 premières restent : **FileMore** les suit, même coque sans barre de ton, vignettes des types cachés empilées (décalage 12px, 3 max), « N more files » 600 et les noms en mono 11,5px `--text-dim`, « Show all » 28px. Déplié : lien « Show fewer files » 12,5px `--text-dim`.
- **PullCard** : même coque que FileCard, tons `--ok` ouverte, `--err` échec, `--text-faint` brouillon/fermée, `--merged` fusionnée. Titre 15px 600, état en pilule (ton 14 %, rayon 10px, 11,5px 600). En ligne : pilule 21px `--surface-3`, aperçu 460px au survol.
- **ShellCard** (commande `!`) : `--surface-1`, contour `--border`, rayon 9px ; contour `--ok` 55 % réussie, `--err` 60 % échouée, `--accent` 55 % en cours. `!` en `--accent`. Sortie `#110E0D`, mono 12px/1,5, 320px max. Commande lancée par `run_sudo` : `sudo` à la place de `!`, sans Relancer ni Terminal.
- **AdvisorCard** : liseré 2px `--advisor`, fond `--advisor-bg`, rayon `0 6px 6px 0`, padding 10px 14px, origine 11px 600 capitales.
- **SubagentsCard** : `--surface-1` + contour `--border-strong`, padding 16px 18px, icône 30px `--surface-3`, cases 14px rayon 2px `--sub-*`.
- **ForkCard** (skill lancé en sous-agent par l'outil Skill) : coque de SubagentsCard, contour animé `snake` teinté `--background`. Icône 30px `--background-bg`, nom 600, pastille « Subagent » 11px 600 rayon 10px (teinte à 14 %), « Started by … · modèle » 12,5px `--text-dim`, étape mono 12px à droite. Segments 4px rayon 2px, étape en cours en dessous (`rise`). Quand il demande une permission : `--wait-bg`, contour `--wait-border`, halo qui respire, icône `--accent` qui sonne, « {nom} asks you » ; la PermissionCard s'ouvre dedans (`nested` : `--wait-note`, icône 26px `--wait-btn`, encart Always sur `--wait-bg`) et ne se répète pas en bas de la conversation. Fini : coche `--ok`, sans contour animé ; arrêté : `--err-bg` + `--err-border`, dernière phrase en `--err-text`.
- **BackgroundCard** : `--surface-1` + contour `--background-line`, icône 30px `--background-bg`.
- **NoticeCard** : encart contour `--border`, 12,5px `--text-muted`, 640px max ; variante réglages 680px, rayon 8px, lignes 40px.
- **CommentChip** : 26px `--surface-3` + contour `--border-strong`, icône `--accent-soft`, mono 12px.
- **FileChip** / **LinkChip** : 20px, rayon 4px, `--surface-3` + contour `--border-strong`, mono 12,5px `--text-2`, tuile 15px. LinkChip : favicon sur tuile `--text`, domaine puis chemin `--text-faint`. Dans la bulle (`light`) : voile `--accent-ink` 13 % + contour 22 % (20 % au survol), texte et icônes `--accent-ink`, chemin et compteur à 58 %.
- **LinkCard** : sous la bulle, 440px, 3 au plus (un lien de PR donne une PullCard). Coque de FileCard, barre de ton `--text-faint`, contour `--border-focus` au survol. Image OpenGraph 104×54 rayon 5px, sinon vignette 30×36 avec le favicon sur `--text`. Méta mono 11,5px (site · domaine), titre 13px 600 blanc, description 12,5px `--text-muted` sur 2 lignes.
- **TaskRef** : carte de tâche (padding 10px 14px, rayon 8px, contour `--border`) ou puce `--surface-3` avec point 6px de la colonne.

### Cartes qui attendent l'utilisateur
Fond `--wait-bg`, contour `--wait-border`, padding 16–18px, rayon 6px, espacement 12px. Icône 30px `--accent` sur `--accent-ink`. Titre `--wait-text` 600, détail en mono `--wait-text-2`.
- **Permission** : « Run a command? », « Edit a file? » (diff 200px max), « Use a tool? » (clé/valeur). Boutons 36–38px : Allow (`--accent`), Always (`--wait-btn`), Deny (`--surface-3`). Encart `--wait-note` qui dit ce que « Always » autorise.
- **Question** : options pleine largeur (`--option`, touche numérotée 22px, titre 600, description `--wait-muted`, badge « Recommended »). Plusieurs questions : un groupe `--wait-note` par question, celui en cours `--option` + contour `#A55E37`.
- **Plan** : Markdown dans un encart `--bg-main`, contour `#4A2E29`, trois boutons de lancement.
- **ValuesCard** : champs 36px mono 13px sur `--option-off`, anneau 2px `--accent-soft` au focus. Enregistrer `--accent` ; désactivé `--wait-btn` / `#D9B6A3`.
- **SecretCard** (`run_sudo`, `add_ssh_key`) : icône cadenas, dossier ou chemin de clé en mono. Pour sudo, raison de l'agent en `--wait-muted` puis encart `--bg-app` contour `#4A2E29` qui montre la commande entière (`sudo` en `--accent-soft`). Libellé du champ à gauche, « va à sudo, jamais à Claude » à droite en `--wait-muted` ; champ masqué comme ValuesCard sur la même ligne que Lancer (`--accent`) et Refuser (`--surface-3`). Erreur : anneau 2px `--err`, texte `--err-text` avec icône. Sans mot de passe exigé par sudo, seulement les deux boutons.

### Carte d'erreur
Fond `--err-bg`, contour `--err-border`, icône `--err`. Titre blanc, contexte `--err-muted`, message brut en `pre` mono 12px `--err-text` sur `--err-pre`. Actions : Recharger (bouton clair), Copier l'erreur, bouton danger `#3A2420` / `#F8D5CC`.

### Barre de saisie
Zone padding 12px 20px 18px, trait haut `--border`. Conteneur `--surface-2`, rayon 6px, largeur `--conversation-width`. Champ CodeMirror 38–200px, blocs de code sur `--bg-main` mono 12,5px `#E8DDD4`. Boutons icône 38px (Joindre, Interrompre, ⋯), Envoyer `.btn-primary`. Mode `!` : contour 1px `--accent` + anneau 3px `--accent-soft`. Menus `/` et `@` en `--menu`. Puces de commentaires et pièces jointes au-dessus. Les liens deviennent des LinkChip, que Ctrl+clic ouvre.

### Barre d'état
26px `--bg-sidebar`, trait haut, 11,5px `--text-dim`, espacement 18px. Jauges 44×4 (`--ok-line`, puis `--accent`, puis `--err`).

### Panneaux de droite (`SidePanel`)
Fond `--bg-sidebar`. En-tête 60px, padding 0 12px 0 18px, trait bas `--border` : titre 14px 700, sous-titre 12px `--text-dim` en ellipse, actions en `.btn-ghost`, bouton de placement et fermer 32px `--surface-2`. Poignée 6px, `--border-strong` au survol. Onglets internes : 12,5px `--text-2`, padding 5px 10px, rayon 4px, actif `--surface-4` blanc.
- **Diff** : segments (`--surface-2`, padding 2px, boutons 26px 12,5px, actif `--surface-4` 600) Unified / Split, Uncommitted / Branch / PR. Arbre 230px. Onglets de fichier 112px max, actif `--surface-4` + contour `--text-faint`. Badges NEW `#1F3330` / `--ok-text`, supprimé `--err-bg`. Lignes en grille `56px 40px 16px 1fr`, mono 12px, interligne 20px. Mon fil de revue : liseré 3px `--accent`.
- **Éditeur** : arbre 220px 12,5px, corps `--bg-app`, onglets 36px, actif `--bg-app` + liseré haut 2px couleur du projet. Barre d'état 30px.
- **Terminal** : sessions en onglets 30px mono 12px, active `--menu` + contour `#4E443E`. Écran `#110E0D`.
- **Sous-agents** : filtres 28px, lignes 36px en grille, mono 11,5px, couleurs `--sub-*`.
- **Artefacts** : `Select` 420px max pour choisir l'artefact quand l'agent en a plusieurs (détail : type et heure), vue `--bg-main`, iframe fond blanc, contour 2px `--accent` en mode « Point ».
  - **En-tête** : Point et Share en texte, puis un bouton ⋯ `.btn-ghost` 32px carré (`--surface-4` ouvert) dont le menu, construit comme celui de Share (300px), donne Copy the source, Open the folder ou Save as a file, un filet, le nettoyage, un filet, puis Delete au titre `--err-text` (aide « Stops its shares too » quand l'artifact est partagé). Icônes `copy`, `folder`, `update`, `clock`, `trash`.
  - **Nettoyage** (menu ⋯) : ligne d'option « Deleted in N days » ou « Never deleted », dessous d'où vient le délai (« Rule of … », « Squadron's rule », « Its own time »), une erreur de nettoyage en `--err-text`. Puis, aligné sur le texte (42px), des segments comme ceux du panneau de contexte : fond `--surface-2` padding 3px rayon 4px, boutons 24px 12px `--text-2`, choisi `--text` sur `--ink` 700 : Rule, 30 days, 1 year, Never. Le menu reste ouvert au changement.
  - **Menu Share** (emplacement `artifact.actions`) : `.btn-ghost` avec chevron, `--surface-4` ouvert, point 6px `--ok` devant « Share » quand l'artefact a un lien. Menu 380px `--menu` à droite du bouton. Destination : icône 30px `--surface-3`, titre 13px 600 blanc, aide 12px `--text-dim`, nom du mod en mono 11,5px `--text-faint` ; désactivée à 0,5. Destinations intégrées d'abord, filet `--border` avant celles des mods. Destination partagée : titre précédé du point `--ok`, heure du partage (ou « Edited since… »), boutons 28px alignés sur le texte : Open `.btn-primary` (`.btn-ghost` quand Update, `.btn-primary`, est proposé), Copy link `.btn-ghost`, Stop sharing `.btn-ghost` poussé à droite, `--err` au survol.
  - **Bandeau de partage** : sous l'en-tête, 52px min, padding 0 12px 4px 18px, 12,5px `--text-2`, teinte propre à chaque état. Un reflet (`sweep` 2,6s) traverse le fond. En haut le nom de la destination 600 blanc et l'étape en mono 600 11,5px teintée (« 3/6 »), dessous l'étape en cours qui monte en fondu (`rise`). En bas, 3px de segments espacés de 3px : finis pleins, en cours à 30 % avec une lueur qui le parcourt (`run` 1,3s) ; un seul segment sans progression.
    - Travail : `--background-bg`, trait `--background-line`, orbe 22px (anneau conique qui tourne 1,1s, point 6px lumineux).
    - Question du sous-agent : `--wait-bg`, trait `--wait-border`, point qui pulse (`ping`), « Answer » `--accent` 700 qui sonne (`ring`).
    - Partagé : `--ok` à 9 % sur `--bg-sidebar`, coche 22px `--ok` qui apparaît (`pop`), lien en mono `--text-dim`, Open et Copy the link `.btn-ghost` ; disparaît après 5 s.
    - Arrêté : `--err-bg`, trait `--err-border`, croix 22px `--err`, raison sur une ligne, Close en bouton danger.

### Pages
- **Kanban** (`Board.svelte`) : colonnes `repeat(N, minmax(220px,1fr))`, espacement 12px, fond `--bg-sidebar`, rayon 8px. En-tête de colonne en capitales 11px, pastille 8px (`--sub-running`, `--accent`, `--done`), compteur mono. Carte `--surface-1` (`--surface-2` au survol), padding 12px 14px, rayon 6px ; en attente `--wait-*`, en erreur `--err-*`. Glisser : origine à 0,4, fantôme `--surface-4` + contour `--border-focus` incliné −2°, repère 2px `--accent`. « + Task » 34px pointillé.
- **Overview** : tableau, en-têtes collants 11px 700 capitales, cellules padding 10px 12px, trait `--border`, agent en attente `rgba(59,34,22,0.55)`.
- **Pull requests** (`PullsView.svelte`) : toutes les PR ouvertes du projet, chacune dans une seule section : To review, Agent reviews (un agent la relit sans vous), In review (les autres, sans les brouillons), Yours. Sections titrées en capitales 13px, aide en casse normale 12,5px `--text-faint` ; lignes `--surface-1` + contour `--border`, padding 12px 14px, rayon 6px, transparentes quand rien n'attend. Verdict 12px 600 rayon 4px : relire `--accent`, corriger `--err`, fusionner `--ok`, revérifier `--remote-dot`, agent de relecture `--persona` sur `--persona-bg` + contour `--persona-line`. À gauche de « Review » : bouton agent 152px au ton agent (`--persona` sur `--persona-bg` + contour `--persona-line`), icône `persona` puis l'état de l'agent de relecture en StatusIcon devant un libellé qui le dit, spinner pendant le démarrage ; « Review » fait alors 88px, grisé avec son menu tant que l'agent démarre, pour que les lignes restent alignées. Nouveaux commits depuis la relecture de l'agent : le bouton agent devient « Review changes », icône de relance `--remote-dot`, et « Go to » l'agent passe dans le menu.

### Dialogues
Voile `--overlay`, boîte `--bg-main`, rayon 8px, contour `--border-strong`. En-tête padding 20px 22px 6px, titre 18px 700, aide 13px `--text-dim`, fermer 32px `--surface-2`. Champs `.field`, chemins en mono. Boutons en bas à droite (Cancel `--surface-2`, action principale claire), séparés par un trait `--border`. Erreur de champ : `--err-text` avec icône, bouton principal désactivé.
- **Tickets Jira** (`JiraDialog.svelte`, bouton Jira du Kanban ou `J`) : Dialog 1180px, hauteur `min(720px, 100vh − 150px)`. Sous le titre, projet et sprints en 13px `--text-dim` + rafraîchir 26px. Filtres à gauche sur 232px `--bg-sidebar` : sections en capitales 11px, lignes 30px 12,5px `--text-2` avec compteur mono 11px `--text-faint` ; projets en boutons (actif `--surface-4` blanc 600, section absente s'il n'y a qu'un projet Jira), le reste en cases 14px rayon 3px (contour `--border-focus`, cochée `--text` + coche `--ink`). À droite, recherche 32px `--surface-2` (`/`), « N of M » et « Reset filters », puis colonnes de statut de 288px alignées à gauche qui défilent en largeur (`--bg-sidebar`, rayon 8px, pastille `.jira-status`). Ticket `--surface-1` (`--surface-2` au survol), padding 10px 12px, rayon 6px : marques, titre 13px sur 2 lignes, « + Create a task » 24px `--surface-3` ou, s'il a déjà sa tâche, fond transparent + contour `--border` et la colonne de la tâche (pastille 8px du Kanban). Ticket ouvert : contour 2px `--border-focus`. Aucun résultat : message 15px 600 centré + « Reset filters » `.btn-secondary`.
- **Ticket Jira** (`TicketDialog.svelte`) : Dialog plein écran, colonne principale (description, pièces jointes en grille `minmax(180px,1fr)` vignette 110px `--surface-1`, commentaires) + colonne 340px `--bg-sidebar` (champs, liens, « Open in Jira » `--ticket-*`), pied « Read only » + « Create a task ».
- **Question d'une commande** (`AskpassDialog`) : 460px, voile, icône cadenas 32px `--accent` sur `--accent-soft`, agent et projet, commande en cours en mono sur `--surface-1`, question en mono `--text-muted`. Seulement Non (`.btn-secondary`) et Oui (`.btn-primary`) : aucun champ.
- **Confirm** : 440px, icône 32px rayon 6px (`--surface-3`, avertissement `--wait-btn` / `--accent-soft`, erreur `--err-bg` / `--err`, ou logo `HostLogo` 18px sur `--surface-3`), titre 16px 700. Case à cocher facultative sous le texte (case native `accent-color: --accent`, libellé 13px `--text-2`, aide 12px `--text-dim`). Action destructrice : bouton danger `#3A2420` / `#F8D5CC` 700 à la place du bouton clair.
- **Settings** : Dialog plein écran. Navigation 230px `--bg-sidebar`, groupes en capitales, entrées 34px, active `--surface-4` blanc 600. Contenu 860px max. Cartes de choix `--surface-1` + contour `--border-strong`, choisie `--surface-3` + anneau 2px `--text`. Listes (Agents archivés, Serveurs MCP) : lignes `--surface-1` rayon 4px, padding 10px 12px, nom 600 et légende 12px `--text-dim` (commande en mono 11,5px), bouton `.btn-secondary` 30px ; sections en capitales 11px ; un serveur à la demande ajoute sous sa ligne un `.field` 34px pour sa description. Fiche d'extension, comptes : lignes 12,5px (valeurs non secrètes en mono séparées par « · », `.btn-secondary` Edit et `.btn.danger` Remove 30px, ligne en cours d'édition en `--text-strong`), puis un `.field` 30px par champ empilés à 6px (secret en `password`, « gardé si laissé vide » en placeholder pendant l'édition) et « Add the account » / Save `.btn-secondary`. Fiche d'une extension qui partage des artifacts avec `cleanup` : section « Clean up shared artifacts », `.field` select (délai de l'extension, règle de Squadron, 30 jours, 90 jours, 1 an, Never) et son aide 12px `--text-dim`.
- **Add project** : dossier en lecture seule + « Browse… », « Git repository · branch X » en `--done-text`.
- **New agent** : nom, modèle et effort en segments, mode de permission en 4 cartes, premier message, « Ctrl ↵ ».
- **Spotlight** : 680px à 14vh, rayon 10px, champ 52px 16px, sections en capitales. Tab fige la ligne choisie (projet, action, agent) en puce 24px rayon 3px au début du champ : projet et agent teintés comme une cible d'action (pastille carrée ou ronde), action en `--surface-3` avec son icône ; marquée par un premier ⌫ : `--surface-5` + contour `--border-focus`; PR : icône, numéro mono 12px `--text-dim`, titre. Ligne de PR : numéro mono, titre, cible projet, auteur 12px `--text-dim`, état en `.note` (`wait` quand elle vous attend). Champ vide : « Pull requests waiting for you » en tête, compte en `.note.wait`. Élément d'extension : son icône (`plugins` sinon), son libellé, le nom de l'extension en `.tag`. Filtres sous le champ dès qu'on tape (puces 22px rayon 3px `--surface-3`, pastille projet 8px, compte mono 11px ; active teintée par la couleur du projet comme la cible d'une action), résultat choisi `--surface-2` + contour `--border-strong`, `↵` en `--accent`. **QuickOpen** : 620px `--menu`, rayon 9px, champ 44px, lignes 34px.

### États vides
Centrés, 440–460px. Aucun projet : triangle cuivre, titre 24px, texte `--text-muted`, bouton « Add project », zone de dépôt en pointillé. Projet sans agent : bouton « New agent » et `Ctrl N`. Dossier introuvable : carte d'alerte avec trois actions.

## Mouvement
Anneau qui travaille : rotation 1,4s linéaire. Tâche de fond : 6s. Curseur de streaming : clignotement 1s. Onglet : `pulse` 2s ou `snake` 2,4s, `ring` 2,4s. Dépliage d'une carte ou d'un formulaire : glisse vers le bas, la conversation défile juste assez pour le montrer. Tout s'arrête sous `prefers-reduced-motion`.

## Accessibilité
Vrais `button`, `a`, `input` et `label`. `aria-label` sur chaque bouton icône. Focus clavier visible : anneau 2px `--accent-soft`, décalé de 2px. Les cartes d'attente sont des `section` étiquetées, les erreurs des `role="alert"`, les onglets des `role="tab"` avec `aria-selected`.
