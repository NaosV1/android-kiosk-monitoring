//! Etat des services et des nodes : boucle des controles, evaluation et incidents.
//! Les rapports JSON lus par l'ecran sont dans [`rapports`].

mod base;
mod rapports;

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::ops::Range;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base::Base;

use crate::api::{Appareil, Niveau};
use crate::config::{Config, ConfigNode};
use crate::controles::{self, Resultat};
use crate::https::tronquer;
use crate::message_de_panique;

/// Controles simultanes au maximum.
const CONTROLEURS: usize = 8;

pub struct Moniteur {
    conf: Config,
    /// Index plat de chaque node -> (index du service, index de la node) dans `conf.services`.
    nodes: Vec<(usize, usize)>,
    /// Nodes de chaque service dans l'index plat.
    plages: Vec<Range<usize>>,
    etat: Mutex<Etat>,
}

struct Etat {
    base: Base,
    services: Vec<EtatService>,
    nodes: Vec<EtatNode>,
    /// Incident en cours par cle (id du service, `tailscale` ou `exit-node`).
    incidents_ouverts: HashMap<String, i64>,
    tailscale: EtatTailscale,
    /// Courbes 24 h, recalculees au plus une fois par minute.
    series: Option<rapports::Series>,
}

struct EtatService {
    debut_suivi: i64,
    etat: Niveau,
    depuis: i64,
    message: String,
}

struct EtatNode {
    etat: Niveau,
    ms: Option<f64>,
    info: String,
    depuis: i64,
    dernier: Option<i64>,
    /// Echecs d'affilee.
    echecs: u32,
    /// Controles reussis mais lents d'affilee.
    lenteurs: u32,
    prochain: Instant,
    en_cours: bool,
}

impl EtatNode {
    /// Compte le resultat d'un controle et rend l'etat qui en decoule : une node ne passe hors
    /// ligne (ou lente) qu'apres plusieurs echecs (ou lenteurs) d'affilee.
    fn etat_apres(&mut self, resultat: &Resultat, conf: &Config) -> Niveau {
        if !resultat.ok {
            self.echecs += 1;
            self.lenteurs = 0;
            return if self.echecs >= conf.echecs_avant_panne {
                Niveau::Down
            } else {
                self.etat
            };
        }
        self.echecs = 0;
        if resultat.ms.unwrap_or(0.0) > conf.seuil_lent as f64 {
            self.lenteurs += 1;
        } else {
            self.lenteurs = 0;
        }
        if self.lenteurs >= conf.lenteurs_avant_alerte {
            Niveau::Lent
        } else {
            Niveau::Up
        }
    }
}

struct EtatTailscale {
    tablette_ip: Option<String>,
    tablette_depuis: i64,
    tablette_connue: bool,
    exit_node: bool,
    exit_depuis: i64,
    api_erreur: Option<String>,
    api_maj: Option<i64>,
    appareils: Option<Vec<Appareil>>,
    tailnet: Option<String>,
}

