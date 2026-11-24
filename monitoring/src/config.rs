//! Lecture de `config.toml` (voir `config.example.toml` pour chaque option).
//!
//! Les regles (valeurs par defaut, bornes, identifiants des services et des nodes) sont celles
//! de l'ancienne version Python : les identifiants servent de cle dans l'historique `data.db`,
//! ils doivent rester identiques d'une version a l'autre.

use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use toml::{Table, Value};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug)]
pub struct ErreurConfig(String);

impl fmt::Display for ErreurConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErreurConfig {}

fn erreur<T>(message: impl Into<String>) -> Result<T, ErreurConfig> {
    Err(ErreurConfig(message.into()))
}

#[derive(Clone, Debug)]
pub struct Config {
    /// Secondes entre deux controles d'une node (minimum 5).
    pub intervalle: u64,
    /// Temps maximum d'une reponse, en secondes.
    pub delai: f64,
    /// Au-dela (ms), une node est consideree comme lente.
    pub seuil_lent: i64,
    /// Echecs d'affilee avant de declarer une node hors ligne (minimum 1).
    pub echecs_avant_panne: u32,
    /// Controles lents d'affilee avant de declarer une node lente (minimum 1) : un pic isole,
    /// souvent du a la tablette elle-meme, ne degrade pas le service.
    pub lenteurs_avant_alerte: u32,
    pub veille_minutes: i64,
    pub ecoute: String,
    pub port: u16,
    pub verifier_certificats: bool,
    pub tailscale: ConfigTailscale,
    pub maj: ConfigMaj,
    pub services: Vec<ConfigService>,
}

#[derive(Clone, Debug)]
pub struct ConfigTailscale {
    pub actif: bool,
    pub cle_api: String,
    pub tailnet: String,
    pub nom_tablette: String,
}

#[derive(Clone, Debug)]
pub struct ConfigMaj {
    /// `proprietaire/depot` sur GitHub, vide = mises a jour desactivees.
    pub depot: String,
    /// Periode de verification (minimum 1 h).
    pub heures: f64,
}

#[derive(Clone, Debug)]
pub struct ConfigService {
    pub id: String,
    pub nom: String,
    pub adresse: String,
    pub tag: String,
    pub nodes: Vec<ConfigNode>,
}

#[derive(Clone, Debug)]
pub struct ConfigNode {
    /// `<id du service>/<nom>`, cle des mesures dans l'historique.
    pub id: String,
    pub service: String,
    pub nom: String,
    pub lieu: String,
    /// Ce qui est controle, tel qu'affiche.
    pub cible: String,
    pub controle: Controle,
    /// Serveur DNS a interroger pour resoudre le nom (url, ping ou dns).
    pub resolveur: Option<String>,
    /// Le controle doit passer par le tunnel Tailscale.
    pub par_tailscale: bool,
    /// `None` : valeur de `[general] verifier_certificats`.
    pub verifier_certificat: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum Controle {
    Http(ControleHttp),
    Ping { hote: String },
    Dns { hote: String },
    Tcp { hote: String, port: u16 },
}

#[derive(Clone, Debug)]
pub struct ControleHttp {
    pub url: String,
    /// Nom d'hote de l'url, en minuscules ("" si illisible).
    pub hote: String,
    /// Code attendu ; sans lui, 2xx et 3xx sont acceptes.
    pub code: Option<u16>,
    /// Texte attendu dans la reponse.
    pub contient: Option<String>,
    /// Champs attendus dans la reponse JSON.
    pub json: Option<Table>,
    /// Serveur precis a interroger (comme `curl --resolve`).
    pub ip: Option<String>,
}

impl Controle {
    pub fn nom_type(&self) -> &'static str {
        match self {
            Controle::Http(_) => "http",
            Controle::Ping { .. } => "ping",
            Controle::Dns { .. } => "dns",
            Controle::Tcp { .. } => "tcp",
        }
    }
}

