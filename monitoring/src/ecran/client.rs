//! Appels a l'API JSON du serveur de monitoring (bloquants : a lancer hors de l'interface).

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::api::{DemandeInstallation, DetailService, Erreur, Etat, Incidents};
use crate::https::encoder_segment;

#[derive(Clone)]
pub struct Client {
    base: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(base: String) -> Client {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(4)))
            .proxy(None)
            .http_status_as_error(false)
            .build()
            .into();
        Client { base, agent }
    }

    pub fn etat(&self) -> Result<Etat, String> {
        self.lire("/api/etat")
    }

    pub fn service(&self, id: &str) -> Result<DetailService, String> {
        self.lire(&format!("/api/service/{}", encoder_segment(id)))
    }

    pub fn incidents(&self) -> Result<Incidents, String> {
        self.lire("/api/incidents")
    }

    pub fn acquitter(&self, incident: i64) -> Result<(), String> {
        self.poster(&format!("/api/incidents/{incident}/acquitter"))
    }

    pub fn relancer(&self, service: &str) -> Result<(), String> {
        self.poster(&format!("/api/controler/{}", encoder_segment(service)))
    }

    pub fn verifier_maj(&self) -> Result<(), String> {
        self.poster("/api/maj/verifier")
    }

    /// `Err` porte le message du serveur quand il refuse (version plus proposee...).
    pub fn installer(&self, version: &str) -> Result<(), String> {
        let corps = serde_json::to_string(&DemandeInstallation {
            version: version.to_string(),
        })
        .map_err(|e| e.to_string())?;
        let mut reponse = self
            .agent
            .post(format!("{}/api/maj/installer", self.base))
            .content_type("application/json")
            .send(corps.as_str())
            .map_err(|e| e.to_string())?;
        if reponse.status().is_success() {
            return Ok(());
        }
        let texte = reponse.body_mut().read_to_string().unwrap_or_default();
        Err(serde_json::from_str::<Erreur>(&texte)
            .map_or_else(|_| "Installation refusée".into(), |e| e.erreur))
    }

    fn lire<T: DeserializeOwned>(&self, chemin: &str) -> Result<T, String> {
        let mut reponse = self
            .agent
            .get(format!("{}{chemin}", self.base))
            .call()
            .map_err(|e| e.to_string())?;
        if !reponse.status().is_success() {
            return Err(format!("HTTP {}", reponse.status().as_u16()));
        }
        let texte = reponse
            .body_mut()
            .read_to_string()
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&texte).map_err(|e| e.to_string())
    }

    fn poster(&self, chemin: &str) -> Result<(), String> {
        let reponse = self
            .agent
            .post(format!("{}{chemin}", self.base))
            .send_empty()
            .map_err(|e| e.to_string())?;
        if reponse.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP {}", reponse.status().as_u16()))
        }
    }
}
