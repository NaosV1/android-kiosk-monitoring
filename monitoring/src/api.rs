//! Contrat JSON de l'API locale, partage par le serveur (qui le produit) et l'ecran (qui le lit).
//!
//! Les noms et les formes sont ceux de l'ancienne version Python : l'ecran Rust peut donc aussi
//! afficher un serveur Python encore en place, et les scripts qui lisent `/api/etat` continuent
//! de fonctionner. Ne pas renommer un champ sans mettre a jour les deux cotes.
//!
//! Routes :
//! - `GET  /api/etat`                       -> [`Etat`]
//! - `GET  /api/service/<id>`               -> [`DetailService`] (404 si inconnu)
//! - `GET  /api/incidents`                  -> [`Incidents`]
//! - `POST /api/incidents/<id>/acquitter`   -> [`Confirmation`]
//! - `POST /api/controler[/<id service>]`   -> [`Confirmation`]
//! - `POST /api/maj/verifier`               -> [`Confirmation`]
//! - `POST /api/maj/installer` [`DemandeInstallation`] -> [`Confirmation`], ou 409 + [`Erreur`]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Etat d'une node, d'un service, d'un incident ou d'un jour de l'historique.
/// `Lent` ne concerne que les nodes ; `Vide` que les jours (avant le debut du suivi).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Niveau {
    Up,
    Lent,
    Warn,
    Down,
    Inconnu,
    Vide,
}