impl Config {
    pub fn lire(chemin: &Path) -> Result<Config, ErreurConfig> {
        let texte = std::fs::read_to_string(chemin).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ErreurConfig(format!(
                    "Fichier de configuration introuvable : {}",
                    chemin.display()
                ))
            } else {
                ErreurConfig(format!("Lecture de {} impossible : {e}", chemin.display()))
            }
        })?;
        Config::depuis_texte(&texte)
            .map_err(|e| ErreurConfig(format!("Erreur dans {} : {e}", chemin.display())))
    }

    pub fn depuis_texte(texte: &str) -> Result<Config, ErreurConfig> {
        let brut: Table = texte
            .parse()
            .map_err(|e: toml::de::Error| ErreurConfig(e.to_string()))?;
        let vide = Table::new();
        let g = section(&brut, "general")?.unwrap_or(&vide);
        let t = section(&brut, "tailscale")?.unwrap_or(&vide);
        let m = section(&brut, "mise_a_jour")?.unwrap_or(&vide);

        let depot = texte_option(m, "depot")?.unwrap_or_default();
        let depot = depot.trim().trim_matches('/').to_string();
        if !depot.is_empty() && !est_depot_github(&depot) {
            return erreur(format!(
                "mise_a_jour.depot doit etre de la forme proprietaire/depot ({depot:?})"
            ));
        }
        let tailnet = texte_option(t, "tailnet")?
            .unwrap_or_default()
            .trim()
            .to_string();

        Ok(Config {
            intervalle: entier(g, "intervalle", 30)?.max(5) as u64,
            delai: flottant(g, "delai", 10.0)?,
            seuil_lent: entier(g, "seuil_lent", 1000)?,
            echecs_avant_panne: entier(g, "echecs_avant_panne", 2)?.clamp(1, u32::MAX.into())
                as u32,
            lenteurs_avant_alerte: entier(g, "lenteurs_avant_alerte", 3)?.clamp(1, u32::MAX.into())
                as u32,
            veille_minutes: entier(g, "veille_minutes", 5)?,
            ecoute: texte_option(g, "ecoute")?.unwrap_or_else(|| "127.0.0.1".into()),
            port: u16::try_from(entier(g, "port", 8080)?)
                .or_else(|_| erreur("port doit etre entre 0 et 65535"))?,
            verifier_certificats: booleen(g, "verifier_certificats", true)?,
            tailscale: ConfigTailscale {
                actif: booleen(t, "actif", false)?,
                cle_api: texte_option(t, "cle_api")?
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                tailnet: if tailnet.is_empty() {
                    "-".into()
                } else {
                    tailnet
                },
                nom_tablette: texte_option(t, "nom_tablette")?
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            },
            maj: ConfigMaj {
                depot,
                heures: flottant(m, "verifier_toutes_les_heures", 6.0)?.max(1.0),
            },
            services: lire_services(&brut)?,
        })
    }

    /// Adresse a laquelle l'ecran et `monitoring sonde` joignent le serveur.
    pub fn url_locale(&self) -> String {
        let hote = match self.ecoute.as_str() {
            "" | "0.0.0.0" => "127.0.0.1".to_string(),
            "::" => "[::1]".to_string(),
            e if e.contains(':') => format!("[{e}]"),
            e => e.to_string(),
        };
        format!("http://{hote}:{}", self.port)
    }
}

