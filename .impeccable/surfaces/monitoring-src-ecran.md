---
version: 1
slug: "monitoring-src-ecran"
primary_target: "monitoring/src/ecran"
related_targets: []
---

# Écran tactile du monitoring

Portée : toute l'interface `monitoring ecran` (accueil, service, Tailscale, incidents, mise à jour, veille).
Mode : operate. Public : le propriétaire seul, tablette sur son bureau, lue de près, touchée souvent.
Étalon de finition choisi par l'utilisateur : Apple Maison / Santé.

## Direction contract

THESIS : le verdict d'abord, posé comme une app système de tablette ; refuse la grille sombre de cartes
néon et ses bordures colorées partout.

OWN-WORLD : fond gris groupé clair, cartes blanches aux grands rayons sans bordure, filets gris fins ;
une seule couleur d'action (bleu) ; vert, ambre, rouge réservés aux états, en pastille + teinte légère ;
Inter, gros chiffres gras, libellés gris en casse de phrase ; StandBy noir pour la veille.

STORY : en un regard, « tout va bien » ou le service cassé et depuis quand ; un toucher ouvre le
détail (nodes, courbe 24 h, 90 jours) puis permet de relancer, acquitter ou mettre à jour.

FIRST VIEWPORT : barre latérale 220 px (navigation, pastilles) ; en-tête titre + résumé, horloge à droite ;
carte-verdict pleine largeur ; rangée de 4 tuiles chiffrées ; services en cartes, 3 colonnes, par tag,
problèmes en tête.

FORM : canon (catégorie standard, choisie par l'utilisateur), hors liste ; seed:52152e08b07fab44.

FINISH : unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
