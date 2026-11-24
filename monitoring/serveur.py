#!/usr/bin/env python3
"""Serveur de monitoring de la tablette.

- lit config.toml (services, nodes, Tailscale)
- controle chaque node (HTTP/HTTPS, TCP, ping ICMP ou resolution DNS) a intervalle regulier
- garde l'historique dans data.db (SQLite)
- interroge l'API Tailscale si une cle est configuree
- sert l'interface tactile (dossier web/) et une petite API JSON

Uniquement la bibliotheque standard de Python (3.11+).
"""
import errno
import http.client
import ipaddress
import itertools
import random
import json
import os
import re
import socket
import sqlite3
import ssl
import struct
import sys
import threading
import time
import tomllib
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ICI = Path(__file__).resolve().parent
CONFIG = Path(os.environ.get("MONITORING_CONFIG", ICI / "config.toml"))
BASE = Path(os.environ.get("MONITORING_DB", ICI / "data.db"))
WEB = ICI / "web"

JOUR = 86400
SEAU = 1200          # 20 min : largeur d'un point des courbes 24 h
NB_SEAUX = 72        # 72 x 20 min = 24 h


def log(*args):
    print(time.strftime("%Y-%m-%d %H:%M:%S"), *args, flush=True)


# --------------------------------------------------------------------------
# Configuration
# --------------------------------------------------------------------------

def slug(texte):
    sans_accents = unicodedata.normalize("NFKD", texte).encode("ascii", "ignore").decode()
    s = re.sub(r"[^a-z0-9]+", "-", sans_accents.lower()).strip("-")
    return s or "service"


def lire_node(brut, nom_defaut):
    node = {"nom": str(brut.get("nom", nom_defaut)), "lieu": str(brut.get("lieu", ""))}
    if "url" in brut:
        node["type"] = "http"
        node["url"] = str(brut["url"])
        node["hote_url"] = (urllib.parse.urlparse(node["url"]).hostname or "").lower()
        node["cible"] = node["url"]
        node["code"] = brut.get("code")
        node["contient"] = brut.get("contient")
        node["json"] = brut.get("json")
        if node["json"] is not None and not isinstance(node["json"], dict):
            raise ValueError(f"json doit etre une table, ex. json = {{ status = \"ok\" }} (node {node['nom']!r})")
        node["ip"] = str(brut["ip"]) if brut.get("ip") else None
        if node["ip"] and not node["lieu"]:
            node["lieu"] = f"via {node['ip']}"
    elif "ping" in brut:
        node["type"] = "ping"
        node["hote"] = node["cible"] = str(brut["ping"])
    elif "dns" in brut:
        node["type"] = "dns"
        node["hote"] = node["cible"] = str(brut["dns"])
    elif "tcp" in brut:
        hote, _, port = str(brut["tcp"]).rpartition(":")
        if not hote or not port.isdigit():
            raise ValueError(f"tcp doit etre de la forme hote:port ({brut['tcp']!r})")
        node["type"] = "tcp"
        node["hote"], node["port"] = hote.strip("[]"), int(port)
        node["cible"] = str(brut["tcp"])
    else:
        raise ValueError(f"la node {node['nom']!r} n'a ni url, ni tcp, ni ping, ni dns")
    # options communes
    node["resolveur"] = str(brut["resolveur"]) if brut.get("resolveur") else None
    node["par_tailscale"] = bool(brut.get("par_tailscale", False))
    node["verifier_certificat"] = brut.get("verifier_certificat")  # None = valeur de [general]
    if node["resolveur"] and node["type"] not in ("http", "ping", "dns"):
        raise ValueError(f"resolveur n'est possible que pour url, ping ou dns (node {node['nom']!r})")
    return node


def lire_config():
    with open(CONFIG, "rb") as f:
        brut = tomllib.load(f)
    g = brut.get("general", {})
    conf = {
        "intervalle": max(5, int(g.get("intervalle", 30))),
        "delai": float(g.get("delai", 10)),
        "seuil_lent": int(g.get("seuil_lent", 1000)),
        "echecs_avant_panne": max(1, int(g.get("echecs_avant_panne", 2))),
        "veille_minutes": int(g.get("veille_minutes", 5)),
        "ecoute": str(g.get("ecoute", "127.0.0.1")),
        "port": int(g.get("port", 8080)),
        "verifier_certificats": bool(g.get("verifier_certificats", True)),
    }
    t = brut.get("tailscale", {})
    conf["tailscale"] = {
        "actif": bool(t.get("actif", False)),
        "cle_api": str(t.get("cle_api", "")).strip(),
        "tailnet": str(t.get("tailnet", "-")).strip() or "-",
        "nom_tablette": str(t.get("nom_tablette", "")).strip(),
    }
    services, vus = [], set()
    for s in brut.get("service", []):
        nom = str(s["nom"])
        sid = slug(nom)
        while sid in vus or sid == "tailscale":
            sid += "-2"
        vus.add(sid)
        if "nodes" in s:
            nodes = [lire_node(n, f"{sid}-{i + 1}") for i, n in enumerate(s["nodes"])]
        else:
            nodes = [lire_node(s, nom)]
        noms = set()
        for n in nodes:
            base, k = n["nom"], 2
            while n["nom"] in noms:
                n["nom"] = f"{base}-{k}"
                k += 1
            noms.add(n["nom"])
            n["id"] = f"{sid}/{n['nom']}"
            n["service"] = sid
        adresse = s.get("adresse") or (urllib.parse.urlparse(nodes[0]["cible"]).hostname
                                        if nodes[0]["type"] == "http" else nodes[0]["cible"])
        services.append({"id": sid, "nom": nom, "adresse": str(adresse or ""), "nodes": nodes,
                         "tag": str(s.get("tag", "")).strip()})
    conf["services"] = services
    return conf


