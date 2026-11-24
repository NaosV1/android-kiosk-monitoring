# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

<!-- Aucune valeur du schema ne correspond exactement : l'interface est une application native
Rust (iced) plein ecran sous Linux/X11, sur une tablette Android. Elle ne suit le langage visuel
d'aucun systeme (ni iOS, ni Android) ; "web" signifie ici « interface autonome, sans conventions d'OS ». -->

## Users

Une seule personne : le propriétaire de l'installation, technicien (développeur / administrateur
de ses propres services). La tablette est posée sur son bureau, à portée de main, à côté de son écran
de travail : il la lit de près, d'un coup d'œil, plusieurs fois par jour, et la touche souvent pour
creuser un problème. Le vocabulaire technique (node, DERP, exit node, TCP, HTTP 503) lui est familier.

## Product Purpose

Surveiller en continu ses services (HTTP(S), port TCP, ping, DNS ; services à plusieurs nodes ;
réseau privé joint par Tailscale) et le dire immédiatement : **tout va bien, ou ce qui est cassé**.
Le verdict passe avant tout ; latences, historique (24 h, 30 et 90 jours) et incidents viennent ensuite,
au toucher. Réussir, c'est ne jamais avoir à chercher l'état global, et savoir en deux secondes quoi
regarder quand quelque chose ne va pas.

## Positioning

Un écran de supervision physique, dédié et toujours allumé, verrouillé en kiosque sur une tablette
Android bon marché, sans root ni serveur externe : les contrôles tournent sur la tablette elle-même,
y compris vers des réseaux privés via Tailscale, et l'historique y reste.

## Operating Context

- Lenovo Tab K10, écran 1920 × 1200 en paysage ; maquette logique 1280 × 800 agrandie ×1,5.
- Debian dans Termux (proot), affichage par Termux:X11 ; **pas de GPU** : rendu logiciel (tiny-skia),
  donc pas d'animation continue, de flou, d'ombre ni de dégradé coûteux.
- Tactile uniquement (pas de souris ni de clavier) ; défilement au doigt.
- Écran de veille après quelques minutes sans toucher ; il se rallume sur tout nouvel incident.
- Mises à jour proposées à l'écran, installées seulement après appui et confirmation.

## Capabilities and Constraints

- Pages : accueil (verdict, services groupés par tag), détail d'un service (nodes, courbe 24 h,
  disponibilité 90 jours, événements), Tailscale (tablette, exit node, appareils), incidents (en cours,
  résolus, acquittement, bilan 30 jours), mise à jour, veille.
- Les données viennent de l'API JSON locale du serveur (`monitoring/src/api.rs`) ; l'interface ne
  calcule pas d'état elle-même.
- Interface en français.
- Polices : seules celles installées sur la tablette ou embarquées dans le binaire sont disponibles.
- Implémentation : iced 0.14 (widgets, `canvas` pour les icônes et les courbes), un seul binaire.

## Evidence on Hand

Aucune donnée réelle partageable : les captures et démonstrations utilisent des données fictives,
à présenter comme telles.

## Product Principles

1. Le verdict d'abord : l'état global se lit sans chercher ni toucher.
2. Calme quand tout va bien, net quand ça casse : la couleur et l'insistance sont réservées aux problèmes.
3. Chaque élément affiché est une donnée vraie ; rien de décoratif qui imite de l'information.
4. Tactile à portée de bras : cibles généreuses, une action par geste, rien qui exige de la précision.
5. Sobre à dessiner : ce que le processeur de la tablette peut rendre sans saccade.

## Accessibility & Inclusion

L'état ne repose jamais sur la couleur seule (forme ou libellé en plus).