/// Secondes depuis l'epoque Unix.
pub fn maintenant() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Moniteur {
    pub fn ouvrir(conf: &Config, chemin_base: &Path) -> rusqlite::Result<Moniteur> {
        let base = Base::ouvrir(chemin_base)?;
        let t0 = maintenant();
        let mut services = Vec::new();
        let mut etats_nodes = Vec::new();
        let mut nodes = Vec::new();
        let mut plages = Vec::new();
        let premier_controle = Instant::now();
        for (s, service) in conf.services.iter().enumerate() {
            services.push(EtatService {
                debut_suivi: base.debut_suivi(&service.id, t0)?,
                etat: Niveau::Inconnu,
                depuis: t0,
                message: String::new(),
            });
            let debut = nodes.len();
            for n in 0..service.nodes.len() {
                // premiers controles etales de 250 ms
                let decalage = Duration::from_millis(250 * nodes.len() as u64);
                nodes.push((s, n));
                etats_nodes.push(EtatNode {
                    etat: Niveau::Inconnu,
                    ms: None,
                    info: "En attente du premier contrôle".into(),
                    depuis: t0,
                    dernier: None,
                    echecs: 0,
                    lenteurs: 0,
                    prochain: premier_controle + decalage,
                    en_cours: false,
                });
            }
            plages.push(debut..nodes.len());
        }

        // incidents restes ouverts lors d'un arret precedent
        let mut incidents_ouverts = HashMap::new();
        for (id, service) in base.incidents_ouverts()? {
            let connu = conf.services.iter().any(|s| s.id == service)
                || service == "tailscale"
                || service == "exit-node";
            if connu {
                incidents_ouverts.insert(service, id);
            } else {
                base.fermer_incident(id, t0)?;
            }
        }

        let etat = Etat {
            base,
            services,
            nodes: etats_nodes,
            incidents_ouverts,
            tailscale: EtatTailscale {
                tablette_ip: None,
                tablette_depuis: t0,
                tablette_connue: false,
                exit_node: false,
                exit_depuis: t0,
                api_erreur: None,
                api_maj: None,
                appareils: None,
                tailnet: None,
            },
            series: None,
        };
        Ok(Moniteur {
            conf: conf.clone(),
            nodes,
            plages,
            etat: Mutex::new(etat),
        })
    }

    pub fn conf(&self) -> &Config {
        &self.conf
    }

    /// Un fil de controle qui panique ne doit pas bloquer les autres : l'etat reste utilisable.
    fn verrou(&self) -> MutexGuard<'_, Etat> {
        self.etat.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn config_node(&self, i: usize) -> &ConfigNode {
        let (s, n) = self.nodes[i];
        &self.conf.services[s].nodes[n]
    }

    /// Lance les controles dus toutes les demi-secondes, et fait le menage de l'historique
    /// toutes les heures. Ne rend jamais la main.
    pub fn boucle_controles(self: &Arc<Self>) {
        let (envoi, reception) = mpsc::channel::<usize>();
        let reception = Arc::new(Mutex::new(reception));
        for _ in 0..CONTROLEURS {
            let moniteur = Arc::clone(self);
            let reception = Arc::clone(&reception);
            thread::spawn(move || {
                loop {
                    let suivant = reception
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .recv();
                    let Ok(i) = suivant else { return };
                    moniteur.controler(i);
                }
            });
        }
        let mut dernier_menage: Option<Instant> = None;
        loop {
            let instant = Instant::now();
            let dus: Vec<usize> = {
                let mut etat = self.verrou();
                etat.nodes
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, n)| n.prochain <= instant && !n.en_cours)
                    .map(|(i, n)| {
                        n.en_cours = true;
                        i
                    })
                    .collect()
            };
            for i in dus {
                let _ = envoi.send(i);
            }
            if dernier_menage.is_none_or(|d| d.elapsed() >= Duration::from_secs(3600)) {
                dernier_menage = Some(instant);
                if let Err(e) = self.verrou().base.menage(maintenant()) {
                    journal!("menage de l'historique impossible : {e}");
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    fn controler(&self, i: usize) {
        let node = self.config_node(i);
        let resultat =
            panic::catch_unwind(AssertUnwindSafe(|| controles::controler(node, &self.conf)))
                .unwrap_or_else(|panique| Resultat {
                    ok: false,
                    ms: None,
                    info: tronquer(
                        &format!("Erreur interne : {}", message_de_panique(panique.as_ref())),
                        110,
                    ),
                });
        let t = maintenant();
        let mut garde = self.verrou();
        let etat = &mut *garde;
        let n = &mut etat.nodes[i];
        n.dernier = Some(t);
        n.ms = resultat.ms.map(|ms| (ms * 10.0).round() / 10.0);
        let nouvel = n.etat_apres(&resultat, &self.conf);
        n.info = resultat.info;
        if nouvel != n.etat {
            n.etat = nouvel;
            n.depuis = t;
        }
        let ms_mesure = if resultat.ok { resultat.ms } else { None };
        if let Err(e) = etat
            .base
            .ajouter_mesure(t, &node.id, resultat.ok, ms_mesure)
        {
            journal!("mesure de {} non enregistree : {e}", node.id);
        }
        self.evaluer_service(etat, self.nodes[i].0, t);
        let intervalle = if resultat.ok {
            self.conf.intervalle
        } else {
            self.conf.intervalle.min(10)
        };
        let n = &mut etat.nodes[i];
        n.prochain = Instant::now() + Duration::from_secs(intervalle);
        n.en_cours = false;
    }

    /// Controle immediat de toutes les nodes, ou de celles d'un service.
    pub fn relancer(&self, service: Option<&str>) {
        let mut etat = self.verrou();
        let instant = Instant::now();
        for (i, n) in etat.nodes.iter_mut().enumerate() {
            if service.is_none_or(|id| self.config_node(i).service == id) {
                n.prochain = instant;
            }
        }
    }

    pub fn acquitter(&self, id: i64) -> rusqlite::Result<()> {
        self.verrou().base.acquitter(id)
    }

    fn evaluer_service(&self, etat: &mut Etat, s: usize, t: i64) {
        let service = &self.conf.services[s];
        let nodes: Vec<(&str, &EtatNode)> = self.plages[s]
            .clone()
            .map(|i| (self.config_node(i).nom.as_str(), &etat.nodes[i]))
            .collect();
        let (niveau, message) = evaluer(&nodes);
        let es = &mut etat.services[s];
        if niveau == es.etat && message == es.message {
            return;
        }
        if niveau != es.etat {
            es.depuis = t;
        }
        es.etat = niveau;
        es.message = message.clone();
        if niveau != Niveau::Inconnu {
            maj_incident(etat, &service.id, &service.nom, niveau, &message, t);
        }
    }

    /// Etat de la tablette sur le tailnet (verifie toutes les 10 s par la boucle Tailscale).
    pub fn noter_tablette_tailscale(&self, ip: Option<Ipv4Addr>, sortie: bool) {
        let t = maintenant();
        let ip = ip.map(|ip| ip.to_string());
        let mut garde = self.verrou();
        let etat = &mut *garde;
        let ts = &mut etat.tailscale;
        if ip != ts.tablette_ip || !ts.tablette_connue {
            if ip.is_some() != ts.tablette_ip.is_some() || !ts.tablette_connue {
                ts.tablette_depuis = t;
            }
            ts.tablette_ip.clone_from(&ip);
            ts.tablette_connue = true;
        }
        if sortie != ts.exit_node {
            ts.exit_node = sortie;
            ts.exit_depuis = t;
            journal!(
                "exit node {}",
                if sortie {
                    "ACTIVE sur la tablette"
                } else {
                    "desactive"
                }
            );
        }
        if self.conf.tailscale.actif {
            let niveau = if ip.is_some() {
                Niveau::Up
            } else {
                Niveau::Down
            };
            maj_incident(
                etat,
                "tailscale",
                "Tailscale",
                niveau,
                "La tablette n'est plus sur le tailnet",
                t,
            );
            let niveau = if sortie { Niveau::Warn } else { Niveau::Up };
            let message = "Exit node activé sur la tablette : les contrôles Internet passent par l'entreprise";
            maj_incident(etat, "exit-node", "Exit node", niveau, message, t);
        }
    }

    /// Reponse de l'API Tailscale : appareils et nom du tailnet, ou message d'erreur.
    pub fn noter_api_tailscale(&self, reponse: Result<(Vec<Appareil>, Option<String>), String>) {
        let mut etat = self.verrou();
        let ts = &mut etat.tailscale;
        match reponse {
            Ok((appareils, tailnet)) => {
                ts.api_erreur = None;
                ts.appareils = Some(appareils);
                ts.tailnet = tailnet;
                ts.api_maj = Some(maintenant());
            }
            Err(message) => ts.api_erreur = Some(message),
        }
    }
}

/// Etat d'un service et message affiche, d'apres ses nodes (nom, etat).
fn evaluer(nodes: &[(&str, &EtatNode)]) -> (Niveau, String) {
    let pannes: Vec<_> = nodes
        .iter()
        .filter(|(_, n)| n.etat == Niveau::Down)
        .collect();
    let lentes: Vec<_> = nodes
        .iter()
        .filter(|(_, n)| n.etat == Niveau::Lent)
        .collect();
    let unique = nodes.len() == 1;
    if nodes.iter().all(|(_, n)| n.etat == Niveau::Inconnu) {
        return (Niveau::Inconnu, String::new());
    }
    if pannes.len() == nodes.len() {
        let info = &pannes[0].1.info;
        let message = if unique {
            format!("Ne répond plus : {info}")
        } else {
            format!("Aucune node ne répond ({info})")
        };
        return (Niveau::Down, message);
    }
    if pannes.is_empty() && lentes.is_empty() {
        return (Niveau::Up, String::new());
    }
    let ms = |n: &EtatNode| n.ms.unwrap_or(0.0).round_ties_even();
    let mut morceaux = Vec::new();
    if let Some((_, premiere)) = pannes.first() {
        if unique {
            morceaux.push(format!("Ne répond plus ({})", premiere.info));
        } else {
            let noms = pannes
                .iter()
                .map(|(nom, _)| *nom)
                .collect::<Vec<_>>()
                .join(", ");
            let verbe = if pannes.len() > 1 {
                "ne répondent plus"
            } else {
                "ne répond plus"
            };
            morceaux.push(format!("{noms} {verbe} ({})", premiere.info));
        }
    }
    if !lentes.is_empty() {
        if unique {
            morceaux.push(format!("Lent : {} ms", ms(lentes[0].1)));
        } else {
            let noms = lentes
                .iter()
                .map(|(nom, n)| format!("{nom} ({} ms)", ms(n)))
                .collect::<Vec<_>>()
                .join(", ");
            morceaux.push(format!("lent : {noms}"));
        }
    }
    (Niveau::Warn, morceaux.join(" · "))
}

/// Ouvre, prolonge ou clot l'incident de `cle` selon le nouvel etat.
fn maj_incident(etat: &mut Etat, cle: &str, nom: &str, niveau: Niveau, message: &str, t: i64) {
    let ouvert = etat.incidents_ouverts.get(cle).copied();
    let resultat = match (niveau, ouvert) {
        (Niveau::Warn | Niveau::Down, None) => etat
            .base
            .ouvrir_incident(cle, nom, niveau, message, t)
            .map(|id| {
                etat.incidents_ouverts.insert(cle.to_string(), id);
                journal!("incident ouvert : {nom} ({}) {message}", niveau.texte());
            }),
        (Niveau::Warn | Niveau::Down, Some(id)) => {
            etat.base.prolonger_incident(id, niveau, message, t)
        }
        (Niveau::Up, Some(id)) => etat.base.fermer_incident(id, t).map(|()| {
            etat.incidents_ouverts.remove(cle);
            journal!("incident clos : {nom}");
        }),
        _ => Ok(()),
    };
    if let Err(e) = resultat {
        journal!("incident {cle} non enregistre : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(etat: Niveau, ms: Option<f64>, info: &str) -> EtatNode {
        EtatNode {
            etat,
            ms,
            info: info.into(),
            depuis: 0,
            dernier: None,
            echecs: 0,
            lenteurs: 0,
            prochain: Instant::now(),
            en_cours: false,
        }
    }

    #[test]
    fn un_pic_de_latence_isole_ne_rend_pas_la_node_lente() {
        // valeurs par defaut : seuil 1000 ms, 3 lenteurs et 2 echecs d'affilee
        let conf = Config::depuis_texte("").unwrap();
        let mesure = |ms: f64| Resultat {
            ok: true,
            ms: Some(ms),
            info: String::new(),
        };
        let echec = || Resultat {
            ok: false,
            ms: None,
            info: String::new(),
        };
        let mut n = node(Niveau::Up, None, "");
        let mut suite = |resultat: &Resultat| {
            n.etat = n.etat_apres(resultat, &conf);
            n.etat
        };
        // pics isoles, entrecoupes de mesures normales ou d'un echec
        for r in [
            mesure(2500.0),
            mesure(1800.0),
            mesure(40.0),
            mesure(3000.0),
            echec(),
            mesure(1500.0),
        ] {
            assert_eq!(suite(&r), Niveau::Up);
        }
        // lenteur qui dure : alerte au troisieme controle lent d'affilee
        assert_eq!(suite(&mesure(1200.0)), Niveau::Up);
        assert_eq!(suite(&mesure(1300.0)), Niveau::Lent);
        assert_eq!(suite(&mesure(1100.0)), Niveau::Lent);
        // un seul controle rapide suffit a revenir
        assert_eq!(suite(&mesure(30.0)), Niveau::Up);
        // pannes : inchangees (2 echecs d'affilee)
        assert_eq!(suite(&echec()), Niveau::Up);
        assert_eq!(suite(&echec()), Niveau::Down);
    }

    #[test]
    fn evaluation_d_un_service_a_une_node() {
        let cas = [
            (node(Niveau::Inconnu, None, ""), Niveau::Inconnu, ""),
            (node(Niveau::Up, Some(12.0), "HTTP 200"), Niveau::Up, ""),
            (
                node(Niveau::Down, None, "Connexion refusée"),
                Niveau::Down,
                "Ne répond plus : Connexion refusée",
            ),
            (
                node(Niveau::Lent, Some(1500.5), "HTTP 200"),
                Niveau::Warn,
                "Lent : 1500 ms",
            ),
        ];
        for (n, niveau, message) in cas {
            assert_eq!(evaluer(&[("a", &n)]), (niveau, message.to_string()));
        }
    }

    #[test]
    fn evaluation_d_un_service_a_plusieurs_nodes() {
        let up = node(Niveau::Up, Some(10.0), "ok");
        let hs = node(Niveau::Down, None, "Délai dépassé après 10 s");
        let lent = node(Niveau::Lent, Some(2500.0), "ok");
        let attente = node(Niveau::Inconnu, None, "");
        assert_eq!(
            evaluer(&[("srv1", &hs), ("srv2", &lent), ("srv3", &up)]),
            (
                Niveau::Warn,
                "srv1 ne répond plus (Délai dépassé après 10 s) · lent : srv2 (2500 ms)".into()
            )
        );
        assert_eq!(
            evaluer(&[("srv1", &hs), ("srv2", &hs), ("srv3", &up)]).1,
            "srv1, srv2 ne répondent plus (Délai dépassé après 10 s)"
        );
        assert_eq!(
            evaluer(&[("a", &hs), ("b", &hs)]),
            (
                Niveau::Down,
                "Aucune node ne répond (Délai dépassé après 10 s)".into()
            )
        );
        assert_eq!(evaluer(&[("a", &up), ("b", &attente)]).0, Niveau::Up);
        assert_eq!(evaluer(&[("a", &hs), ("b", &attente)]).0, Niveau::Warn);
    }
}