# --------------------------------------------------------------------------
# Controles
# --------------------------------------------------------------------------

def raison(exc, delai):
    r = getattr(exc, "reason", exc)
    if isinstance(r, TimeoutError):
        return f"Délai dépassé après {delai:g} s".replace(".", ",")
    if isinstance(r, ConnectionRefusedError):
        return "Connexion refusée"
    if isinstance(r, socket.gaierror):
        return "Nom introuvable (DNS)"
    if isinstance(r, ssl.SSLCertVerificationError):
        return "Certificat TLS invalide"
    if isinstance(r, ssl.SSLError):
        return "Erreur TLS"
    if isinstance(r, ConnectionResetError):
        return "Connexion coupée"
    if isinstance(r, OSError) and r.errno in (errno.ENETUNREACH, errno.EHOSTUNREACH):
        return "Réseau injoignable"
    texte = str(r) or r.__class__.__name__
    return texte[:110]


class Echec(Exception):
    """Echec de controle avec un message deja redige pour l'ecran."""


def resoudre_par(nom, serveur, delai):
    """Adresses IPv4 de `nom` demandees directement au serveur DNS `serveur`
    (ex. le DNS interne d'une entreprise joint par Tailscale)."""
    try:
        ipaddress.IPv4Address(nom)
        return [nom]
    except ValueError:
        pass
    qid = random.randint(0, 0xFFFF)
    requete = struct.pack("!HHHHHH", qid, 0x0100, 1, 0, 0, 0)
    requete += b"".join(bytes([len(p)]) + p.encode("idna") for p in nom.rstrip(".").split("."))
    requete += b"\0" + struct.pack("!HH", 1, 1)
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        rep = None
        for _ in range(2):  # une relance si le paquet UDP se perd
            s.settimeout(max(0.5, min(3.0, delai / 2)))
            try:
                s.sendto(requete, (serveur, 53))
                while True:
                    r = s.recv(4096)
                    if len(r) >= 12 and struct.unpack("!H", r[:2])[0] == qid:
                        rep = r
                        break
                break
            except TimeoutError:
                continue
    except OSError as e:
        raise Echec(f"DNS {serveur} injoignable ({raison(e, delai)})") from e
    finally:
        s.close()
    if rep is None:
        raise Echec(f"DNS {serveur} ne répond pas")
    rcode = rep[3] & 0x0F
    if rcode == 3:
        raise Echec(f"{nom} inconnu du DNS {serveur}")
    if rcode:
        raise Echec(f"DNS {serveur} : erreur {({2: 'SERVFAIL', 5: 'REFUSED'}).get(rcode, rcode)}")
    nb = struct.unpack("!H", rep[6:8])[0]
    i = 12
    while rep[i]:          # saute la question
        i += rep[i] + 1
    i += 5
    ips = []
    for _ in range(nb):
        if rep[i] & 0xC0 == 0xC0:
            i += 2
        else:
            while rep[i]:
                i += rep[i] + 1
            i += 1
        typ, _, _, lg = struct.unpack("!HHIH", rep[i:i + 10])
        i += 10
        if typ == 1 and lg == 4:
            ips.append(socket.inet_ntoa(rep[i:i + 4]))
        i += lg
    if not ips:
        raise Echec(f"{nom} : aucune adresse IPv4 (DNS {serveur})")
    return ips


def verifier_route(node, ip):
    """Pour une node `par_tailscale`, refuse de controler si la route ne passe pas par le tunnel."""
    if node.get("par_tailscale") and not passe_par_tailscale(ip):
        raise Echec(f"Pas de route Tailscale vers {ip}")


def chemin(node, ip):
    """Precision affichee apres le resultat : par ou est passe le controle."""
    if not (node.get("resolveur") or node.get("par_tailscale")):
        return ""
    voie = "par Tailscale" if passe_par_tailscale(ip) else "en direct"
    return f" · {voie}" if ip == node.get("hote") else f" · {ip} {voie}"


def ouvreur_ip(hote, ip, ctx):
    """Ouvreur urllib qui joint `hote` a l'adresse `ip` (comme curl --resolve) :
    le nom reste utilise pour Host, SNI et la verification du certificat."""
    def fabrique(classe):
        def creer(host, **kw):
            c = classe(host, **kw)
            if c.host.lower() == hote:
                c._create_connection = lambda adr, *a, **k: socket.create_connection((ip, adr[1]), *a, **k)
            return c
        return creer

    class Http(urllib.request.HTTPHandler):
        def http_open(self, req):
            return self.do_open(fabrique(http.client.HTTPConnection), req)

    class Https(urllib.request.HTTPSHandler):
        def https_open(self, req):
            return self.do_open(fabrique(http.client.HTTPSConnection), req, context=ctx)

    return urllib.request.build_opener(Http, Https(context=ctx))