fn lire_services(brut: &Table) -> Result<Vec<ConfigService>, ErreurConfig> {
    let liste = match brut.get("service") {
        None => return Ok(Vec::new()),
        Some(Value::Array(liste)) => liste,
        Some(_) => return erreur("[[service]] doit etre une liste de tables"),
    };
    let mut services = Vec::new();
    let mut vus = HashSet::new();
    for s in liste {
        let Value::Table(s) = s else {
            return erreur("chaque [[service]] doit etre une table");
        };
        let Some(nom) = texte_option(s, "nom")? else {
            return erreur("un [[service]] n'a pas de nom");
        };
        // "tailscale" est la cle des incidents Tailscale dans l'historique
        let mut id = slug(&nom);
        while vus.contains(&id) || id == "tailscale" {
            id.push_str("-2");
        }
        vus.insert(id.clone());

        let mut nodes = match s.get("nodes") {
            Some(Value::Array(liste)) => liste
                .iter()
                .enumerate()
                .map(|(i, n)| match n {
                    Value::Table(n) => lire_node(n, &format!("{id}-{}", i + 1)),
                    _ => erreur(format!(
                        "les nodes du service {nom:?} doivent etre des tables"
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return erreur(format!("nodes du service {nom:?} doit etre une liste")),
            None => vec![lire_node(s, &nom)?],
        };
        if nodes.is_empty() {
            return erreur(format!("le service {nom:?} n'a aucune node"));
        }
        let mut noms = HashSet::new();
        for n in &mut nodes {
            let base = n.nom.clone();
            let mut k = 2;
            while noms.contains(&n.nom) {
                n.nom = format!("{base}-{k}");
                k += 1;
            }
            noms.insert(n.nom.clone());
            n.id = format!("{id}/{}", n.nom);
            n.service = id.clone();
        }
        let adresse = match texte_option(s, "adresse")? {
            Some(a) if !a.is_empty() => a,
            _ => match &nodes[0].controle {
                Controle::Http(h) => h.hote.clone(),
                _ => nodes[0].cible.clone(),
            },
        };
        let tag = texte_option(s, "tag")?
            .unwrap_or_default()
            .trim()
            .to_string();
        services.push(ConfigService {
            id,
            nom,
            adresse,
            tag,
            nodes,
        });
    }
    Ok(services)
}

fn lire_node(brut: &Table, nom_defaut: &str) -> Result<ConfigNode, ErreurConfig> {
    let nom = texte_option(brut, "nom")?.unwrap_or_else(|| nom_defaut.to_string());
    let mut lieu = texte_option(brut, "lieu")?.unwrap_or_default();
    let (controle, cible) = if let Some(url) = texte_option(brut, "url")? {
        let json = match brut.get("json") {
            None => None,
            Some(Value::Table(t)) => Some(t.clone()).filter(|t| !t.is_empty()),
            Some(_) => {
                return erreur(format!(
                    "json doit etre une table, ex. json = {{ status = \"ok\" }} (node {nom:?})"
                ));
            }
        };
        let code = match entier_option(brut, "code")? {
            None | Some(0) => None,
            Some(c) => {
                Some(u16::try_from(c).or_else(|_| erreur(format!("code HTTP invalide ({c})")))?)
            }
        };
        let ip = texte_option(brut, "ip")?.filter(|ip| !ip.is_empty());
        if let Some(ip) = &ip
            && lieu.is_empty()
        {
            lieu = format!("via {ip}");
        }
        let controle = Controle::Http(ControleHttp {
            hote: hote_de_url(&url),
            code,
            contient: texte_option(brut, "contient")?.filter(|c| !c.is_empty()),
            json,
            ip,
            url: url.clone(),
        });
        (controle, url)
    } else if let Some(hote) = texte_option(brut, "ping")? {
        (Controle::Ping { hote: hote.clone() }, hote)
    } else if let Some(hote) = texte_option(brut, "dns")? {
        (Controle::Dns { hote: hote.clone() }, hote)
    } else if let Some(cible) = texte_option(brut, "tcp")? {
        let (hote, port) = cible.rsplit_once(':').unwrap_or(("", ""));
        let port = match port.parse::<u16>() {
            Ok(p) if !hote.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) => p,
            _ => return erreur(format!("tcp doit etre de la forme hote:port ({cible:?})")),
        };
        let hote = hote.trim_matches(|c| c == '[' || c == ']').to_string();
        (Controle::Tcp { hote, port }, cible)
    } else {
        return erreur(format!(
            "la node {nom:?} n'a ni url, ni tcp, ni ping, ni dns"
        ));
    };

    let resolveur = texte_option(brut, "resolveur")?.filter(|r| !r.is_empty());
    if resolveur.is_some() && matches!(controle, Controle::Tcp { .. }) {
        return erreur(format!(
            "resolveur n'est possible que pour url, ping ou dns (node {nom:?})"
        ));
    }
    let verifier_certificat = match brut.get("verifier_certificat") {
        None => None,
        Some(Value::Boolean(b)) => Some(*b),
        Some(_) => {
            return erreur(format!(
                "verifier_certificat doit valoir true ou false (node {nom:?})"
            ));
        }
    };
    Ok(ConfigNode {
        id: String::new(),
        service: String::new(),
        nom,
        lieu,
        cible,
        controle,
        resolveur,
        par_tailscale: booleen(brut, "par_tailscale", false)?,
        verifier_certificat,
    })
}

/// Identifiant d'un service a partir de son nom : `"Base de données"` -> `"base-de-donnees"`.
pub fn slug(texte: &str) -> String {
    let ascii: String = texte
        .nfkd()
        .filter(char::is_ascii)
        .collect::<String>()
        .to_ascii_lowercase();
    let mut s = String::new();
    let mut separateur = false;
    for c in ascii.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if separateur && !s.is_empty() {
                s.push('-');
            }
            separateur = false;
            s.push(c);
        } else {
            separateur = true;
        }
    }
    if s.is_empty() { "service".into() } else { s }
}

/// Nom d'hote d'une url, en minuscules et sans crochets (IPv6) ; "" si l'url est illisible.
pub fn hote_de_url(url: &str) -> String {
    url.parse::<ureq::http::Uri>()
        .ok()
        .and_then(|u| {
            u.host().map(|h| {
                h.trim_matches(|c| c == '[' || c == ']')
                    .to_ascii_lowercase()
            })
        })
        .unwrap_or_default()
}

fn est_depot_github(depot: &str) -> bool {
    let partie_valide = |p: &str| {
        !p.is_empty()
            && p.chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
    };
    matches!(depot.split_once('/'), Some((a, b)) if partie_valide(a) && partie_valide(b))
}

fn section<'a>(brut: &'a Table, nom: &str) -> Result<Option<&'a Table>, ErreurConfig> {
    match brut.get(nom) {
        None => Ok(None),
        Some(Value::Table(t)) => Ok(Some(t)),
        Some(_) => erreur(format!("[{nom}] doit etre une table")),
    }
}

