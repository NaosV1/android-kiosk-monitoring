//! Controle d'une node : requete HTTP(S), port TCP, ping ICMP ou resolution DNS.
//! Chaque controle rend un [`Resultat`] dont le texte est deja redige pour l'ecran.

mod dns;
mod http;
mod ping;
pub mod reseau;

use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::config::{Config, ConfigNode, Controle};
use crate::https;

pub struct Resultat {
    pub ok: bool,
    /// Temps de reponse ; aussi renseigne pour un echec HTTP qui a recu une reponse.
    pub ms: Option<f64>,
    pub info: String,
}

impl Resultat {
    fn reussi(ms: f64, info: impl Into<String>) -> Resultat {
        Resultat {
            ok: true,
            ms: Some(ms),
            info: info.into(),
        }
    }

    fn echec(info: impl Into<String>) -> Resultat {
        Resultat {
            ok: false,
            ms: None,
            info: info.into(),
        }
    }

    fn echec_mesure(ms: f64, info: impl Into<String>) -> Resultat {
        Resultat {
            ok: false,
            ms: Some(ms),
            info: info.into(),
        }
    }
}

/// Echec deja redige pour l'ecran (resolution, route Tailscale...).
#[derive(Debug)]
pub struct Echec(pub String);

pub fn controler(node: &ConfigNode, conf: &Config) -> Resultat {
    match &node.controle {
        Controle::Http(h) => http::controler(node, h, conf),
        Controle::Ping { hote } => ping::controler(node, hote, conf.delai),
        Controle::Dns { hote } => dns::controler(node, hote, conf.delai),
        Controle::Tcp { hote, port } => controler_tcp(hote, *port, conf.delai),
    }
}

fn controler_tcp(hote: &str, port: u16, delai: f64) -> Resultat {
    let debut = Instant::now();
    let adresses = match (hote, port).to_socket_addrs() {
        Ok(a) => a,
        Err(_) => return Resultat::echec("Nom introuvable (DNS)"),
    };
    let mut derniere_erreur = None;
    for adresse in adresses {
        match TcpStream::connect_timeout(&adresse, duree(delai)) {
            Ok(_) => return Resultat::reussi(ms_depuis(debut), format!("TCP {port} ouvert")),
            Err(e) => derniere_erreur = Some(e),
        }
    }
    Resultat::echec(match derniere_erreur {
        Some(e) => https::raison_io(&e, delai),
        None => "Nom introuvable (DNS)".into(),
    })
}

/// Pour une node `par_tailscale`, refuse de controler si la route ne passe pas par le tunnel
/// (rien n'est envoye).
fn verifier_route(node: &ConfigNode, ip: &str) -> Result<(), Echec> {
    if node.par_tailscale && !reseau::passe_par_tailscale(ip) {
        return Err(Echec(format!("Pas de route Tailscale vers {ip}")));
    }
    Ok(())
}

/// Precision affichee apres le resultat, pour les nodes a resolveur ou `par_tailscale` :
/// par ou est passe le controle. `hote` : cible configuree (pour ne pas repeter l'adresse).
fn chemin(node: &ConfigNode, hote: Option<&str>, ip: &str) -> String {
    if node.resolveur.is_none() && !node.par_tailscale {
        return String::new();
    }
    let voie = if reseau::passe_par_tailscale(ip) {
        "par Tailscale"
    } else {
        "en direct"
    };
    if hote == Some(ip) {
        format!(" · {voie}")
    } else {
        format!(" · {ip} {voie}")
    }
}

fn duree(secondes: f64) -> Duration {
    Duration::from_secs_f64(secondes.max(0.001))
}

fn ms_depuis(debut: Instant) -> f64 {
    debut.elapsed().as_secs_f64() * 1000.0
}
