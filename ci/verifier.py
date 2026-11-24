"""Verifications de la CI : compilation, config d'exemple, demarrage du serveur et API.

Lance par .github/workflows/ci.yml ; aussi utilisable en local :  python3 ci/verifier.py
"""
import json
import os
import py_compile
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
MON = RACINE / "monitoring"


def etape(texte):
    print(f"== {texte}", flush=True)


def lire(url, timeout=3):
    with urllib.request.urlopen(url, timeout=timeout) as rep:
        return rep.status, rep.read()


etape("compilation de serveur.py")
py_compile.compile(str(MON / "serveur.py"), doraise=True)

etape("lecture de config.example.toml")
os.environ["MONITORING_CONFIG"] = str(MON / "config.example.toml")
sys.path.insert(0, str(MON))
import serveur  # noqa: E402

conf = serveur.lire_config()
assert conf["services"], "aucun service dans config.example.toml"
print(f"   {len(conf['services'])} services, mises a jour : {conf['maj']['depot'] or 'desactivees'}")

etape("comparaison des versions")
assert serveur.cle_version("v1.0.12") == (1, 0, 12)
assert serveur.cle_version("dev") == ()
assert serveur.cle_version("1.0.10") > serveur.cle_version("1.0.9") > serveur.cle_version("dev")
assert serveur.cle_version("1.1.3") > serveur.cle_version("1.0.40")

etape("demarrage du serveur et API")
with tempfile.TemporaryDirectory() as tmp:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    cfg = Path(tmp) / "config.toml"
    cfg.write_text(f"""
[general]
port = {port}
intervalle = 5
[[service]]
nom = "Local"
tag = "Test"
tcp = "127.0.0.1:{port}"
""", encoding="utf-8")
    env = dict(os.environ, MONITORING_CONFIG=str(cfg), MONITORING_DB=str(Path(tmp) / "test.db"))
    journal = open(Path(tmp) / "serveur.log", "w+")
    proc = subprocess.Popen([sys.executable, str(MON / "serveur.py")], env=env, stdout=journal, stderr=subprocess.STDOUT)
    base = f"http://127.0.0.1:{port}"
    try:
        etat = None
        fin = time.time() + 30
        while time.time() < fin:
            assert proc.poll() is None, "le serveur s'est arrete"
            try:
                etat = json.loads(lire(base + "/api/etat")[1])
                if etat["services"] and etat["services"][0]["etat"] == "up":
                    break
            except OSError:
                pass
            time.sleep(0.5)
        assert etat, "l'API /api/etat ne repond pas"
        assert etat["services"][0]["etat"] == "up", f"le controle TCP local n'est pas passe : {etat['services'][0]}"
        assert etat["services"][0]["tag"] == "Test"
        assert etat["maj"]["actuelle"] in ("dev",) or serveur.cle_version(etat["maj"]["actuelle"])
        code, page = lire(base + "/")
        assert code == 200 and b"app.js" in page, "page d'accueil incorrecte"
        for fichier in ("/app.js", "/style.css"):
            assert lire(base + fichier)[0] == 200, f"{fichier} introuvable"
        assert lire(base + "/api/service/local")[0] == 200
        assert lire(base + "/api/incidents")[0] == 200
        print(f"   OK : {etat['services'][0]['nodes'][0]['info']}, version {etat['maj']['actuelle']}")
    except Exception:
        journal.seek(0)
        print("---- journal du serveur ----\n" + journal.read())
        raise
    finally:
        proc.terminate()
        proc.wait(timeout=10)

etape("tout est bon")