/// Texte d'une valeur ; les nombres et booleens sont acceptes et convertis.
fn texte_option(table: &Table, cle: &str) -> Result<Option<String>, ErreurConfig> {
    Ok(match table.get(cle) {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Integer(i)) => Some(i.to_string()),
        Some(Value::Float(f)) => Some(f.to_string()),
        Some(Value::Boolean(b)) => Some(b.to_string()),
        Some(_) => return erreur(format!("{cle} doit etre un texte")),
    })
}

// Les conversions acceptent ce que la version Python acceptait (`int("30")`, `int(30.5)`,
// `bool(1)`) : une config qui marchait avant ne doit pas empecher le demarrage.

/// Entier ; un nombre a virgule est tronque, un texte numerique est accepte.
fn entier_option(table: &Table, cle: &str) -> Result<Option<i64>, ErreurConfig> {
    match table.get(cle) {
        None => Ok(None),
        Some(Value::Integer(i)) => Ok(Some(*i)),
        Some(Value::Float(f)) if f.is_finite() => Ok(Some(f.trunc() as i64)),
        Some(Value::String(s)) if s.trim().parse::<i64>().is_ok() => Ok(s.trim().parse().ok()),
        Some(_) => erreur(format!("{cle} doit etre un nombre entier")),
    }
}

fn entier(table: &Table, cle: &str, defaut: i64) -> Result<i64, ErreurConfig> {
    Ok(entier_option(table, cle)?.unwrap_or(defaut))
}

fn flottant(table: &Table, cle: &str, defaut: f64) -> Result<f64, ErreurConfig> {
    let valeur = match table.get(cle) {
        None => Some(defaut),
        Some(Value::Integer(i)) => Some(*i as f64),
        Some(Value::Float(f)) => Some(*f),
        Some(Value::String(s)) => s.trim().parse().ok(),
        Some(_) => None,
    };
    match valeur {
        Some(v) if v.is_finite() => Ok(v),
        _ => erreur(format!("{cle} doit etre un nombre")),
    }
}

