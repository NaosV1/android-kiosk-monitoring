# Tablette de monitoring

Une tablette Android transformée en écran de supervision tactile, verrouillé en plein écran.
Debian tourne dans Termux (proot, **sans root**). Un seul binaire Rust contrôle les services et affiche
l'interface tactile native, sans navigateur.

Testé sur une Lenovo Tab K10 (Android 12, écran 1920 × 1200).

![Accueil](docs/accueil.png)

| Détail d'un service | Écran de veille |
|---|---|
| ![Service](docs/service.png) | ![Veille](docs/veille.png) |

*Captures d'une instance de démonstration.*

## Ce que ça fait

- **Contrôles** : HTTP(S) (code attendu, texte attendu, champs JSON attendus), port TCP, ping ICMP
  (sans root) et résolution DNS.
- **Services à plusieurs nodes** : le service est *dégradé* si une node tombe, *hors ligne* si toutes tombent.
  Chaque serveur derrière un même nom peut être interrogé à part (`ip = "…"`, comme `curl --resolve`).
- **Réseaux privés** : résolution par un DNS précis (`resolveur`) et vérification que le contrôle passe
  bien par le tunnel Tailscale (`par_tailscale`), par exemple pour un réseau d'entreprise joint par
  une route de sous-réseau.
- **Tags** : les services sont regroupés par tag sur l'accueil, et les problèmes passent en tête de leur groupe.
  Tout tient sur un écran.
- **Historique** (SQLite) : courbe des temps de réponse sur 24 h, disponibilité sur 30 et 90 jours,
  journal des incidents avec acquittement.
- **Tailscale** : état de la tablette sur le tailnet, alerte si un exit node est activé, et liste des
  appareils si une clé API (lecture) est fournie.