impl Niveau {
    /// Nom tel qu'ecrit dans le JSON et dans l'historique (`up`, `warn`...).
    pub fn texte(self) -> &'static str {
        match self {
            Niveau::Up => "up",
            Niveau::Lent => "lent",
            Niveau::Warn => "warn",
            Niveau::Down => "down",
            Niveau::Inconnu => "inconnu",
            Niveau::Vide => "vide",
        }
    }

    pub fn depuis_texte(texte: &str) -> Option<Niveau> {
        [
            Niveau::Up,
            Niveau::Lent,
            Niveau::Warn,
            Niveau::Down,
            Niveau::Inconnu,
            Niveau::Vide,
        ]
        .into_iter()
        .find(|n| n.texte() == texte)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Etat {
    pub maintenant: i64,
    /// Secondes entre deux controles d'une node.
    pub intervalle: u64,
    /// Au-dela (ms), une node est consideree comme lente.
    pub seuil_lent: i64,
    /// Passage en veille sans toucher l'ecran (0 ou moins = jamais).
    pub veille_minutes: i64,
    pub services: Vec<Service>,
    pub tailscale: Tailscale,
    pub incidents_ouverts: Vec<Incident>,
    pub maj: Maj,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Service {
    pub id: String,
    pub nom: String,
    pub adresse: String,
    pub tag: String,
    pub etat: Niveau,
    pub depuis: i64,
    pub message: String,
    /// Moyenne des temps de reponse des nodes en ligne.
    pub ms: Option<f64>,
    /// Temps de reponse moyens sur 24 h (72 tranches de 20 min, `None` = pas de mesure).
    pub spark: Vec<Option<f64>>,
    pub nodes: Vec<Node>,
    /// Disponibilite sur 30 jours, en %.
    pub dispo30: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Node {
    /// `<id du service>/<nom de la node>`
    pub id: String,
    pub nom: String,
    pub lieu: String,
    pub cible: String,
    /// `http`, `tcp`, `ping` ou `dns`.
    #[serde(rename = "type")]
    pub type_controle: String,
    pub etat: Niveau,
    pub ms: Option<f64>,
    pub info: String,
    pub depuis: i64,
    pub dernier: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tailscale {
    /// La tablette doit etre sur le tailnet (alerte sinon).
    pub actif: bool,
    pub tablette: TabletteTailscale,
    pub exit_node: ExitNode,
    pub api: ApiTailscale,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TabletteTailscale {
    /// Adresse Tailscale de la tablette, `None` si elle n'est pas sur le tailnet.
    pub ip: Option<String>,
    pub depuis: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExitNode {
    pub actif: bool,
    pub depuis: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiTailscale {
    /// Une cle API est configuree.
    pub configuree: bool,
    pub erreur: Option<String>,
    /// Derniere reponse de l'API.
    pub maj: Option<i64>,
    pub tailnet: Option<String>,
    /// `None` tant que l'API n'a pas repondu (ou sans cle).
    pub appareils: Option<Vec<Appareil>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Appareil {
    pub nom: String,
    pub os: String,
    pub ip: String,
    pub en_ligne: bool,
    pub vu: Option<i64>,
    /// Relais DERP prefere, ex. `par · 12 ms`.
    pub derp: Option<String>,
    /// Annonce une route de sortie (exit node).
    pub sortie: bool,
    pub sous_reseaux: Vec<String>,
    pub maj_dispo: bool,
    #[serde(default)]
    pub cette_tablette: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Incident {
    pub id: i64,
    /// Id du service, ou `tailscale` / `exit-node`.
    pub service: String,
    pub nom: String,
    pub niveau: Niveau,
    pub message: String,
    pub debut: i64,
    pub fin: Option<i64>,
    /// 0 ou 1 (valeur SQLite, comme dans la version Python).
    pub acquitte: i64,
    /// Secondes de panne totale deja cumulees.
    pub panne_s: i64,
    /// Debut de la panne totale en cours, le cas echeant.
    pub panne_depuis: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetailService {
    pub id: String,
    pub debut_courbe: i64,
    /// Largeur d'un point des courbes, en secondes.
    pub seau: i64,
    /// Courbe 24 h de chaque node, par id de node.
    pub courbes: BTreeMap<String, Vec<Option<f64>>>,
    /// 90 jours, du plus ancien a aujourd'hui : `up`, `warn`, `down` ou `vide`.
    pub jours: Vec<Niveau>,
    pub dispo90: Option<f64>,
    pub jours_avec_incident: i64,
    /// Derniers incidents du service, du plus recent au plus ancien.
    pub evenements: Vec<Incident>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Incidents {
    pub maintenant: i64,
    /// Incidents des 30 derniers jours, du plus recent au plus ancien.
    pub incidents: Vec<Incident>,
    pub bilan: Bilan,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Bilan {
    pub nombre: i64,
    /// Secondes de perturbation cumulees.
    pub duree: i64,
    /// Disponibilite moyenne des services sur 30 jours, en %.
    pub dispo: Option<f64>,
    /// Les 6 services les plus perturbes.
    pub par_service: Vec<PerturbationService>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PerturbationService {
    pub service: String,
    pub nom: String,
    pub duree: i64,
    pub en_cours: bool,
}

/// Etape du cycle de mise a jour.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatutMaj {
    /// Pas de depot configure.
    Desactive,
    /// Pas encore verifiee.
    Inconnu,
    Telechargement,
    /// Telechargee et verifiee, en attente de confirmation.
    Prete,
    AJour,
    Erreur,
    Installation,
    Redemarrage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Maj {
    /// Version installee, `dev` hors release.
    pub actuelle: String,
    /// Depot GitHub suivi (`proprietaire/depot`), vide si desactive.
    pub depot: String,
    /// Periode de verification, en heures.
    pub heures: f64,
    pub statut: StatutMaj,
    pub verifie: Option<i64>,
    pub en_cours: bool,
    pub erreur: Option<String>,
    pub disponible: Option<VersionDisponible>,
    pub derniere_installation: Option<DerniereInstallation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VersionDisponible {
    pub version: String,
    pub publiee: Option<i64>,
    /// Notes de release (texte brut).
    pub notes: String,
    pub url: Option<String>,
}

/// Ecrit dans `.maj/derniere-installation.json` a chaque installation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DerniereInstallation {
    pub version: String,
    pub depuis: String,
    pub quand: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct DemandeInstallation {
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Confirmation {
    pub ok: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Erreur {
    pub erreur: String,
}