fn booleen(table: &Table, cle: &str, defaut: bool) -> Result<bool, ErreurConfig> {
    match table.get(cle) {
        None => Ok(defaut),
        Some(Value::Boolean(b)) => Ok(*b),
        Some(Value::Integer(i)) => Ok(*i != 0),
        Some(_) => erreur(format!("{cle} doit valoir true ou false")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_slug_est_celui_de_la_version_python() {
        assert_eq!(slug("Base de données"), "base-de-donnees");
        assert_eq!(slug("API"), "api");
        assert_eq!(slug("  Été 2024 ! "), "ete-2024");
        assert_eq!(slug("1.1.1.1"), "1-1-1-1");
        assert_eq!(slug("Œuvre"), "uvre");
        assert_eq!(slug("x²"), "x2");
        assert_eq!(slug("!!!"), "service");
    }

    #[test]
    fn la_config_d_exemple_se_charge() {
        let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.toml");
        let conf = Config::lire(&chemin).unwrap();
        let ids: Vec<_> = conf.services.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "internet",
                "dns",
                "site",
                "api",
                "routeur",
                "intranet",
                "base-de-donnees"
            ]
        );
        let api = &conf.services[3];
        assert_eq!(api.adresse, "api.exemple.fr/health");
        assert_eq!(api.nodes[0].id, "api/srv1");
        assert_eq!(api.nodes[0].lieu, "via 203.0.113.10");
        let Controle::Http(h) = &api.nodes[1].controle else {
            panic!()
        };
        assert_eq!(
            (h.hote.as_str(), h.code, h.ip.as_deref()),
            ("api.exemple.fr", Some(200), Some("203.0.113.20"))
        );
        assert_eq!(
            h.json.as_ref().unwrap()["database"]["connected"],
            Value::Boolean(true)
        );
        assert_eq!(conf.services[1].adresse, "google.com");
        assert_eq!(conf.services[2].adresse, "www.exemple.fr");
        assert!(matches!(
            conf.services[6].nodes[0].controle,
            Controle::Tcp { port: 5432, .. }
        ));
        assert_eq!(conf.maj.depot, "NaosV1/android-kiosk-monitoring");
        assert_eq!(conf.url_locale(), "http://127.0.0.1:8080");
    }

    #[test]
    fn valeurs_par_defaut_et_bornes() {
        let conf = Config::depuis_texte(
            "[general]\nintervalle = 1\ndelai = 2\necoute = \"0.0.0.0\"\nport = \"9000\"",
        )
        .unwrap();
        assert_eq!(
            (conf.intervalle, conf.delai, conf.echecs_avant_panne),
            (5, 2.0, 2)
        );
        assert_eq!(
            (
                conf.seuil_lent,
                conf.veille_minutes,
                conf.verifier_certificats
            ),
            (1000, 5, true)
        );
        assert_eq!(conf.tailscale.tailnet, "-");
        assert_eq!((conf.maj.depot.as_str(), conf.maj.heures), ("", 6.0));
        assert_eq!(conf.url_locale(), "http://127.0.0.1:9000");
        assert!(conf.services.is_empty());
    }

    #[test]
    fn identifiants_uniques_pour_services_et_nodes() {
        let conf = Config::depuis_texte(
            r#"
            [[service]]
            nom = "Tailscale"
            ping = "1.1.1.1"
            [[service]]
            nom = "Web"
            nodes = [{ url = "https://a.fr" }, { nom = "x", tcp = "[::1]:22" }, { nom = "x", dns = "b.fr" }]
            [[service]]
            nom = "web"
            dns = "c.fr"
            "#,
        )
        .unwrap();
        let s = &conf.services;
        assert_eq!(
            (s[0].id.as_str(), s[1].id.as_str(), s[2].id.as_str()),
            ("tailscale-2", "web", "web-2")
        );
        let noms: Vec<_> = s[1].nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(noms, ["web/web-1", "web/x", "web/x-2"]);
        assert!(
            matches!(&s[1].nodes[1].controle, Controle::Tcp { hote, port: 22 } if hote == "::1")
        );
        assert_eq!(s[1].adresse, "a.fr");
    }

    #[test]
    fn erreurs_de_configuration() {
        let cas = [
            ("[[service]]\nnom = \"A\"\ntcp = \"hote\"", "tcp doit etre"),
            ("[[service]]\nnom = \"A\"\nlieu = \"x\"", "ni url"),
            (
                "[[service]]\nnom = \"A\"\ntcp = \"a:1\"\nresolveur = \"10.0.0.53\"",
                "resolveur",
            ),
            (
                "[[service]]\nnom = \"A\"\nurl = \"https://a\"\njson = \"ok\"",
                "json doit etre une table",
            ),
            ("[[service]]\nping = \"1.1.1.1\"", "pas de nom"),
            (
                "[mise_a_jour]\ndepot = \"pas un depot\"",
                "proprietaire/depot",
            ),
            ("[general]\nintervalle = \"trente\"", "intervalle"),
            (
                "[general]\nverifier_certificats = \"non\"",
                "verifier_certificats",
            ),
        ];
        for (texte, attendu) in cas {
            let e = Config::depuis_texte(texte).unwrap_err().to_string();
            assert!(e.contains(attendu), "{texte:?} -> {e}");
        }
    }
}
