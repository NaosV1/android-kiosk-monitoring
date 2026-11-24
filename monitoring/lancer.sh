#!/bin/bash
# Lance le serveur de monitoring puis l'ecran tactile : un meme binaire, deux processus.
# Appele par /root/kiosk-app.sh ; kiosk-session relance ce script des qu'il s'arrete.
#
# Apres une mise a jour (.maj/a-confirmer present), la nouvelle version doit faire ses preuves :
# si le serveur ne repond pas, ou si le serveur ou l'ecran s'arrete dans les premieres secondes,
# la version precedente (.maj/precedente) est remise en place et l'appli relancee.
cd "$(dirname "$(readlink -f "$0")")" || exit 1
export DISPLAY="${DISPLAY:-:0}"

MARQUE=.maj/a-confirmer
PRECEDENTE=.maj/precedente
OBSERVATION_ECRAN=15        # secondes pendant lesquelles une nouvelle version doit tenir
VERSION=$(tr -d ' \r\n' < VERSION 2>/dev/null)
SERVEUR=
ECRAN=

horodatage() { date '+%F %T'; }

arreter() {
  kill $SERVEUR $ECRAN 2>/dev/null
  wait 2>/dev/null
}
trap arreter EXIT

retour_arriere() {
  echo "$(horodatage) la version $(cat "$MARQUE" 2>/dev/null) n'a pas démarré ($1) : retour à la version précédente" \
    | tee -a serveur.log > .maj/retour-arriere
  local f nom
  for f in "$PRECEDENTE"/*; do
    [ -f "$f" ] || continue
    nom=$(basename "$f")
    # copie puis renommage : remplace aussi un binaire encore ouvert
    cp -p "$f" ".$nom.retour" && mv -f ".$nom.retour" "$nom"
  done
  if [ -d "$PRECEDENTE/web" ]; then rm -rf web && cp -rp "$PRECEDENTE/web" web; fi
  rm -f "$MARQUE"
}

# Echec au demarrage : retour a la version precedente s'il s'agit d'une mise a jour.
echec() {
  arreter
  if [ -f "$MARQUE" ] && [ -d "$PRECEDENTE" ]; then
    retour_arriere "$1"
  else
    echo "$(horodatage) $1" >> serveur.log
  fi
  sleep 3
  exit 1          # kiosk-session relance aussitot (avec la version restauree le cas echeant)
}

# Passage depuis la version Python (1.0.x) : son installateur ne copie que serveur.py, lancer.sh,
# config.example.toml, VERSION et web/. Le binaire est donc pris dans l'archive qu'il a
# telechargee et verifiee (SHA-256), restee dans .maj/.
installer_binaire() {
  local archive=".maj/monitoring-$VERSION.tar.gz" dossier=.maj/extrait-binaire
  [ -f "$archive" ] || return
  rm -rf "$dossier" && mkdir -p "$dossier" \
    && tar -xzf "$archive" -C "$dossier" --no-same-owner "monitoring-$VERSION/monitoring" \
    && chmod 755 "$dossier/monitoring-$VERSION/monitoring" \
    && mv -f "$dossier/monitoring-$VERSION/monitoring" monitoring \
    && echo "$(horodatage) binaire $VERSION installe depuis $archive" >> serveur.log
  rm -rf "$dossier"
}
[ "$(./monitoring --version 2>/dev/null)" = "$VERSION" ] || installer_binaire

# garder les journaux courts
for f in serveur.log ecran.log; do
  [ -f "$f" ] && [ "$(wc -c < "$f")" -gt 2000000 ] && tail -c 500000 "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done

./monitoring serveur >> serveur.log 2>&1 &
SERVEUR=$!
pret=0
for _ in $(seq 1 60); do
  if ./monitoring sonde; then pret=1; break; fi
  kill -0 "$SERVEUR" 2>/dev/null || break
  sleep 0.5
done
[ "$pret" = 1 ] || echec "le serveur ne répond pas, voir serveur.log"

debut_journal=$(wc -c 2>/dev/null < ecran.log || echo 0)
./monitoring ecran >> ecran.log 2>&1 &
ECRAN=$!

if [ -f "$MARQUE" ]; then
  for _ in $(seq 1 "$OBSERVATION_ECRAN"); do
    kill -0 "$SERVEUR" 2>/dev/null || echec "le serveur s'est arrêté, voir serveur.log"
    if ! kill -0 "$ECRAN" 2>/dev/null; then
      echec "l'écran s'est arrêté : $(tail -c +$((debut_journal + 1)) ecran.log | grep -v '^[[:space:]]*$' | tail -n 1 | sed 's/^Ecran arrete : //')"
    fi
    sleep 1
  done
  echo "$(horodatage) version $(cat "$MARQUE") demarree" >> serveur.log
  rm -f "$MARQUE"
fi
# fichiers de l'ancienne version Python, inutiles ici (copies dans .maj/precedente si besoin)
rm -rf serveur.py web

wait "$ECRAN"