- **Écran de veille** après quelques minutes sans toucher l'écran. Il se rallume sur tout nouvel incident.
- **Verrouillage** : une petite appli Android *device owner* (« Kiosque ») bloque la tablette sur l'écran
  Linux (pas d'accueil Android, pas de notifications). On la pilote en SSH.
- **Mises à jour** : chaque push sur `main` est vérifié puis publié en release. La tablette télécharge
  la release et l'installe quand on appuie sur « Mettre à jour » (avec confirmation). Voir [plus bas](#mises-à-jour-cicd).

Rien à installer côté exécution : le binaire contient le serveur, l'interface et SQLite.

## Comment c'est monté

```
Android
├── Termux ............. sshd (8022 : Termux, 2222 : Debian), Termux:Boot au démarrage
│   ├── Termux:X11 ..... serveur X plein écran (:0)
│   └── proot Debian
│       ├── kiosk-session ........ gestionnaire de fenêtres + relance de l'appli
│       ├── monitoring serveur ... contrôles, historique, mises à jour, API JSON (127.0.0.1:8080)
│       └── monitoring ecran ..... interface tactile plein écran, cliente de l'API
└── Kiosque (device owner) ... verrouillage sur Termux:X11 (lock task)
```

L'interface est dessinée par le processeur (iced avec le moteur de rendu logiciel tiny-skia) :
la tablette n'a pas d'accélération GPU sous proot.

```
monitoring/            le monitoring (Rust) : un binaire, deux sous-commandes
  src/api.rs           contrat JSON entre le serveur et l'écran
  src/config.rs        lecture de config.toml
  src/controles/       contrôles HTTP, TCP, ping, DNS, route Tailscale
  src/moniteur/        états, incidents, historique SQLite, rapports
  src/maj.rs           mises à jour depuis les releases GitHub
  src/serveur_http.rs  API JSON
  src/ecran/           interface tactile
  tests/               serveur réel, lancer.sh (passage de version, retour arrière)
  lancer.sh            démarre le serveur puis l'écran, revient à la version précédente si besoin
  config.example.toml  exemple commenté de toutes les options
tablette/termux/       scripts côté Termux (démarrage, SSH, commande kiosk)
tablette/debian/       scripts côté Debian (session X, relance de l'appli)
kiosque/               sources de l'appli Android de verrouillage (Java, compilée sur la tablette)
.github/workflows/     CI : vérifications puis release à chaque push sur main
VERSION                version majeure.mineure des releases (le dernier chiffre est ajouté par la CI)
```

## Installation

Les étapes ci-dessous résument ce qui a été fait sur la tablette. Elles supposent un accès
`adb` depuis un PC.

### 1. Applis Android

Installer **Termux**, **Termux:X11** et **Termux:Boot**, depuis F-Droid ou les versions GitHub (pas le Play Store).
Ouvrir Termux:Boot une fois pour qu'il s'active.

### 2. Termux

```sh
pkg install openssh proot-distro x11-repo termux-tools
pkg install termux-x11-nightly
proot-distro install debian
```

Copier les scripts :

| Fichier du dépôt | Destination (Termux) |
|---|---|
| `tablette/termux/linux-start`, `linux-stop`, `debian-ssh`, `plein-ecran-auto` | `~/bin/` |
| `tablette/termux/kiosk` | `$PREFIX/bin/kiosk` |
| `tablette/termux/boot/00-linux` | `~/.termux/boot/00-linux` |
| `tablette/termux/sshd_config.extrait` | à ajouter à `$PREFIX/etc/ssh/sshd_config` |

Ajouter ta clé publique SSH dans `~/.ssh/authorized_keys` (Termux) et dans `/root/.ssh/authorized_keys` (Debian).
La connexion par mot de passe est désactivée.

### 3. Debian

```sh
proot-distro login debian
apt install fonts-inter fonts-jetbrains-mono libxkbcommon-x11-0 curl \
            matchbox-window-manager x11-utils x11-xserver-utils openssh-server locales
```

| Fichier du dépôt | Destination (Debian) |
|---|---|
| `tablette/debian/kiosk-session`, `kiosk-restart` | `/usr/local/bin/` |
| `tablette/debian/kiosk-app.sh` | `/root/kiosk-app.sh` |

Le monitoring s'installe depuis la dernière [release](https://github.com/NaosV1/android-kiosk-monitoring/releases)
(binaire Linux aarch64) :

```sh
mkdir -p /root/monitoring && cd /root/monitoring
curl -fLO https://github.com/NaosV1/android-kiosk-monitoring/releases/download/vX.Y.N/monitoring-X.Y.N.tar.gz
tar -xzf monitoring-X.Y.N.tar.gz --strip-components=1 && rm monitoring-X.Y.N.tar.gz
cp config.example.toml config.toml   # puis mettre tes services
```

### 4. Appli Kiosque (verrouillage)

Elle se compile directement dans Termux :

```sh
pkg install aapt apksigner d8 openjdk-21
cp -r kiosque ~/kiosque-src && bash ~/kiosque-src/build.sh    # produit ~/kiosque.apk
```

`build.sh` génère un jeton de contrôle et une clé de signature dans `~` (`.kiosk-token`, `.kiosk-keypass`,
`.kiosk.keystore`). Ces fichiers restent sur la tablette.

Depuis le PC (la tablette ne doit avoir **aucun compte** configuré pour devenir *device owner*) :

```sh
adb install -t kiosque.apk
adb shell dpm set-device-owner fr.tablette.kiosque/.Admin
```

Redémarrer la tablette : tout se lance seul et l'écran est verrouillé sur le monitoring.

## Utilisation

```sh
ssh -p 2222 root@<tablette>          # Debian
kiosk-restart                        # relance l'appli après une modification de config.toml

ssh -p 8022 <tablette> kiosk status        # état du verrouillage
ssh -p 8022 <tablette> kiosk unlock        # rend Android (Wi-Fi, réglages...)
ssh -p 8022 <tablette> kiosk lock          # reverrouille
ssh -p 8022 <tablette> kiosk release       # retire définitivement le mode kiosque
ssh -p 8022 <tablette> kiosk plein-ecran   # remasque les barres Android
```

Pour voir l'interface depuis un PC : `ssh -L 8080:127.0.0.1:8080 -p 2222 root@<tablette>`, puis, dans
`monitoring/` sur le PC, `cargo run --release -- ecran http://127.0.0.1:8080`.

Journaux : `/root/monitoring/serveur.log` et `ecran.log` (Debian), `~/linux-start.log` et `~/kiosk.log` (Termux).

## Configuration

Tout est dans `monitoring/config.toml`. Le fichier [`config.example.toml`](monitoring/config.example.toml)
décrit chaque option, avec un exemple pour chacune. En bref :

```toml
[[service]]
nom = "API"
tag = "Mes services"
adresse = "api.exemple.fr/health"
nodes = [
  { nom = "srv1", url = "https://api.exemple.fr/health", ip = "203.0.113.10", json = { status = "ok" } },
  { nom = "srv2", url = "https://api.exemple.fr/health", ip = "203.0.113.20", json = { status = "ok" } },
]
```

## Mises à jour (CI/CD)

```
push sur main ──► GitHub Actions : vérifications ──► release vX.Y.N ──► la tablette la télécharge
                                                                          │
                                         bouton « Mettre à jour » + confirmation ──► installation
```

**À chaque push sur `main`**, le workflow [`ci.yml`](.github/workflows/ci.yml) vérifie, sur une machine
aarch64 avec la même Debian que la tablette :

- **Rust** : formatage, `clippy` sans avertissement, tests (dont un vrai serveur qui démarre et répond,
  et les scénarios de `lancer.sh` : passage de version et retour arrière), puis compilation du binaire ;
- **shell** : la syntaxe de tous les scripts ;
- **Java** : l'appli Kiosque compile contre le SDK Android.

Si tout passe **et** que `monitoring/` ou `VERSION` a changé depuis la dernière release, il publie
une release `vX.Y.N` :

- `X.Y` vient du fichier `VERSION`. Modifie-le pour changer de version majeure ou mineure.
- `N` est le numéro du run de la CI.
- La release contient `monitoring-X.Y.N.tar.gz` et `SHA256SUMS`. Le contenu de l'archive est un contrat
  avec les tablettes déjà installées : il est décrit en tête de [`ci.yml`](.github/workflows/ci.yml).

Un push qui ne touche que le README ne crée pas de release.

**Sur la tablette**, le serveur interroge l'API GitHub toutes les 6 h. Le bouton « Vérifier maintenant »
de la page *Mise à jour* lance la même vérification tout de suite.

Quand une release plus récente existe, la tablette :

1. télécharge l'archive et vérifie son empreinte SHA-256 ;
2. contrôle son contenu (ni chemin qui sort du dossier, ni lien) ;
3. affiche une pastille sur « Mise à jour » dans le menu, avec les notes de version.

**Rien n'est installé sans appui sur « Mettre à jour » puis confirmation.** À l'installation :

- le nouveau binaire est d'abord lancé une fois (`--version`) : s'il ne s'exécute pas sur la tablette,
  rien n'est remplacé ;
- `config.toml`, `data.db` et les journaux ne sont pas touchés ;
- la version en place est sauvegardée dans `monitoring/.maj/precedente/` ;
- l'appli redémarre, puis affiche « Mise à jour installée ».

Si la nouvelle version ne démarre pas (serveur qui ne répond pas, serveur ou écran qui s'arrête dans les
15 premières secondes), `lancer.sh` remet automatiquement la précédente et la page *Mise à jour* indique
pourquoi. Pour revenir à la main à la version précédente :

```sh
cd /root/monitoring
for f in .maj/precedente/*; do [ -f "$f" ] && cp -p --remove-destination "$f" .; done
[ -d .maj/precedente/web ] && rm -rf web && cp -rp .maj/precedente/web web   # retour à la version Python
kiosk-restart
```

Configuration (section `[mise_a_jour]` de `config.toml`) :

```toml
[mise_a_jour]
depot = "NaosV1/android-kiosk-monitoring"   # "" = désactivé
verifier_toutes_les_heures = 6
```

### Passage de la version Python (1.0.x) à la version Rust (2.x)

La première release Rust s'installe par le bouton « Mettre à jour » de la version Python, comme les
autres : l'archive contient ce que l'installateur Python exige, et le nouveau `lancer.sh` termine
l'installation en prenant le binaire dans l'archive déjà vérifiée. La configuration et l'historique sont
repris tels quels.

**Avant d'appuyer sur « Mettre à jour »**, installer la seule bibliothèque nouvelle de l'écran :

```sh
ssh -p 2222 root@<tablette> apt install -y libxkbcommon-x11-0
```

Sans elle, l'écran Rust ne démarre pas : `lancer.sh` remet alors la version Python et la page *Mise à jour*
affiche la commande à lancer. Python et Chromium peuvent rester installés : ils servent au retour arrière.

**Debian 12 (bookworm)** : l'installateur Python 1.0.x utilise une option de `tarfile` absente de Python 3.11.2,
la version de Debian 12. Il télécharge et vérifie la release, mais « Mettre à jour » échoue
(`Installation impossible : … 'filter'`) sans rien modifier. Debian 13 (trixie, Python 3.13) n'est pas concerné.
Pour savoir ce qu'il en est : `ssh -p 2222 root@<tablette> 'cat /etc/debian_version; python3 --version'`.
Sur Debian 12, faire à la main ce que l'installateur aurait fait, une fois la release téléchargée par la tablette
(page *Mise à jour* : « Nouvelle version disponible ») ; `lancer.sh` termine ensuite l'installation, avec le
même retour arrière automatique :

```sh
cd /root/monitoring && V=X.Y.N                       # la version proposée par la tablette
mkdir -p .maj/precedente && cp -p serveur.py lancer.sh config.example.toml VERSION .maj/precedente/ \
  && rm -rf .maj/precedente/web && cp -rp web .maj/precedente/web
echo "$V" > .maj/a-confirmer
tar -xzf ".maj/monitoring-$V.tar.gz" --strip-components=1 \
  "monitoring-$V/lancer.sh" "monitoring-$V/VERSION" "monitoring-$V/config.example.toml"
kiosk-restart
```

Les mises à jour suivantes sont faites par la version Rust, qui n'a pas ce problème.

Test facultatif avant la mise à jour, sans rien changer à l'installation : l'écran Rust sait afficher le
serveur Python en place.

```sh
ssh -p 2222 root@<tablette>
cd /tmp && curl -fLO https://github.com/NaosV1/android-kiosk-monitoring/releases/download/vX.Y.N/monitoring-X.Y.N.tar.gz
tar -xzf monitoring-X.Y.N.tar.gz && cd monitoring-X.Y.N
./monitoring --version                                   # le binaire s'exécute sur la tablette
DISPLAY=:0 ./monitoring ecran http://127.0.0.1:8080      # l'écran Rust s'ouvre (Ctrl+C pour le fermer)
```

## Développement

```sh
cd monitoring
cargo test                                               # tests unitaires et d'intégration
MONITORING_CONFIG=config.example.toml MONITORING_DB=/tmp/data.db cargo run -- serveur
cargo run -- ecran http://127.0.0.1:8080                 # l'écran sur le serveur ci-dessus
```

Les ICMP « datagramme » (ping sans root) doivent être autorisés par `net.ipv4.ping_group_range` sous Linux.

## Tailscale (facultatif)

- Installer l'appli Android Tailscale, la mettre en **VPN permanent** et l'exclure de l'optimisation de la batterie.
- Dans Debian, mettre `nameserver 100.100.100.100` en tête de `/etc/resolv.conf` pour utiliser MagicDNS.
- Dans `config.toml`, section `[tailscale]` : `actif = true`. Pour la liste des appareils, ajouter une clé API
  dans `cle_api`. Une clé en lecture suffit.
- Pour joindre un réseau privé **sans** faire passer tout le trafic par un exit node, il faut que la machine de
  ce réseau annonce une route de sous-réseau. On marque ensuite les nodes concernées avec `par_tailscale = true`.

## Limites

Debian tourne dans proot, sans root Android :

- pas de systemd : tout est lancé par des scripts ;
- pas de ports en dessous de 1024 ;
- pas de Docker ni d'accélération GPU : l'interface est rendue par le processeur.

Termux:X11 laisse parfois les barres Android visibles quand la fenêtre de l'appli est recréée. `plein-ecran-auto` les
remasque à chaque relance de l'appli.

L'appli Kiosque reste en Java : Android exige une classe Java pour le composant *device owner*
(`DeviceAdminReceiver`), et changer ce composant obligerait à réinitialiser la tablette.

## Licence

MIT, voir [LICENSE](LICENSE).
