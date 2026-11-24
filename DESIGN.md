---
name: Monitoring
description: "Écran tactile de supervision posé sur le bureau : le verdict d'abord, le détail au toucher."
colors:
  accent: "#2563EB"
  accent-fond: "#EAF1FE"
  fond: "#F2F2F6"
  surface: "#FFFFFF"
  creux: "#F2F2F6"
  piste: "#E6E6EB"
  separateur: "#E5E5EA"
  texte: "#1C1C1E"
  secondaire: "#636366"
  tertiaire: "#AEAEB2"
  up-plein: "#34C759"
  up-texte: "#1B7F3B"
  up-fond: "#E8F6EC"
  up-texte-fond: "#17602E"
  warn-plein: "#FF9F0A"
  warn-texte: "#A15C00"
  warn-fond: "#FFF4E3"
  warn-texte-fond: "#774400"
  down-plein: "#FF3B30"
  down-texte: "#C8261B"
  down-fond: "#FDECEA"
  down-texte-fond: "#8F1D14"
  series-0: "#0891B2"
  series-1: "#9333EA"
  series-2: "#64748B"
  series-3: "#1C1C1E"
  nuit-fond: "#000000"
  nuit-texte: "#F2F2F7"
  nuit-secondaire: "#98989F"
  nuit-discret: "#8E8E93"
  nuit-up: "#30D158"
  nuit-warn: "#FF9F0A"
  nuit-down: "#FF453A"
typography:
  horloge-veille:
    fontFamily: "Inter, sans-serif"
    fontSize: "176px"
    fontWeight: 300
    lineHeight: 1
  display:
    fontFamily: "Inter, sans-serif"
    fontSize: "34px"
    fontWeight: 700
  horloge:
    fontFamily: "Inter, sans-serif"
    fontSize: "30px"
    fontWeight: 600
    lineHeight: 1
  headline:
    fontFamily: "Inter, sans-serif"
    fontSize: "28px"
    fontWeight: 700
  chiffre:
    fontFamily: "Inter, sans-serif"
    fontSize: "26px"
    fontWeight: 700
  verdict:
    fontFamily: "Inter, sans-serif"
    fontSize: "22px"
    fontWeight: 700
  title:
    fontFamily: "Inter, sans-serif"
    fontSize: "17px"
    fontWeight: 600
  nom:
    fontFamily: "Inter, sans-serif"
    fontSize: "16px"
    fontWeight: 600
  body:
    fontFamily: "Inter, sans-serif"
    fontSize: "15px"
    fontWeight: 400
  bouton:
    fontFamily: "Inter, sans-serif"
    fontSize: "15px"
    fontWeight: 600
  meta:
    fontFamily: "Inter, sans-serif"
    fontSize: "14px"
    fontWeight: 400
  pastille:
    fontFamily: "Inter, sans-serif"
    fontSize: "14px"
    fontWeight: 600
  label:
    fontFamily: "Inter, sans-serif"
    fontSize: "13px"
    fontWeight: 500
  legende:
    fontFamily: "Inter, sans-serif"
    fontSize: "13px"
    fontWeight: 400
  axe:
    fontFamily: "Inter, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  mono:
    fontFamily: "JetBrains Mono, monospace"
    fontSize: "13px"
    fontWeight: 400
rounded:
  barre: "2px"
  puce: "8px"
  controle: "10px"
  bouton: "12px"
  dense: "14px"
  carte: "16px"
  fenetre: "20px"
  capsule: "9999px"
spacing:
  ligne: "2px"
  serre: "4px"
  interne: "8px"
  bloc: "10px"
  grille: "12px"
  rangee: "14px"
  entete: "16px"
  section: "18px"
  colonnes: "20px"
  groupe: "22px"
  page-y: "24px"
  page-x: "28px"
