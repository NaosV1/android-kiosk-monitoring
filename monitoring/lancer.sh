#!/bin/bash
# Lance le serveur de monitoring, puis l'interface en plein ecran (Chromium).
# Appele par /root/kiosk-app.sh ; relance automatique si l'un des deux s'arrete.
cd /root/monitoring || exit 1
export DISPLAY="${DISPLAY:-:0}"

PORT=$(python3 -c "import tomllib; print(tomllib.load(open('config.toml','rb')).get('general',{}).get('port',8080))" 2>/dev/null || echo 8080)

python3 -u serveur.py >> serveur.log 2>&1 &
SERVEUR=$!
trap 'kill $SERVEUR 2>/dev/null' EXIT

for _ in $(seq 1 60); do
  curl -s -o /dev/null "http://127.0.0.1:$PORT/" && break
  kill -0 $SERVEUR 2>/dev/null || { echo "serveur arrete, voir serveur.log"; sleep 5; exit 1; }
  sleep 0.5
done

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