def ecart_json(attendu, recu, chemin=""):
    """Premier champ de `attendu` absent ou different dans `recu` (None si tout correspond)."""
    for cle, val in attendu.items():
        p = f"{chemin}.{cle}" if chemin else str(cle)
        if not isinstance(recu, dict) or cle not in recu:
            return f"{p} absent de la réponse"
        v = recu[cle]
        if isinstance(val, dict):
            e = ecart_json(val, v, p)
            if e:
                return e
        elif v != val or isinstance(v, bool) != isinstance(val, bool):
            return f"{p} = {json.dumps(v, ensure_ascii=False)[:40]} (attendu {json.dumps(val, ensure_ascii=False)})"
    return None


def controler_http(node, conf):
    ctx = ssl.create_default_context()
    verifier = node.get("verifier_certificat")
    if not (conf["verifier_certificats"] if verifier is None else verifier):
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
    req = urllib.request.Request(node["url"], headers={"User-Agent": "tablette-monitoring/1.0"})
    debut = time.perf_counter()
    corps = b""
    ip = node.get("ip")
    try:
        if node.get("resolveur"):
            ip = resoudre_par(node["hote_url"], node["resolveur"], conf["delai"])[0]
        if node.get("par_tailscale"):
            verifier_route(node, ip or socket.getaddrinfo(node["hote_url"], None, socket.AF_INET)[0][4][0])
        if ip:
            rep = ouvreur_ip(node["hote_url"], ip, ctx).open(req, timeout=conf["delai"])
        else:
            rep = urllib.request.urlopen(req, timeout=conf["delai"], context=ctx)
        with rep:
            corps = rep.read(262144)
            code = rep.status
    except urllib.error.HTTPError as e:
        code = e.code
    except Echec as e:
        return False, None, str(e)
    except Exception as e:  # noqa: BLE001 - toute erreur reseau = node en echec
        return False, None, raison(e, conf["delai"]) + (f" ({ip})" if node.get("resolveur") and ip else "")
    voie = chemin(node, ip) if ip else ""
    ms = (time.perf_counter() - debut) * 1000
    if node.get("code"):
        ok = code == int(node["code"])
    else:
        ok = 200 <= code < 400
    if not ok:
        return False, ms, (f"HTTP {code} (attendu {node['code']})" if node.get("code") else f"Erreur HTTP {code}") + voie
    if node.get("contient") and str(node["contient"]).encode() not in corps:
        return False, ms, "Réponse inattendue (texte absent)" + voie
    if node.get("json"):
        try:
            donnees = json.loads(corps)
        except ValueError:
            return False, ms, f"HTTP {code} mais la réponse n'est pas du JSON" + voie
        ecart = ecart_json(node["json"], donnees)
        if ecart:
            return False, ms, ecart
        return True, ms, f"HTTP {code} · JSON conforme" + voie
    return True, ms, f"HTTP {code}" + voie


NUMEROS_PING = itertools.count(1)


def controler_ping(node, conf):
    """Ping ICMP (IPv4) sans droits root : socket ICMP datagramme du noyau.
    Un echo par seconde jusqu'a la premiere reponse ou la fin du delai."""
    delai = conf["delai"]
    try:
        if node.get("resolveur"):
            ip = resoudre_par(node["hote"], node["resolveur"], delai)[0]
        else:
            ip = socket.getaddrinfo(node["hote"], None, socket.AF_INET)[0][4][0]
        verifier_route(node, ip)
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_ICMP)
    except Echec as e:
        return False, None, str(e)
    except PermissionError:
        return False, None, "Ping non autorisé sur ce système"
    except Exception as e:  # noqa: BLE001
        return False, None, raison(e, delai)
    voie = chemin(node, ip)
    envois = {}
    fin = time.perf_counter() + delai
    prochain = 0.0
    try:
        while True:
            t = time.perf_counter()
            if t >= fin:
                return False, None, f"Aucune réponse au ping en {delai:g} s".replace(".", ",") + voie
            if t >= prochain:
                seq = next(NUMEROS_PING) & 0xFFFF
                s.sendto(struct.pack("!BBHHH", 8, 0, 0, 0, seq) + b"tablette-monitoring", (ip, 0))
                envois[seq] = t
                prochain = t + 1
            s.settimeout(max(0.01, min(prochain, fin) - time.perf_counter()))
            try:
                data = s.recv(1024)
            except TimeoutError:
                continue
            if len(data) >= 8 and data[0] == 0:
                seq_r = struct.unpack("!H", data[6:8])[0]
                if seq_r in envois:
                    ms = (time.perf_counter() - envois[seq_r]) * 1000
                    perdus = len(envois) - 1
                    return True, ms, ("Répond au ping" + (f" ({perdus} perdu{'s' if perdus > 1 else ''})" if perdus else "")
                                      + voie)
    except Exception as e:  # noqa: BLE001
        return False, None, raison(e, delai)
    finally:
        s.close()