components:
  carte:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    rounded: "{rounded.carte}"
    padding: "16px 20px"
  tuile:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    typography: "{typography.chiffre}"
    rounded: "{rounded.carte}"
    padding: "14px 18px"
  carte-service:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    typography: "{typography.nom}"
    rounded: "{rounded.dense}"
    padding: "14px 16px"
  carte-service-warn:
    backgroundColor: "{colors.warn-fond}"
    textColor: "{colors.warn-texte-fond}"
    rounded: "{rounded.dense}"
    padding: "14px 16px"
  carte-service-down:
    backgroundColor: "{colors.down-fond}"
    textColor: "{colors.down-texte-fond}"
    rounded: "{rounded.dense}"
    padding: "14px 16px"
  verdict-up:
    backgroundColor: "{colors.up-fond}"
    textColor: "{colors.up-texte}"
    typography: "{typography.verdict}"
    rounded: "{rounded.carte}"
    padding: "14px 18px"
  verdict-warn:
    backgroundColor: "{colors.warn-fond}"
    textColor: "{colors.warn-texte}"
    typography: "{typography.verdict}"
    rounded: "{rounded.carte}"
    padding: "14px 18px"
  verdict-down:
    backgroundColor: "{colors.down-fond}"
    textColor: "{colors.down-texte}"
    typography: "{typography.verdict}"
    rounded: "{rounded.carte}"
    padding: "14px 18px"
  verdict-perime:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    typography: "{typography.verdict}"
    rounded: "{rounded.carte}"
    padding: "14px 18px"
  bouton-principal:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    typography: "{typography.bouton}"
    rounded: "{rounded.bouton}"
    padding: "0 18px"
    height: "44px"
  bouton:
    backgroundColor: "{colors.accent-fond}"
    textColor: "{colors.accent}"
    typography: "{typography.bouton}"
    rounded: "{rounded.bouton}"
    padding: "0 16px"
    height: "44px"
  bouton-neutre:
    backgroundColor: "{colors.creux}"
    textColor: "{colors.texte}"
    typography: "{typography.bouton}"
    rounded: "{rounded.bouton}"
    padding: "0 18px"
    height: "44px"
  bouton-carte:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    typography: "{typography.bouton}"
    rounded: "{rounded.bouton}"
    padding: "0 16px"
    height: "44px"
  lien-down:
    textColor: "{colors.down-texte}"
    typography: "{typography.bouton}"
    rounded: "{rounded.controle}"
    padding: "0 12px"
    height: "44px"
  bouton-retour:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    rounded: "{rounded.capsule}"
    size: "44px"
  pastille-up:
    backgroundColor: "{colors.up-fond}"
    textColor: "{colors.up-texte}"
    typography: "{typography.pastille}"
    rounded: "{rounded.capsule}"
    padding: "0 12px"
    height: "30px"
  pastille-warn:
    backgroundColor: "{colors.warn-fond}"
    textColor: "{colors.warn-texte}"
    typography: "{typography.pastille}"
    rounded: "{rounded.capsule}"
    padding: "0 12px"
    height: "30px"
  pastille-down:
    backgroundColor: "{colors.down-fond}"
    textColor: "{colors.down-texte}"
    typography: "{typography.pastille}"
    rounded: "{rounded.capsule}"
    padding: "0 12px"
    height: "30px"
  pastille-info:
    backgroundColor: "{colors.accent-fond}"
    textColor: "{colors.accent}"
    typography: "{typography.pastille}"
    rounded: "{rounded.capsule}"
    padding: "0 12px"
    height: "30px"
  puce:
    backgroundColor: "{colors.creux}"
    textColor: "{colors.secondaire}"
    rounded: "{rounded.puce}"
    padding: "2px 8px"
  puce-moi:
    backgroundColor: "{colors.accent-fond}"
    textColor: "{colors.accent}"
    rounded: "{rounded.puce}"
    padding: "2px 8px"
  compteur-down:
    backgroundColor: "{colors.down-texte}"
    textColor: "{colors.surface}"
    rounded: "{rounded.capsule}"
    padding: "2px 8px"
  compteur-info:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.surface}"
    rounded: "{rounded.capsule}"
    padding: "2px 8px"
  barre-laterale:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    padding: "22px 14px"
    width: "220px"
  nav-lien:
    textColor: "{colors.texte}"
    typography: "{typography.body}"
    rounded: "{rounded.bouton}"
    padding: "0 12px"
    height: "48px"
  nav-lien-actif:
    backgroundColor: "{colors.accent-fond}"
    textColor: "{colors.accent}"
    typography: "{typography.bouton}"
    rounded: "{rounded.bouton}"
    padding: "0 12px"
    height: "48px"
  segment-rail:
    backgroundColor: "{colors.piste}"
    rounded: "13px"
    padding: "3px"
    height: "50px"
  segment:
    textColor: "{colors.secondaire}"
    rounded: "{rounded.controle}"
    width: "112px"
    height: "44px"
  segment-actif:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    typography: "{typography.bouton}"
    rounded: "{rounded.controle}"
    width: "112px"
  alerte:
    backgroundColor: "{colors.down-fond}"
    textColor: "{colors.down-texte-fond}"
    typography: "{typography.body}"
    rounded: "{rounded.dense}"
    padding: "12px 16px"
  note-info:
    backgroundColor: "{colors.accent-fond}"
    textColor: "{colors.texte}"
    typography: "{typography.meta}"
    rounded: "{rounded.dense}"
    padding: "14px 16px"
  fenetre:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.texte}"
    rounded: "{rounded.fenetre}"
    padding: "26px 28px"
    width: "560px"
  toast:
    backgroundColor: "rgba(28, 28, 30, 0.92)"
    textColor: "{colors.surface}"
    typography: "{typography.bouton}"
    rounded: "{rounded.capsule}"
    padding: "12px 20px"
  bandeau-hors-ligne:
    backgroundColor: "{colors.down-texte}"
    textColor: "{colors.surface}"
    typography: "{typography.pastille}"
    padding: "9px"
  veille:
    backgroundColor: "{colors.nuit-fond}"
    textColor: "{colors.nuit-texte}"
    typography: "{typography.horloge-veille}"
