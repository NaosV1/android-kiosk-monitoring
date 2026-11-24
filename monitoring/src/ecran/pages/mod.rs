//! Les pages de l'ecran, construites a partir du dernier etat recu.

mod accueil;
mod incidents;
pub mod maj;
mod service;
mod tailscale;
mod veille;

use crate::api::{DetailService, Etat, Incidents};

use super::Filtre;
use super::commun::Vue;
use super::logique::Page;

/// Donnees disponibles pour construire une page.
pub struct Contexte<'a> {
    pub etat: &'a Etat,
    pub maintenant: i64,
    /// Detail du dernier service consulte (peut concerner un autre service).
    pub service: Option<&'a DetailService>,
    pub incidents: Option<&'a Incidents>,
    pub filtre: Filtre,
}

pub fn page<'a>(page: &Page, c: &Contexte<'a>) -> Vue<'a> {
    match page {
        Page::Accueil => accueil::vue(c),
        Page::Service(id) => match c.etat.services.iter().find(|s| &s.id == id) {
            Some(s) => service::vue(c, s),
            None => accueil::vue(c),
        },
        Page::Tailscale => tailscale::vue(c),
        Page::Incidents => incidents::vue(c),
        Page::Maj => maj::vue(c),
        Page::Veille => veille::vue(c),
    }
}