def controler_dns(node, conf):
    """Resolution du nom par le resolveur de la tablette (/etc/resolv.conf),
    ou par le serveur DNS indique dans `resolveur`."""
    delai = conf["delai"]
    if node.get("resolveur"):
        debut = time.perf_counter()
        try:
            ips = resoudre_par(node["hote"], node["resolveur"], delai)
        except Echec as e:
            return False, None, str(e)
        ms = (time.perf_counter() - debut) * 1000
        return True, ms, f"Résolu en {ips[0]}" + (f" (+{len(ips) - 1})" if len(ips) > 1 else "")
    resultat = {}

    def resoudre():
        try:
            infos = socket.getaddrinfo(node["hote"], None, socket.AF_INET, socket.SOCK_STREAM)
            resultat["ips"] = sorted({i[4][0] for i in infos})
        except Exception as e:  # noqa: BLE001
            resultat["erreur"] = e

    debut = time.perf_counter()
    fil = threading.Thread(target=resoudre, daemon=True)
    fil.start()
    fil.join(delai)
    ms = (time.perf_counter() - debut) * 1000
    if fil.is_alive():
        return False, None, f"Délai dépassé après {delai:g} s".replace(".", ",")
    e = resultat.get("erreur")
    if isinstance(e, socket.gaierror):
        if e.errno in (socket.EAI_NONAME, getattr(socket, "EAI_NODATA", -5)):
            return False, None, "Nom introuvable (NXDOMAIN)"
        if e.errno == socket.EAI_AGAIN:
            return False, None, "Serveur DNS injoignable"
        return False, None, f"Échec de résolution ({e.strerror or e})"[:110]
    if e:
        return False, None, raison(e, delai)
    ips = resultat.get("ips") or []
    if not ips:
        return False, None, "Aucune adresse IPv4"
    return True, ms, f"Résolu en {ips[0]}" + (f" (+{len(ips) - 1})" if len(ips) > 1 else "")


def controler_tcp(node, conf):
    debut = time.perf_counter()
    try:
        with socket.create_connection((node["hote"], node["port"]), timeout=conf["delai"]):
            pass
    except Exception as e:  # noqa: BLE001
        return False, None, raison(e, conf["delai"])
    return True, (time.perf_counter() - debut) * 1000, f"TCP {node['port']} ouvert"


CONTROLES = {"http": controler_http, "tcp": controler_tcp, "ping": controler_ping, "dns": controler_dns}


def adresse_source(ip):
    """Adresse locale que le systeme utiliserait pour joindre `ip` (aucun paquet envoye)."""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        try:
            s.connect((ip, 9))
            return s.getsockname()[0]
        finally:
            s.close()
    except OSError:
        return None


def est_ip_tailscale(ip):
    try:
        return ipaddress.IPv4Address(ip) in ipaddress.IPv4Network("100.64.0.0/10")
    except ValueError:
        return False


def passe_par_tailscale(ip):
    """Vrai si le trafic vers `ip` part dans le tunnel Tailscale (route de sous-reseau ou exit node)."""
    return est_ip_tailscale(adresse_source(ip))


def tablette_sur_tailnet():
    """Adresse Tailscale de la tablette si le tunnel Tailscale (VPN Android) est actif."""
    ip = adresse_source("100.100.100.100")
    return ip if est_ip_tailscale(ip) else None


# --------------------------------------------------------------------------
# Moniteur
# --------------------------------------------------------------------------