---

# Design System: Monitoring

## Overview

**Creative North Star: « La vigie de bureau »**

Une tablette posée à côté de l'écran de travail, toujours allumée, lue de près et touchée souvent. Elle se tait quand tout va bien et parle net quand quelque chose casse : le verdict occupe une carte pleine largeur en tête de l'accueil, teintée de l'état, et tout le reste (chiffres, services, courbes) se lit ensuite. Le registre est celui d'une app système de tablette, avec Apple Maison / Santé pour étalon : fond gris groupé clair, cartes blanches aux grands rayons sans bordure ni ombre, filets gris fins, Inter en casse de phrase, gros chiffres gras.

La couleur est rare et toujours signifiante. Le bleu est la seule voix d'action ; le vert, l'ambre et le rouge ne disent qu'un état, et chaque état a aussi sa forme, pour ne jamais dépendre de la couleur seule. Quand le serveur ne répond plus, l'affichage figé perd ses teintes : seul un bandeau rouge dit que ce qu'on voit est périmé. La veille bascule dans le noir d'une horloge de chevet.

La matière est contrainte par la machine : rendu logiciel tiny-skia, sans GPU. Pas d'ombre, de flou, de dégradé ni d'animation continue ; la profondeur vient du seul écart entre le gris du fond et le blanc des cartes. Rejet fondateur : la grille sombre de cartes néon et ses bordures colorées partout.

Toutes les mesures de ce document sont en pixels logiques d'une maquette 1280 × 800, agrandie ×1,5 sur l'écran 1920 × 1200 de la tablette : une cible de 44 px logiques fait 66 px physiques.

**Key Characteristics:**
- Le verdict d'abord : une carte pleine largeur, teintée de l'état, sous le titre de l'accueil.
- Une seule élévation : la carte blanche (`surface`) sur le gris groupé (`fond`), sans bordure ni ombre.
- Une seule couleur d'action : le bleu `accent`.
- Trois tons d'état (up, warn, down), chacun en quatre rôles : plein, texte, fond, texte sur fond.
- Une forme par état : rond plein, carré, anneau, rond gris.
- Inter en casse de phrase, chiffres gras ; JetBrains Mono pour les adresses, IP et heures.
- Rien d'animé en continu ; tout se touche du doigt, cibles de 44 px au moins.

## Colors

Des gris froids d'app système, un bleu d'action, et trois tons d'état déclinés chacun en quatre rôles ; chaque couleur de texte tient au moins 4,5:1 sur le fond où elle est posée.

