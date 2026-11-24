#!/bin/bash
# /root/kiosk-app.sh (Debian)
# === Appli affichee en plein ecran tactile sur la tablette ===
# Monitoring : /root/monitoring (config : /root/monitoring/config.toml). Apres modification : kiosk-restart
exec /root/monitoring/lancer.sh