class Moniteur:
    def __init__(self, conf):
        self.conf = conf
        self.verrou = threading.RLock()
        self.db = sqlite3.connect(BASE, check_same_thread=False, isolation_level=None)
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.executescript("""
            CREATE TABLE IF NOT EXISTS mesures (ts INTEGER NOT NULL, node TEXT NOT NULL,
                ok INTEGER NOT NULL, ms REAL);
            CREATE INDEX IF NOT EXISTS mesures_ts ON mesures(ts);
            CREATE TABLE IF NOT EXISTS incidents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service TEXT NOT NULL, nom TEXT NOT NULL,
                niveau TEXT NOT NULL, message TEXT NOT NULL,
                debut INTEGER NOT NULL, fin INTEGER,
                acquitte INTEGER NOT NULL DEFAULT 0,
                panne_s INTEGER NOT NULL DEFAULT 0, panne_depuis INTEGER);
            CREATE TABLE IF NOT EXISTS meta (cle TEXT PRIMARY KEY, valeur TEXT);
        """)
        maintenant = int(time.time())
        self.services = {}
        self.nodes = {}
        decalage = 0.0
        for s in conf["services"]:
            # debut du suivi propre a chaque service (un service ajoute plus tard ne
            # doit pas compter comme disponible avant d'avoir ete controle)
            cle = f"debut:{s['id']}"
            self.db.execute("INSERT OR IGNORE INTO meta VALUES (?, ?)", (cle, str(maintenant)))
            debut_s = int(self.db.execute("SELECT valeur FROM meta WHERE cle=?", (cle,)).fetchone()[0])
            etat_s = {"id": s["id"], "nom": s["nom"], "adresse": s["adresse"], "debut_suivi": debut_s, "tag": s["tag"],
                      "etat": "inconnu", "depuis": maintenant, "message": "", "nodes": []}
            for n in s["nodes"]:
                etat_n = dict(n)
                etat_n.update({"etat": "inconnu", "ms": None, "info": "En attente du premier contrôle",
                               "depuis": maintenant, "dernier": None, "echecs": 0,
                               "prochain": time.time() + decalage, "en_cours": False})
                decalage += 0.25
                self.nodes[n["id"]] = etat_n
                etat_s["nodes"].append(n["id"])
            self.services[s["id"]] = etat_s

        # incidents restes ouverts lors d'un arret precedent
        self.incident_ouvert = {}
        for iid, service in self.db.execute("SELECT id, service FROM incidents WHERE fin IS NULL"):
            if service in self.services or service in ("tailscale", "exit-node"):
                self.incident_ouvert[service] = iid
            else:  # service retire de la config : on ferme
                self.fermer_incident(iid, maintenant)

        self.tailscale = {"tablette_ip": None, "tablette_depuis": maintenant, "tablette_connue": False,
                          "exit_node": False, "exit_depuis": maintenant,
                          "api_erreur": None, "api_maj": None, "appareils": None, "tailnet": None}
        self.cache_series = {"ts": 0}
        self.dernier_menage = 0

    # ---------------------------------------------------------------- incidents
    def fermer_incident(self, iid, maintenant):
        ligne = self.db.execute("SELECT panne_s, panne_depuis FROM incidents WHERE id=?", (iid,)).fetchone()
        panne_s = ligne[0] + ((maintenant - ligne[1]) if ligne[1] else 0)
        self.db.execute("UPDATE incidents SET fin=?, panne_s=?, panne_depuis=NULL WHERE id=?",
                        (maintenant, panne_s, iid))

    def maj_incident(self, cle, nom, etat, message, maintenant):
        iid = self.incident_ouvert.get(cle)
        if etat in ("warn", "down"):
            if iid is None:
                cur = self.db.execute(
                    "INSERT INTO incidents (service, nom, niveau, message, debut, panne_depuis) "
                    "VALUES (?, ?, ?, ?, ?, ?)",
                    (cle, nom, etat, message, maintenant, maintenant if etat == "down" else None))
                self.incident_ouvert[cle] = cur.lastrowid
                log(f"incident ouvert : {nom} ({etat}) {message}")
                return
            niveau, panne_s, panne_depuis = self.db.execute(
                "SELECT niveau, panne_s, panne_depuis FROM incidents WHERE id=?", (iid,)).fetchone()
            if etat == "down":
                niveau = "down"
                panne_depuis = panne_depuis or maintenant
            elif panne_depuis:
                panne_s += maintenant - panne_depuis
                panne_depuis = None
            self.db.execute("UPDATE incidents SET niveau=?, message=?, panne_s=?, panne_depuis=? WHERE id=?",
                            (niveau, message, panne_s, panne_depuis, iid))
        elif etat == "up" and iid is not None:
            self.fermer_incident(iid, maintenant)
            del self.incident_ouvert[cle]
            log(f"incident clos : {nom}")

    # ---------------------------------------------------------------- etats
    def evaluer_service(self, sid, maintenant):
        s = self.services[sid]
        nodes = [self.nodes[n] for n in s["nodes"]]
        etats = [n["etat"] for n in nodes]
        pannes = [n for n in nodes if n["etat"] == "down"]
        lentes = [n for n in nodes if n["etat"] == "lent"]
        if all(e == "inconnu" for e in etats):
            etat = "inconnu"
        elif all(e == "down" for e in etats):
            etat = "down"
        elif pannes or lentes:
            etat = "warn"
        else:
            etat = "up"

        if etat == "down":
            message = (f"Ne répond plus : {pannes[0]['info']}" if len(nodes) == 1
                       else f"Aucune node ne répond ({pannes[0]['info']})")
        elif etat == "warn":
            morceaux = []
            if pannes:
                noms = ", ".join(n["nom"] for n in pannes)
                verbe = "ne répondent plus" if len(pannes) > 1 else "ne répond plus"
                morceaux.append(f"{noms} {verbe} ({pannes[0]['info']})" if len(nodes) > 1
                                else f"Ne répond plus ({pannes[0]['info']})")
            if lentes:
                if len(nodes) == 1:
                    morceaux.append(f"Lent : {round(lentes[0]['ms'])} ms")
                else:
                    noms = ", ".join(f"{n['nom']} ({round(n['ms'])} ms)" for n in lentes)
                    morceaux.append(f"lent : {noms}")
            message = " · ".join(morceaux)
        else:
            message = ""

        if etat != s["etat"] or message != s["message"]:
            if etat != s["etat"]:
                s["depuis"] = maintenant
            s["etat"], s["message"] = etat, message
            if etat != "inconnu":
                self.maj_incident(sid, s["nom"], etat, message, maintenant)

    def controler(self, node):
        try:
            ok, ms, info = CONTROLES[node["type"]](node, self.conf)
        except Exception as e:  # noqa: BLE001
            ok, ms, info = False, None, f"Erreur interne : {e}"[:110]
        maintenant = int(time.time())
        with self.verrou:
            node["dernier"] = maintenant
            node["ms"] = round(ms, 1) if ms is not None else None
            node["info"] = info
            if ok:
                node["echecs"] = 0
                nouvel = "lent" if ms > self.conf["seuil_lent"] else "up"
            else:
                node["echecs"] += 1
                nouvel = "down" if node["echecs"] >= self.conf["echecs_avant_panne"] else node["etat"]
            if nouvel != node["etat"]:
                node["etat"], node["depuis"] = nouvel, maintenant
            self.db.execute("INSERT INTO mesures VALUES (?, ?, ?, ?)",
                            (maintenant, node["id"], 1 if ok else 0, ms if ok else None))
            self.evaluer_service(node["service"], maintenant)
            intervalle = self.conf["intervalle"]
            node["prochain"] = time.time() + (intervalle if ok else min(10, intervalle))
            node["en_cours"] = False

    def relancer(self, sid=None):
        with self.verrou:
            for n in self.nodes.values():
                if sid is None or n["service"] == sid:
                    n["prochain"] = 0

    def boucle_controles(self):
        with ThreadPoolExecutor(max_workers=8) as pool:
            while True:
                maintenant = time.time()
                with self.verrou:
                    dus = [n for n in self.nodes.values() if n["prochain"] <= maintenant and not n["en_cours"]]
                    for n in dus:
                        n["en_cours"] = True
                for n in dus:
                    pool.submit(self.controler, n)
                if maintenant - self.dernier_menage > 3600:
                    self.dernier_menage = maintenant
                    with self.verrou:
                        self.db.execute("DELETE FROM mesures WHERE ts < ?", (int(maintenant) - JOUR - 7200,))
                        self.db.execute("DELETE FROM incidents WHERE fin IS NOT NULL AND fin < ?",
                                        (int(maintenant) - 120 * JOUR,))
                time.sleep(0.5)

    # ---------------------------------------------------------------- Tailscale
    def interroger_api_tailscale(self):
        t = self.conf["tailscale"]
        url = f"https://api.tailscale.com/api/v2/tailnet/{urllib.parse.quote(t['tailnet'])}/devices?fields=all"
        req = urllib.request.Request(url, headers={"Authorization": f"Bearer {t['cle_api']}",
                                                   "User-Agent": "tablette-monitoring/1.0"})
        try:
            with urllib.request.urlopen(req, timeout=15) as rep:
                donnees = json.load(rep)
        except urllib.error.HTTPError as e:
            return None, ("Clé API refusée par Tailscale" if e.code in (401, 403)
                          else f"API Tailscale : erreur HTTP {e.code}")
        except Exception as e:  # noqa: BLE001
            return None, f"API Tailscale injoignable ({raison(e, 15)})"
        appareils, tailnet = [], None
        for d in donnees.get("devices", []):
            ips = [a for a in d.get("addresses", []) if "." in a]
            vu = None
            if d.get("lastSeen"):
                try:
                    vu = int(datetime.fromisoformat(d["lastSeen"].replace("Z", "+00:00")).timestamp())
                except ValueError:
                    vu = None
            en_ligne = d.get("connectedToControl")
            if en_ligne is None:
                en_ligne = vu is not None and time.time() - vu < 300
            derp = None
            for region, info in ((d.get("clientConnectivity") or {}).get("latency") or {}).items():
                if info.get("preferred"):
                    derp = f"{region} · {round(info.get('latencyMs', 0))} ms"
            routes = d.get("enabledRoutes") or []
            fqdn = d.get("name", "")
            if not tailnet and "." in fqdn:
                tailnet = fqdn.split(".", 1)[1]
            appareils.append({
                "nom": d.get("hostname") or fqdn.split(".")[0],
                "os": d.get("os", ""),
                "ip": ips[0] if ips else "",
                "en_ligne": bool(en_ligne),
                "vu": vu,
                "derp": derp,
                "sortie": "0.0.0.0/0" in routes,
                "sous_reseaux": [r for r in routes if r not in ("0.0.0.0/0", "::/0")],
                "maj_dispo": bool(d.get("updateAvailable")),
            })
        return {"appareils": appareils, "tailnet": tailnet}, None

    def boucle_tailscale(self):
        dernier_api = 0
        while True:
            maintenant = int(time.time())
            ip = tablette_sur_tailnet()
            # exit node utilise par la tablette = Internet (ici 1.1.1.1) route dans le tunnel
            sortie = bool(ip) and passe_par_tailscale("1.1.1.1")
            t = self.conf["tailscale"]
            with self.verrou:
                ts = self.tailscale
                if ip != ts["tablette_ip"] or not ts["tablette_connue"]:
                    if bool(ip) != bool(ts["tablette_ip"]) or not ts["tablette_connue"]:
                        ts["tablette_depuis"] = maintenant
                    ts["tablette_ip"], ts["tablette_connue"] = ip, True
                if sortie != ts["exit_node"]:
                    ts["exit_node"], ts["exit_depuis"] = sortie, maintenant
                    log("exit node " + ("ACTIVE sur la tablette" if sortie else "desactive"))
                if t["actif"]:
                    self.maj_incident("tailscale", "Tailscale", "up" if ip else "down",
                                      "La tablette n'est plus sur le tailnet", maintenant)
                    self.maj_incident("exit-node", "Exit node", "warn" if sortie else "up",
                                      "Exit node activé sur la tablette : les contrôles Internet "
                                      "passent par l'entreprise", maintenant)
            if t["cle_api"] and maintenant - dernier_api >= 60:
                dernier_api = maintenant
                resultat, erreur = self.interroger_api_tailscale()
                with self.verrou:
                    self.tailscale["api_erreur"] = erreur
                    if resultat:
                        self.tailscale.update(resultat)
                        self.tailscale["api_maj"] = maintenant
            time.sleep(10)

    # ---------------------------------------------------------------- statistiques
    def series(self):
        """Courbes 24 h par node (moyenne par tranche de 20 min), recalculees au plus 1 fois par minute."""
        maintenant = int(time.time())
        if maintenant - self.cache_series["ts"] < 60:
            return self.cache_series
        debut = (maintenant // SEAU) * SEAU - (NB_SEAUX - 1) * SEAU
        par_node = {nid: [None] * NB_SEAUX for nid in self.nodes}
        with self.verrou:
            lignes = self.db.execute(
                "SELECT node, (ts - ?) / ?, AVG(ms), SUM(ok), COUNT(*) FROM mesures "
                "WHERE ts >= ? GROUP BY node, (ts - ?) / ?", (debut, SEAU, debut, debut, SEAU)).fetchall()
        for nid, k, moy, oks, total in lignes:
            if nid in par_node and 0 <= k < NB_SEAUX:
                par_node[nid][k] = round(moy, 1) if oks else None
        par_service = {}
        for sid, s in self.services.items():
            pts = []
            for k in range(NB_SEAUX):
                vals = [par_node[n][k] for n in s["nodes"] if par_node[n][k] is not None]
                pts.append(round(sum(vals) / len(vals), 1) if vals else None)
            par_service[sid] = pts
        self.cache_series = {"ts": maintenant, "debut": debut, "nodes": par_node, "services": par_service}
        return self.cache_series

    def incidents_entre(self, debut, fin, service=None):
        req = "SELECT id, service, nom, niveau, message, debut, fin, acquitte, panne_s, panne_depuis " \
              "FROM incidents WHERE debut < ? AND (fin IS NULL OR fin > ?)"
        args = [fin, debut]
        if service:
            req += " AND service = ?"
            args.append(service)
        with self.verrou:
            lignes = self.db.execute(req + " ORDER BY debut DESC", args).fetchall()
        cles = ("id", "service", "nom", "niveau", "message", "debut", "fin", "acquitte", "panne_s", "panne_depuis")
        return [dict(zip(cles, l)) for l in lignes]

    @staticmethod
    def secondes_panne(inc, maintenant):
        return inc["panne_s"] + ((maintenant - inc["panne_depuis"]) if inc["panne_depuis"] else 0)

    def dispo(self, sid, jours, maintenant):
        debut = max(self.services[sid]["debut_suivi"], maintenant - jours * JOUR)
        observe = maintenant - debut
        if observe < 60:
            return None
        panne = sum(self.secondes_panne(i, maintenant) for i in self.incidents_entre(debut, maintenant, sid))
        return round(max(0.0, 100 * (1 - panne / observe)), 2)

    # ---------------------------------------------------------------- JSON
    def etat_json(self):
        maintenant = int(time.time())
        series = self.series()
        with self.verrou:
            services = []
            for sid, s in self.services.items():
                nodes = []
                for nid in s["nodes"]:
                    n = self.nodes[nid]
                    nodes.append({k: n[k] for k in ("id", "nom", "lieu", "cible", "type", "etat", "ms",
                                                    "info", "depuis", "dernier")})
                ms_ok = [n["ms"] for n in nodes if n["etat"] in ("up", "lent") and n["ms"] is not None]
                services.append({
                    "id": sid, "nom": s["nom"], "adresse": s["adresse"], "tag": s["tag"], "etat": s["etat"],
                    "depuis": s["depuis"], "message": s["message"],
                    "ms": round(sum(ms_ok) / len(ms_ok)) if ms_ok else None,
                    "spark": series["services"].get(sid, []),
                    "nodes": nodes,
                })
            ts = self.tailscale
            t = self.conf["tailscale"]
            appareils = ts["appareils"]
            if appareils is not None:
                for a in appareils:
                    a["cette_tablette"] = bool((ts["tablette_ip"] and a["ip"] == ts["tablette_ip"]) or
                                               (t["nom_tablette"] and a["nom"] == t["nom_tablette"]))
            ouverts = [{"id": iid, "service": cle} for cle, iid in self.incident_ouvert.items()]
        for s in services:
            s["dispo30"] = self.dispo(s["id"], 30, maintenant)
        incidents_ouverts = self.incidents_entre(maintenant - 400 * JOUR, maintenant + 1)
        incidents_ouverts = [i for i in incidents_ouverts if i["fin"] is None]
        return {
            "maintenant": maintenant,
            "intervalle": self.conf["intervalle"],
            "seuil_lent": self.conf["seuil_lent"],
            "veille_minutes": self.conf["veille_minutes"],
            "services": services,
            "tailscale": {
                "actif": t["actif"],
                "tablette": {"ip": ts["tablette_ip"], "depuis": ts["tablette_depuis"]},
                "exit_node": {"actif": ts["exit_node"], "depuis": ts["exit_depuis"]},
                "api": {"configuree": bool(t["cle_api"]), "erreur": ts["api_erreur"], "maj": ts["api_maj"],
                        "tailnet": ts["tailnet"], "appareils": appareils},
            },
            "incidents_ouverts": incidents_ouverts,
            "_ouverts": ouverts,
        }

    def service_json(self, sid):
        if sid not in self.services:
            return None
        maintenant = int(time.time())
        series = self.series()
        jours = []
        aujourdhui = datetime.now().replace(hour=0, minute=0, second=0, microsecond=0)
        debut_90 = int(aujourdhui.timestamp()) - 89 * JOUR
        incidents = self.incidents_entre(debut_90, maintenant + 1, sid)
        for k in range(90):
            d0 = debut_90 + k * JOUR
            d1 = d0 + JOUR
            if d1 <= self.services[sid]["debut_suivi"]:
                jours.append("vide")
                continue
            niveaux = [i["niveau"] for i in incidents if i["debut"] < d1 and (i["fin"] is None or i["fin"] > d0)]
            jours.append("down" if "down" in niveaux else "warn" if niveaux else "up")
        return {
            "id": sid,
            "debut_courbe": series.get("debut"),
            "seau": SEAU,
            "courbes": {nid: series["nodes"].get(nid, []) for nid in self.services[sid]["nodes"]},
            "jours": jours,
            "dispo90": self.dispo(sid, 90, maintenant),
            "jours_avec_incident": sum(1 for j in jours if j in ("warn", "down")),
            "evenements": incidents[:12],
        }

    def incidents_json(self):
        maintenant = int(time.time())
        liste = self.incidents_entre(maintenant - 30 * JOUR, maintenant + 1)
        par_service = {}
        total = 0
        for i in liste:
            duree = (i["fin"] or maintenant) - max(i["debut"], maintenant - 30 * JOUR)
            total += duree
            cle = (i["service"], i["nom"])
            par_service[cle] = par_service.get(cle, 0) + duree
        dispos = [d for d in (self.dispo(sid, 30, maintenant) for sid in self.services) if d is not None]
        return {
            "maintenant": maintenant,
            "incidents": liste,
            "bilan": {
                "nombre": len(liste),
                "duree": total,
                "dispo": round(sum(dispos) / len(dispos), 2) if dispos else None,
                "par_service": sorted(({"service": k[0], "nom": k[1], "duree": v, "en_cours": any(
                    i["fin"] is None and i["service"] == k[0] for i in liste)} for k, v in par_service.items()),
                    key=lambda x: -x["duree"])[:6],
            },
        }

    def acquitter(self, iid):
        with self.verrou:
            self.db.execute("UPDATE incidents SET acquitte=1 WHERE id=?", (iid,))


# --------------------------------------------------------------------------
# Serveur HTTP
# --------------------------------------------------------------------------

TYPES = {".html": "text/html; charset=utf-8", ".css": "text/css; charset=utf-8",
         ".js": "text/javascript; charset=utf-8", ".svg": "image/svg+xml", ".png": "image/png",
         ".json": "application/json", ".woff2": "font/woff2"}


def creer_gestionnaire(moniteur):
    class Gestionnaire(BaseHTTPRequestHandler):
        server_version = "tablette-monitoring"

        def log_message(self, *args):
            pass

        def envoyer(self, code, corps, type_):
            self.send_response(code)
            self.send_header("Content-Type", type_)
            self.send_header("Content-Length", str(len(corps)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(corps)

        def json(self, objet, code=200):
            self.envoyer(code, json.dumps(objet, ensure_ascii=False).encode(), "application/json")

        def do_GET(self):
            chemin = urllib.parse.urlparse(self.path).path
            try:
                if chemin == "/api/etat":
                    return self.json(moniteur.etat_json())
                if chemin.startswith("/api/service/"):
                    rep = moniteur.service_json(urllib.parse.unquote(chemin[len("/api/service/"):]))
                    return self.json(rep) if rep else self.json({"erreur": "service inconnu"}, 404)
                if chemin == "/api/incidents":
                    return self.json(moniteur.incidents_json())
            except Exception as e:  # noqa: BLE001
                log("erreur API", chemin, repr(e))
                return self.json({"erreur": str(e)}, 500)
            if chemin == "/":
                chemin = "/index.html"
            fichier = (WEB / chemin.lstrip("/")).resolve()
            if WEB.resolve() not in fichier.parents or not fichier.is_file():
                return self.envoyer(404, b"introuvable", "text/plain; charset=utf-8")
            self.envoyer(200, fichier.read_bytes(), TYPES.get(fichier.suffix, "application/octet-stream"))

        def do_POST(self):
            chemin = urllib.parse.urlparse(self.path).path
            m = re.fullmatch(r"/api/incidents/(\d+)/acquitter", chemin)
            if m:
                moniteur.acquitter(int(m.group(1)))
                return self.json({"ok": True})
            m = re.fullmatch(r"/api/controler(?:/([^/]+))?", chemin)
            if m:
                moniteur.relancer(urllib.parse.unquote(m.group(1)) if m.group(1) else None)
                return self.json({"ok": True})
            self.json({"erreur": "inconnu"}, 404)

    return Gestionnaire


def main():
    try:
        conf = lire_config()
    except FileNotFoundError:
        sys.exit(f"Fichier de configuration introuvable : {CONFIG}")
    except (tomllib.TOMLDecodeError, KeyError, ValueError) as e:
        sys.exit(f"Erreur dans {CONFIG} : {e}")
    moniteur = Moniteur(conf)
    log(f"{len(conf['services'])} services, {len(moniteur.nodes)} nodes, "
        f"Tailscale actif={conf['tailscale']['actif']} api={'oui' if conf['tailscale']['cle_api'] else 'non'}")
    threading.Thread(target=moniteur.boucle_controles, daemon=True).start()
    threading.Thread(target=moniteur.boucle_tailscale, daemon=True).start()
    serveur = ThreadingHTTPServer((conf["ecoute"], conf["port"]), creer_gestionnaire(moniteur))
    serveur.daemon_threads = True
    log(f"interface sur http://{conf['ecoute']}:{conf['port']}/")
    serveur.serve_forever()


if __name__ == "__main__":
    main()