### Primary
- **Bleu signal** (`accent`, #2563EB) : la seule couleur d'action. Bouton principal plein, libellé et icône des boutons secondaires, lien actif de la barre latérale, capsule d'information (« Nouvelle version disponible »), compteur de mise à jour, puce « cette tablette ».
- **Voile bleu** (`accent-fond`, #EAF1FE) : fond des boutons secondaires, du lien actif de la barre latérale, de la note d'information et des capsules d'information ; le bleu y tient 4,56:1.

### Neutral
- **Gris groupé** (`fond`, #F2F2F6) : le sol de toutes les pages, sous les cartes ; aussi l'écran de démarrage et celui d'installation.
- **Blanc carte** (`surface`, #FFFFFF) : cartes, tuiles, barre latérale, fenêtre de confirmation ; texte des surfaces pleines (bouton principal, compteurs, bandeau hors ligne, toast).
- **Creux** (`creux`, #F2F2F6) : même valeur que `fond`, autre rôle : une zone en retrait à l'intérieur d'une carte (bouton neutre, puce grise, rond du verdict périmé, tuile d'icône d'un appareil).
- **Piste** (`piste`, #E6E6EB) : rail du contrôle segmenté, fond des barres de durée, jours sans donnée des barres 90 jours.
- **Filet** (`separateur`, #E5E5EA) : filets de 1 px entre lignes de liste, bord droit de la barre latérale, lignes de grille du graphe 24 h.
- **Encre** (`texte`, #1C1C1E) : texte principal, titres, valeurs ; base du toast, à 92 % d'opacité.
- **Gris légende** (`secondaire`, #636366) : lignes de détail, libellés de tuile, légendes, axes, métadonnées ; 5,36:1 sur `fond`, 4,81:1 sur `piste`.
- **Gris inerte** (`tertiaire`, #AEAEB2) : décor et inactif seulement (2,21:1 sur blanc) : curseur de défilement, sparkline d'un service en ligne, barre de durée d'un incident résolu, point « inconnu ».

### États
Chaque état a un ton (`theme::ton`) en quatre rôles : `plein` pour les points, barres et icônes ; `texte` pour un mot ou un chiffre sur blanc ; `fond` pour une surface teintée ; `texte-fond` pour le texte secondaire sur cette surface. `warn` couvre aussi « lent ». L'état inconnu n'a pas de ton propre : il reprend `tertiaire`, `secondaire` et `creux`.
- **Vert en ligne** : `up-plein` (#34C759), `up-texte` (#1B7F3B), `up-fond` (#E8F6EC), `up-texte-fond` (#17602E).
- **Ambre dégradé** : `warn-plein` (#FF9F0A), `warn-texte` (#A15C00), `warn-fond` (#FFF4E3), `warn-texte-fond` (#774400). `warn-plein` trace aussi les tirets du seuil « lent » du graphe.
- **Rouge hors ligne** : `down-plein` (#FF3B30), `down-texte` (#C8261B), `down-fond` (#FDECEA), `down-texte-fond` (#8F1D14). `down-texte` sert aussi de fond plein au bandeau « serveur injoignable » et au compteur d'incidents.

### Séries de courbes
`SERIES`, dans l'ordre des nodes de la config : **Cyan** (`series-0`, #0891B2), **Violet** (`series-1`, #9333EA), **Ardoise** (`series-2`, #64748B), **Encre** (`series-3`, #1C1C1E). Quatre familles nettement séparées, loin des tons d'état et du bleu d'action ; elles identifient une node (trait de 14 × 3 px devant son nom, courbe du graphe 24 h) et ne disent jamais un état.

### Veille
La palette `theme::nuit` ne sert qu'à l'écran de veille : **Noir chevet** (`nuit-fond`, #000000) ; **Blanc lune** (`nuit-texte`, #F2F2F7) pour l'heure ; **Gris nuit** (`nuit-secondaire`, #98989F) pour la date, le détail et les noms des services sains ; **Gris discret** (`nuit-discret`, #8E8E93) pour l'invite « Touche l'écran… » ; et les tons d'état éclaircis pour le noir : `nuit-up` (#30D158), `nuit-warn` (#FF9F0A), `nuit-down` (#FF453A).

### Named Rules
**La règle de la voix bleue.** Le bleu `accent` est la seule couleur d'action. Seule exception, voulue : sur une surface teintée d'un état (carte-verdict, carte d'incident), le lien ou le bouton d'action prend le texte de ce ton (« Voir le détail », « Acquitter »), pour ne pas poser du bleu sur du rouge.

**La règle des états réservés.** Le vert, l'ambre et le rouge ne disent qu'un état, jamais un décor ni une catégorie. Un état s'affiche en point plein (`*-plein`), en mot ou chiffre teinté sur blanc (`*-texte`), ou en surface teintée (`*-fond`) dont le texte secondaire prend `*-texte-fond`.

**La règle du calme.** Quand tout va bien, seul le verdict est vert. Les cartes des services en ligne restent blanches, leur sparkline est en `tertiaire`, les jours sans incident des barres 90 jours sont à 45 % de `up-plein`, et « Connecté » reste à l'encre.

**La règle du périmé incolore.** Serveur injoignable : l'état figé s'affiche sans teinte (points gris, textes en `secondaire`, verdict blanc « État inconnu »). Seul le bandeau rouge du haut parle.

## Typography

**Display Font:** Inter (installée sur la tablette, paquet `fonts-inter`)
**Body Font:** Inter
**Label/Mono Font:** JetBrains Mono (paquet `fonts-jetbrains-mono`)

**Character:** Une seule famille sans empattement en cinq graisses (légère, normale, moyenne, demi-grasse, grasse), lue de près comme une app système ; le mono n'apparaît que pour ce qui est une donnée machine.

### Hierarchy
- **Horloge de veille** (`horloge-veille`, légère 300, 176 px, interligne 1) : l'heure de la veille, seule au centre ; la date dessous en 24 px `nuit-secondaire`, le statut en 30 px demi-gras.
- **Display** (`display`, grasse 700, 34 px) : numéros de version de la page Mise à jour ; le bilan 30 jours des incidents monte à 40 px.
- **Horloge** (`horloge`, demi-grasse 600, 30 px, interligne 1) : l'heure en haut à droite de chaque page, la date en 13 px `secondaire` dessous.
- **Headline** (`headline`, grasse 700, 28 px) : titre de page, sur une ligne.
- **Chiffre** (`chiffre`, grasse 700, 26 px) : la valeur d'une tuile ; l'unité suit en 17 px demi-gras `secondaire`.
- **Verdict** (`verdict`, grasse 700, 22 px) : titre de la carte-verdict, titre de la fenêtre de confirmation, pourcentage 90 jours.
- **Title** (`title`, demi-grasse 600, 17 px) : titres de section et de carte (« Temps de réponse sur 24 h », nom d'un groupe de tags) ; la mesure d'une node prend la même taille, en gras.
- **Nom** (`nom`, demi-grasse 600, 16 px) : nom d'un service ou d'une node.
- **Body** (`body`, normale 400, 15 px) : texte courant, messages, paragraphes de la fenêtre de confirmation.
- **Bouton** (`bouton`, demi-grasse 600, 15 px) : libellés de boutons, de liens et du segment choisi ; un lien de barre latérale passe de moyen (500) à demi-gras quand il est actif.
- **Meta** (`meta`, normale 400, 14 px) : ligne de détail sous un titre, résumé d'un groupe, issue d'un événement.
- **Pastille** (`pastille`, demi-grasse 600, 14 px) : libellé d'une capsule d'état et du bandeau hors ligne.
- **Label** (`label`, moyenne 500, 13 px) : libellé d'une tuile, note d'une carte de service.
- **Légende** (`legende`, normale 400, 13 px) : légendes de tuile et de graphe, nombre de nodes, version en pied de barre latérale.
- **Axe** (`axe`, normale 400, 12 px) : graduations du graphe 24 h, légende des barres 90 jours ; les puces le prennent en demi-gras.
- **Mono** (`mono`, JetBrains Mono 400, 13 px, de 12,5 à 13,5 px selon la ligne) : adresses, IP, cibles, chemins, heures des listes d'événements.

### Named Rules
**La règle du chiffre gras.** Une valeur est toujours plus grande et plus grasse que son libellé ; l'unité la suit, plus petite et en gris (« 99,97 % », « 1 ms »).

**La règle de la casse de phrase.** Titres, libellés et boutons en casse de phrase, sans capitales ni lettres espacées ; seuls les sigles (TCP, HTTP, DERP) restent en capitales.

**La règle de la ligne unique.** Une ligne de métadonnées tient sur une ligne : au plus un « · » pour séparer deux faits, et ce qui déborde est abrégé avec « … » (`format::abreger`) ou coupé par son conteneur, jamais renvoyé à la ligne.

**La règle du mono pour les données.** JetBrains Mono est réservé à ce qu'on pourrait copier dans un terminal : adresse, IP, chemin, heure d'événement. Jamais pour un libellé.

## Layout

Un canevas fixe : la maquette 1280 × 800 est mise à l'échelle de la fenêtre (`scale_factor`, ×1,5 sur la tablette), sans point de rupture ni mise en page de repli. Paysage seulement, tactile seulement.

- **Squelette** : barre latérale blanche de 220 px à gauche (marges 22 et 14 px, séparée du contenu par un filet de 1 px) ; contenu à droite, à 24 px du haut et du bas et 28 px des côtés.
- **En-tête de page** : titre 28 px (suivi de sa capsule d'état éventuelle) et sa ligne de détail 14 px ; actions éventuelles ; horloge de 170 px calée à droite ; 16 px entre ces éléments.
- **Accueil** : une pile espacée de 18 px : en-tête, carte-verdict pleine largeur, rangée de quatre tuiles égales (écart 14 px), puis les services groupés par tag, les groupes à problème en tête, 22 px entre groupes, en grille de 3 colonnes (écart 12 px) ; une rangée incomplète garde ses cases vides pour ne pas étirer les cartes.
- **Détail d'un service** : deux colonnes défilantes en proportion 5:7, écart 20 px : les nodes à gauche ; la courbe 24 h, les 90 jours et les événements à droite, cartes espacées de 14 px.
- **Rythme** : un pas de 2 px, de 2 à 22 px ; titre de section vers contenu 10 px, éléments d'une carte 8 px, libellé, chiffre et légende d'une tuile 4 px.
- **Défilement** : au doigt ; un curseur de 4 px en `tertiaire`, sans rail, posé sur le contenu pour que la largeur utile ne change pas.

### Named Rules
**La règle des 44.** Toute cible tactile fait au moins 44 px de haut (`commun::CIBLE`, 66 px physiques) ; les liens de la barre latérale font 48 px, une ligne d'événement 44 px.

## Elevation & Depth

Le système est plat. Une seule élévation : la carte blanche posée sur le gris groupé, sans bordure ni ombre ; la profondeur vient de cet écart de valeur et de rien d'autre. Une zone en retrait dans une carte descend vers `creux`. Le rendu logiciel exclut ombres, flous et dégradés : ce n'est pas un style à contourner.

Trois surfaces font exception, toutes sans ombre :
- la fenêtre de confirmation, même matière blanche (rayon 20 px), posée sur un voile noir à 32 % qui couvre toute la page ;
- le toast, capsule d'encre à 92 % d'opacité, centrée à 28 px du bas, affichée 2,5 s puis retirée sans fondu ;
- le bandeau hors ligne, bande pleine `down-texte` bord à bord, au-dessus de tout.

### Shadow Vocabulary
Aucune ombre : le système n'a pas de `box-shadow`.

### Named Rules
**La règle de la carte unique.** La carte blanche est la seule élévation : pas de bordure, pas d'ombre. Sous le doigt, un élément s'assombrit (son fond × 0,9, ou un voile noir à 6 % s'il n'a pas de fond) ; désactivé, il passe à 40 % d'opacité. Rien ne bouge entre deux états.

## Shapes

Des rectangles aux grands rayons, des capsules et des ronds ; aucune bordure, sauf l'anneau de l'état hors ligne.

- **Rayons** : 16 px pour les cartes, tuiles, verdict et cartes d'incident (`carte`) ; 14 px pour les cartes plus denses : carte de service, alerte, note d'information (`dense`) ; 12 px pour les boutons et les liens de la barre latérale (`bouton`) ; 10 px pour les segments, les liens nus et les tuiles d'icône (`controle`) ; 8 px pour les puces (`puce`) ; 20 px pour la fenêtre (`fenetre`) ; 2 px pour les barres des 90 jours et les traits de série (`barre`).
- **Capsules et ronds** : rayon égal à la moitié de la hauteur (capsule d'état 30 px, compteur, toast, bouton retour 44 px, rond du verdict 44 px, rond d'incident 40 px).
- **Rayons concentriques** : le rail segmenté (13 px) entoure des segments à 10 px avec une marge de 3 px ; les lignes d'une liste groupée n'arrondissent que leurs coins au bord de la carte, à 16 px (`ligne_de_liste`).
- **Icônes** : tracées au trait dans une grille 24 × 24 (`icones.rs`), bouts et jointures arrondis, trait de 1,8 à 2,4, affichées de 18 à 22 px ; ni icône pleine, ni caractère utilisé comme icône.
- **Courbes** : traits de 2 px à bouts ronds ; un trou de mesure coupe la ligne ; le seuil « lent » est en tirets de 6 et 5 px ; une node hors ligne est marquée d'une croix dans un cercle rouge sur sa dernière mesure.

### Named Rules
**La règle d'une forme par état.** Chaque état a sa forme de point (`commun::point`), pour ne jamais dépendre de la couleur seule : rond plein (en ligne), carré aux coins de 25 % (dégradé ou lent), anneau au trait de 22 % de la taille, 2 px au moins (hors ligne), rond gris (inconnu). Les grandes icônes d'état suivent : coche, triangle, croix, trois points.

## Components

### Buttons
Francs et calmes : 44 px de haut, rayon 12 px, libellé demi-gras 15 px, icône au trait de 20 px à 8 px du libellé.
- **Shape:** rectangle doux (12 px).
- **Principal** (`bouton-principal`) : plein `accent`, texte blanc, marges latérales 18 px. Réservé à l'action qui engage (« Mettre à jour », « Oui, mettre à jour »).
- **Secondaire** (`bouton`) : fond `accent-fond`, texte et icône `accent`, marges 16 px (« Relancer les contrôles », « Vérifier maintenant »).
- **Neutre** (`bouton-neutre`) : fond `creux`, texte à l'encre ; pour renoncer (« Annuler »).
- **Bouton de carte** (`bouton-carte`) : fond blanc sur une carte teintée ; libellé à l'encre (« Voir ») ou dans le ton de la carte (« Acquitter »).
- **Lien** (`lien-down` et ses pareils) : sans fond, texte et chevron dans le ton de la surface teintée qui le porte (« Voir le détail »), rayon 10 px, marges 12 px.
- **Retour** (`bouton-retour`) : rond blanc de 44 px, chevron à l'encre.
- **Pressé / désactivé :** fond assombri à 90 % sous le doigt ; 40 % d'opacité quand l'action est indisponible. Ni survol ni focus visible : l'écran n'a ni souris ni clavier.

### Chips
- **Capsule d'état** (`pastille-up`, `pastille-warn`, `pastille-down`) : 30 px de haut, fond `*-fond`, point de 9 px à la forme de l'état puis libellé demi-gras 14 px en `*-texte` ; posée à côté d'un titre de page.
- **Capsule d'information** (`pastille-info`) : même capsule en `accent-fond` et `accent`, sans point.
- **Puce** (`puce`, `puce-moi`) : étiquette 12 px demi-grasse, rayon 8 px ; grise sur `creux` (« routeur de sous-réseau »), bleue sur `accent-fond` pour « cette tablette ».
- **Compteur** (`compteur-down`, `compteur-info`) : capsule pleine dans la barre latérale, chiffre gras 13 px blanc, sur le `*-texte` du pire incident ouvert ou sur `accent` pour une mise à jour prête.

### Cards / Containers
- **Corner Style:** 16 px (`carte`) ; 14 px pour les cartes denses (`dense`).
- **Background:** `surface` ; `*-fond` quand la carte porte un état en alerte.
- **Shadow Strategy:** aucune (voir Elevation & Depth).
- **Border:** aucune. Dans une liste, les lignes sont séparées par des filets `separateur` de 1 px qui commencent au début du texte, pas au bord de la carte.
- **Internal Padding:** 16 × 20 px pour une carte ; 14 × 18 px pour une tuile, le verdict et une ligne de node ; 14 × 16 px pour une carte de service.
- **Tuile chiffrée** (`tuile`) : libellé 13 px moyen gris, chiffre 26 px gras, légende 13 px ; touchable quand elle mène à une page (incidents, Tailscale). Le chiffre prend `*-texte` quand il dit un problème.
- **Carte de service** (`carte-service`) : point d'état 10 px, nom 16 px demi-gras (abrégé à 20 caractères), latence à droite ; adresse en mono ; sparkline 24 h de 22 px ; nombre de nodes et note en pied. Blanche en ligne, `warn-fond` ou `down-fond` en alerte, ses textes secondaires passant alors en `*-texte-fond`.
- **Carte d'incident** : fond `*-fond`, rayon 16 px, rond blanc de 40 px portant l'icône d'état, titre en `*-texte`, boutons de carte alignés sous le texte.
- **Alerte** (`alerte`) : bandeau `down-fond` de rayon 14 px, icône info `down-texte`, message `down-texte-fond`, en haut d'une page dont l'appel a échoué.
- **Note d'information** (`note-info`) : `accent-fond`, icône info bleue, texte à l'encre ; explique une situation sans être un état.

### Navigation
- **Barre latérale** (`barre-laterale`, `nav-lien`, `nav-lien-actif`) : blanche, 220 px ; titre « Monitoring » 20 px gras ; liens de 48 px (icône 22 px, libellé 15 px) ; Veille et numéro de version en pied. Lien actif : fond `accent-fond`, icône et texte `accent`, demi-gras. Inactif : icône `secondaire`, texte à l'encre, graisse moyenne.
- **Contrôle segmenté** (`segment-rail`, `segment`, `segment-actif`) : rail `piste` de 50 px, rayon 13 px, marge 3 px ; segments de 112 × 44 px (la règle des 44) ; le segment choisi est blanc, texte à l'encre demi-gras ; les autres sont sans fond, en `secondaire`.

### Carte-verdict
La signature du système. Pleine largeur sous l'en-tête de l'accueil : rond de 44 px avec la grande icône d'état, titre 22 px gras, une ligne de détail 15 px, et « Voir le détail » à droite quand un problème a sa page. Teintée (`verdict-up`, `verdict-warn`, `verdict-down` : titre `*-texte`, détail `*-texte-fond`, rond blanc) quand l'état est connu ; blanche (`verdict-perime` : titre à l'encre, détail `secondaire`, rond `creux`) quand il est inconnu ou périmé.

### Courbes et barres
- **Sparkline** : 22 px de haut, trait de 2 px ; `tertiaire` quand tout va bien, `*-plein` quand le service est en alerte ; aucune courbe sous deux mesures, plutôt qu'un trait qui imiterait une donnée.
- **Graphe 24 h** : 200 px de haut ; une courbe par node, en `SERIES` ; trois lignes de grille `separateur` ; graduations et axe du temps en 12 px `secondaire` ; légende en traits de série au-dessus.
- **Barres 90 jours** : une barre de 30 px par jour, rayon 2 px, écart 2 px ; `piste` sans donnée, `up-plein` à 45 % les jours sans incident, `*-plein` les jours à problème.
- **Barres de durée** : 8 px sur une piste `piste`, rayon 4 px ; `*-plein` pour un incident en cours, `tertiaire` une fois résolu.

### Bandeau hors ligne
Bande pleine `down-texte`, bord à bord au-dessus de toute la page, texte blanc demi-gras 14 px centré : depuis quand l'affichage est figé, et la cadence de reprise. C'est la seule voix de l'état périmé.

### Veille
Écran noir façon horloge de chevet : heure de 176 px légère au centre, date dessous, puis le statut (icône d'état et titre 30 px) et son détail 18 px ; en pied, une rangée de points d'état et de noms de services, et l'invite à toucher. Un filet de 6 px dans la couleur d'état barre le haut quand quelque chose ne va pas. Un toucher n'importe où ouvre l'accueil.

## Do's and Don'ts

### Do:
- **Toujours** poser le verdict en tête de l'accueil, pleine largeur, teinté de l'état.
- **Toujours** dire un état par sa forme et sa couleur (rond plein, carré, anneau, rond gris, via `commun::point`), ou par un libellé.
- **Toujours** prendre `*-texte-fond` pour le texte secondaire posé sur un fond `*-fond`, et `*-texte` pour un mot d'état sur blanc.
- **Toujours** afficher l'état périmé sans teinte, et laisser le bandeau hors ligne le dire.
- **Toujours** abréger avec « … » (`format::abreger`) plutôt que renvoyer une métadonnée à la ligne.
- **Toujours** donner au moins 44 px de haut à ce qui se touche.
- **Toujours** réserver la palette `nuit` à la veille.

### Don't:
- **Jamais** d'ombre, de flou, de dégradé ni d'animation continue : le rendu est logiciel, sans GPU.
- **Jamais** de bordure sur une carte, ni de bordure colorée pour dire un état : la grille sombre de cartes néon est le rejet fondateur.
- **Jamais** de vert, d'ambre ou de rouge pour décorer ou catégoriser, ni pour une action hors d'une surface teintée de cet état ; le bleu est la seule couleur d'action.
- **Jamais** une couleur de `SERIES` pour dire un état, ni un ton d'état pour tracer une node.
- **Jamais** plus d'un « · » dans une ligne de métadonnées.
- **Jamais** de libellé en capitales ou en lettres espacées.
- **Jamais** de texte en `tertiaire`.
- **Jamais** de courbe ni de chiffre inventé quand la mesure manque : « — », « … » ou pas de courbe.
