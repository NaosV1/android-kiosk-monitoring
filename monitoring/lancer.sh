#!/bin/bash
# Lance le serveur de monitoring, puis l'interface en plein ecran (Chromium).
# Appele par /root/kiosk-app.sh ; relance automatique si l'un des deux s'arrete.
# Apres une mise a jour (.maj/a-confirmer present) : si le nouveau serveur ne demarre
# pas, on remet la version precedente (.maj/precedente) et on relance.
cd "$(dirname "$(readlink -f "$0")")" || exit 1
export DISPLAY="${DISPLAY:-:0}"

MARQUE=.maj/a-confirmer
PRECEDENTE=.maj/precedente

retour_arriere() {
  local version
  version=$(cat "$MARQUE" 2>/dev/null)
  echo "$(date '+%F %T') la version $version n'a pas demarre : retour a la version precedente" \
    | tee -a serveur.log > .maj/retour-arriere
  for f in serveur.py lancer.sh config.example.toml VERSION; do
    [ -f "$PRECEDENTE/$f" ] && cp -p "$PRECEDENTE/$f" "$f"
  done
  if [ -d "$PRECEDENTE/web" ]; then rm -rf web && cp -rp "$PRECEDENTE/web" web; fi
  rm -f "$MARQUE"
}

PORT=$(python3 -c "import tomllib; print(tomllib.load(open('config.toml','rb')).get('general',{}).get('port',8080))" 2>/dev/null || echo 8080)

python3 -u serveur.py >> serveur.log 2>&1 &
SERVEUR=$!
trap 'kill $SERVEUR 2>/dev/null' EXIT

pret=0
for _ in $(seq 1 60); do
  if curl -s -o /dev/null "http://127.0.0.1:$PORT/"; then pret=1; break; fi
  kill -0 $SERVEUR 2>/dev/null || break
  sleep 0.5
done

if [ "$pret" = 0 ]; then
  if [ -f "$MARQUE" ] && [ -d "$PRECEDENTE" ]; then
    kill $SERVEUR 2>/dev/null
    retour_arriere
  else
    echo "serveur arrete, voir serveur.log"
  fi
  sleep 3
  exit 1          # kiosk-session relance aussitot (avec la version restauree le cas echeant)
fi
if [ -f "$MARQUE" ]; then
  echo "$(date '+%F %T') version $(cat "$MARQUE") demarree" >> serveur.log
  rm -f "$MARQUE"
fi

# garder les journaux courts
for f in serveur.log chromium.log; do
  [ -f "$f" ] && [ "$(stat -c %s "$f")" -gt 2000000 ] && tail -c 500000 "$f" > "$f.tmp" && mv "$f.tmp" "$f"
done

chromium \
  --no-sandbox --incognito --kiosk --app="http://127.0.0.1:$PORT/" --lang=fr \
  --force-device-scale-factor=1.5 \
  --user-data-dir=/root/.config/chromium-kiosk \
  --disable-gpu --disable-dev-shm-usage \
  --touch-events=enabled --disable-pinch --overscroll-history-navigation=0 \
  --no-first-run --no-default-browser-check --noerrdialogs --disable-infobars \
  --disable-session-crashed-bubble --disable-features=Translate,TranslateUI,MediaRouter \
  --disable-component-update --check-for-update-interval=31536000 --password-store=basic \
  >> chromium.log 2>&1
