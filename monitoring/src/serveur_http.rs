//! API JSON locale lue par l'ecran (routes et formes : voir `api.rs`).
//!
//! Elle n'ecoute que sur 127.0.0.1 par defaut (`[general] ecoute`). Comme dans la version
//! Python, elle n'a pas d'authentification : ne l'ouvrir que sur un reseau de confiance.

use std::error::Error;
use std::io::Read;
use std::sync::Arc;
use std::thread;

use serde::Serialize;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::api;
use crate::config::Config;
use crate::maj::MisesAJour;
use crate::moniteur::Moniteur;

const FILS: usize = 4;

/// Ne rend la main qu'en cas d'erreur d'ecoute.
pub fn servir(
    conf: &Config,
    moniteur: Arc<Moniteur>,
    maj: Arc<MisesAJour>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let serveur = Arc::new(Server::http((conf.ecoute.as_str(), conf.port))?);
    journal!("API sur http://{}:{}/api/etat", conf.ecoute, conf.port);
    let fils: Vec<_> = (0..FILS)
        .map(|_| {
            let (serveur, moniteur, maj) = (
                Arc::clone(&serveur),
                Arc::clone(&moniteur),
                Arc::clone(&maj),
            );
            thread::spawn(move || {
                for requete in serveur.incoming_requests() {
                    repondre(requete, &moniteur, &maj);
                }
            })
        })
        .collect();
    for f in fils {
        let _ = f.join();
    }
    Ok(())
}

struct Reponse {
    code: u16,
    corps: String,
}

fn repondre(mut requete: Request, moniteur: &Moniteur, maj: &Arc<MisesAJour>) {
    let chemin = requete.url().split('?').next().unwrap_or("").to_string();
    let methode = requete.method().clone();
    let segments: Vec<&str> = chemin.trim_start_matches('/').split('/').collect();
    let reponse = match (&methode, segments.as_slice()) {
        (Method::Get, ["api", "etat"]) => json_ou_erreur(moniteur.etat(maj.resume())),
        (Method::Get, ["api", "service", id]) => match moniteur.detail_service(&decoder(id)) {
            Ok(Some(detail)) => json(200, &detail),
            Ok(None) => erreur(404, "service inconnu"),
            Err(e) => erreur_interne(&chemin, &e),
        },
        (Method::Get, ["api", "incidents"]) => json_ou_erreur(moniteur.incidents()),
        (Method::Post, ["api", "incidents", id, "acquitter"])
            if id.bytes().all(|b| b.is_ascii_digit()) =>
        {
            match id.parse().map(|id| moniteur.acquitter(id)) {
                Ok(Ok(())) => confirmation(),
                Ok(Err(e)) => erreur_interne(&chemin, &e),
                Err(_) => erreur(404, "inconnu"),
            }
        }
        (Method::Post, ["api", "controler"]) => {
            moniteur.relancer(None);
            confirmation()
        }
        (Method::Post, ["api", "controler", id]) if !id.is_empty() => {
            moniteur.relancer(Some(&decoder(id)));
            confirmation()
        }
        (Method::Post, ["api", "maj", "verifier"]) => {
            maj.verifier_maintenant();
            confirmation()
        }
        (Method::Post, ["api", "maj", "installer"]) => installer(&mut requete, maj),
        _ => erreur(404, "inconnu"),
    };
    let entetes = [
        ("Content-Type", "application/json"),
        ("Cache-Control", "no-store"),
    ];
    let mut envoi = Response::from_string(reponse.corps).with_status_code(reponse.code);
    for (nom, valeur) in entetes {
        envoi.add_header(Header::from_bytes(nom, valeur).expect("en-tete constant valide"));
    }
    let _ = requete.respond(envoi);
}

/// La version demandee doit etre exactement celle proposee (confirmee sur l'ecran).
fn installer(requete: &mut Request, maj: &Arc<MisesAJour>) -> Reponse {
    let mut corps = Vec::new();
    if requete
        .as_reader()
        .take(4096)
        .read_to_end(&mut corps)
        .is_err()
    {
        return erreur(400, "requête invalide");
    }
    let demande = if corps.is_empty() {
        api::DemandeInstallation::default()
    } else {
        match serde_json::from_slice::<api::DemandeInstallation>(&corps) {
            Ok(d) => d,
            Err(_) => return erreur(400, "requête invalide"),
        }
    };
    match maj.installer(&demande.version) {
        Ok(()) => confirmation(),
        Err(message) => erreur(409, &message),
    }
}

fn json(code: u16, valeur: &impl Serialize) -> Reponse {
    match serde_json::to_string(valeur) {
        Ok(corps) => Reponse { code, corps },
        Err(e) => erreur(500, &e.to_string()),
    }
}

fn json_ou_erreur(resultat: rusqlite::Result<impl Serialize>) -> Reponse {
    match resultat {
        Ok(valeur) => json(200, &valeur),
        Err(e) => erreur_interne("api", &e),
    }
}

fn confirmation() -> Reponse {
    json(200, &api::Confirmation { ok: true })
}

fn erreur(code: u16, message: &str) -> Reponse {
    json(
        code,
        &api::Erreur {
            erreur: message.to_string(),
        },
    )
}

fn erreur_interne(chemin: &str, e: &rusqlite::Error) -> Reponse {
    journal!("erreur API {chemin} : {e}");
    erreur(500, &e.to_string())
}

/// Decodage `%XX` d'un segment d'url.
fn decoder(segment: &str) -> String {
    let octets = segment.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while i < octets.len() {
        let hex = octets
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (octets[i], hex) {
            (b'%', Some(o)) => {
                sortie.push(o);
                i += 3;
            }
            (o, _) => {
                sortie.push(o);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&sortie).into_owned()
}

#[cfg(test)]
mod tests {
    use super::decoder;

    #[test]
    fn decodage_des_segments() {
        assert_eq!(decoder("base-de-donnees"), "base-de-donnees");
        assert_eq!(decoder("caf%C3%A9%20x"), "café x");
        assert_eq!(decoder("100%"), "100%");
    }
}
