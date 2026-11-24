//! Rapports JSON lus par l'ecran : etat general, detail d'un service, bilan des incidents.

use std::collections::HashMap;

use chrono::{Local, Timelike};

use super::{Etat, Moniteur, maintenant};
use crate::api;
use crate::api::Niveau;

const JOUR: i64 = 86_400;
/// Largeur d'un point des courbes 24 h (20 min).
const SEAU: i64 = 1200;
/// 72 x 20 min = 24 h
const NB_SEAUX: usize = 72;

#[derive(Clone)]
pub(super) struct Series {
    calculees: i64,
    debut: i64,
    /// Par index plat de node.
    nodes: Vec<Vec<Option<f64>>>,
    /// Par index de service : moyenne de ses nodes.
    services: Vec<Vec<Option<f64>>>,
}

impl Moniteur {
    pub fn etat(&self, maj: api::Maj) -> rusqlite::Result<api::Etat> {
        let t = maintenant();
        let mut garde = self.verrou();
        let etat = &mut *garde;
        let series = self.series(etat, t)?;
        let mut services = Vec::new();
        for (s, service) in self.conf.services.iter().enumerate() {
            let es = &etat.services[s];
            let nodes: Vec<api::Node> = self.plages[s]
                .clone()
                .map(|i| {
                    let c = self.config_node(i);
                    let n = &etat.nodes[i];
                    api::Node {
                        id: c.id.clone(),
                        nom: c.nom.clone(),
                        lieu: c.lieu.clone(),
                        cible: c.cible.clone(),
                        type_controle: c.controle.nom_type().into(),
                        etat: n.etat,
                        ms: n.ms,
                        info: n.info.clone(),
                        depuis: n.depuis,
                        dernier: n.dernier,
                    }
                })
                .collect();
            let ms_en_ligne: Vec<f64> = nodes
                .iter()
                .filter(|n| matches!(n.etat, Niveau::Up | Niveau::Lent))
                .filter_map(|n| n.ms)
                .collect();
            let ms = (!ms_en_ligne.is_empty()).then(|| {
                (ms_en_ligne.iter().sum::<f64>() / ms_en_ligne.len() as f64).round_ties_even()
            });
            services.push(api::Service {
                id: service.id.clone(),
                nom: service.nom.clone(),
                adresse: service.adresse.clone(),
                tag: service.tag.clone(),
                etat: es.etat,
                depuis: es.depuis,
                message: es.message.clone(),
                ms,
                spark: series.services[s].clone(),
                nodes,
                dispo30: self.dispo(etat, s, 30, t)?,
            });
        }
        let incidents_ouverts = etat
            .base
            .incidents_entre(t - 400 * JOUR, t + 1, None)?
            .into_iter()
            .filter(|i| i.fin.is_none())
            .collect();
        Ok(api::Etat {
            maintenant: t,
            intervalle: self.conf.intervalle,
            seuil_lent: self.conf.seuil_lent,
            veille_minutes: self.conf.veille_minutes,
            services,
            tailscale: self.tailscale(etat),
            incidents_ouverts,
            maj,
        })
    }

    pub fn detail_service(&self, id: &str) -> rusqlite::Result<Option<api::DetailService>> {
        let Some(s) = self.conf.services.iter().position(|s| s.id == id) else {
            return Ok(None);
        };
        let t = maintenant();
        let mut garde = self.verrou();
        let etat = &mut *garde;
        let series = self.series(etat, t)?;
        let local = Local::now();
        let minuit = local.timestamp() - i64::from(local.num_seconds_from_midnight());
        let debut_90 = minuit - 89 * JOUR;
        let incidents = etat.base.incidents_entre(debut_90, t + 1, Some(id))?;
        let debut_suivi = etat.services[s].debut_suivi;
        let jours: Vec<Niveau> = (0..90)
            .map(|k| {
                let d0 = debut_90 + k * JOUR;
                let d1 = d0 + JOUR;
                if d1 <= debut_suivi {
                    return Niveau::Vide;
                }
                let niveaux: Vec<Niveau> = incidents
                    .iter()
                    .filter(|i| i.debut < d1 && i.fin.is_none_or(|f| f > d0))
                    .map(|i| i.niveau)
                    .collect();
                if niveaux.contains(&Niveau::Down) {
                    Niveau::Down
                } else if niveaux.is_empty() {
                    Niveau::Up
                } else {
                    Niveau::Warn
                }
            })
            .collect();
        Ok(Some(api::DetailService {
            id: id.to_string(),
            debut_courbe: series.debut,
            seau: SEAU,
            courbes: self.plages[s]
                .clone()
                .map(|i| (self.config_node(i).id.clone(), series.nodes[i].clone()))
                .collect(),
            jours_avec_incident: jours
                .iter()
                .filter(|j| matches!(j, Niveau::Warn | Niveau::Down))
                .count() as i64,
            jours,
            dispo90: self.dispo(etat, s, 90, t)?,
            evenements: incidents.into_iter().take(12).collect(),
        }))
    }

    pub fn incidents(&self) -> rusqlite::Result<api::Incidents> {
        let t = maintenant();
        let debut = t - 30 * JOUR;
        let mut garde = self.verrou();
        let etat = &mut *garde;
        let liste = etat.base.incidents_entre(debut, t + 1, None)?;
        let mut total = 0;
        // cumul par (service, nom), dans l'ordre de premiere apparition
        let mut par_service: Vec<((String, String), i64)> = Vec::new();
        for i in &liste {
            let duree = i.fin.unwrap_or(t) - i.debut.max(debut);
            total += duree;
            let cle = (i.service.clone(), i.nom.clone());
            match par_service.iter_mut().find(|(c, _)| *c == cle) {
                Some((_, d)) => *d += duree,
                None => par_service.push((cle, duree)),
            }
        }
        par_service.sort_by_key(|(_, duree)| -duree);
        let par_service = par_service
            .into_iter()
            .take(6)
            .map(|((service, nom), duree)| api::PerturbationService {
                en_cours: liste
                    .iter()
                    .any(|i| i.fin.is_none() && i.service == service),
                service,
                nom,
                duree,
            })
            .collect();
        let mut dispos = Vec::new();
        for s in 0..self.conf.services.len() {
            dispos.extend(self.dispo(etat, s, 30, t)?);
        }
        let dispo = (!dispos.is_empty())
            .then(|| arrondi(dispos.iter().sum::<f64>() / dispos.len() as f64, 2));
        Ok(api::Incidents {
            maintenant: t,
            bilan: api::Bilan {
                nombre: liste.len() as i64,
                duree: total,
                dispo,
                par_service,
            },
            incidents: liste,
        })
    }

    fn tailscale(&self, etat: &Etat) -> api::Tailscale {
        let ts = &etat.tailscale;
        let conf = &self.conf.tailscale;
        let appareils = ts.appareils.as_ref().map(|liste| {
            liste
                .iter()
                .map(|a| api::Appareil {
                    cette_tablette: ts.tablette_ip.as_deref().is_some_and(|ip| a.ip == ip)
                        || (!conf.nom_tablette.is_empty() && a.nom == conf.nom_tablette),
                    ..a.clone()
                })
                .collect()
        });
        api::Tailscale {
            actif: conf.actif,
            tablette: api::TabletteTailscale {
                ip: ts.tablette_ip.clone(),
                depuis: ts.tablette_depuis,
            },
            exit_node: api::ExitNode {
                actif: ts.exit_node,
                depuis: ts.exit_depuis,
            },
            api: api::ApiTailscale {
                configuree: !conf.cle_api.is_empty(),
                erreur: ts.api_erreur.clone(),
                maj: ts.api_maj,
                tailnet: ts.tailnet.clone(),
                appareils,
            },
        }
    }

    /// Courbes 24 h par node (moyenne par tranche de 20 min), recalculees au plus une fois par minute.
    fn series(&self, etat: &mut Etat, t: i64) -> rusqlite::Result<Series> {
        if let Some(s) = &etat.series
            && t - s.calculees < 60
        {
            return Ok(s.clone());
        }
        let debut = (t / SEAU) * SEAU - (NB_SEAUX as i64 - 1) * SEAU;
        let index: HashMap<&str, usize> = (0..self.nodes.len())
            .map(|i| (self.config_node(i).id.as_str(), i))
            .collect();
        let mut nodes = vec![vec![None; NB_SEAUX]; self.nodes.len()];
        for m in etat.base.moyennes(debut, SEAU)? {
            if let (Some(&i), Ok(k)) = (index.get(m.node.as_str()), usize::try_from(m.tranche))
                && k < NB_SEAUX
            {
                nodes[i][k] = m.ms.map(|ms| arrondi(ms, 1));
            }
        }
        let services = self
            .plages
            .iter()
            .map(|plage| {
                (0..NB_SEAUX)
                    .map(|k| {
                        let valeurs: Vec<f64> = plage.clone().filter_map(|i| nodes[i][k]).collect();
                        (!valeurs.is_empty())
                            .then(|| arrondi(valeurs.iter().sum::<f64>() / valeurs.len() as f64, 1))
                    })
                    .collect()
            })
            .collect();
        let series = Series {
            calculees: t,
            debut,
            nodes,
            services,
        };
        etat.series = Some(series.clone());
        Ok(series)
    }

    /// Disponibilite (%) sur `jours` jours, depuis le debut du suivi au plus tot.
    fn dispo(&self, etat: &Etat, s: usize, jours: i64, t: i64) -> rusqlite::Result<Option<f64>> {
        let debut = etat.services[s].debut_suivi.max(t - jours * JOUR);
        let observe = t - debut;
        if observe < 60 {
            return Ok(None);
        }
        let panne: i64 = etat
            .base
            .incidents_entre(debut, t, Some(&self.conf.services[s].id))?
            .iter()
            .map(|i| i.panne_s + i.panne_depuis.map_or(0, |d| t - d))
            .sum();
        Ok(Some(arrondi(
            (100.0 * (1.0 - panne as f64 / observe as f64)).max(0.0),
            2,
        )))
    }
}

fn arrondi(valeur: f64, decimales: i32) -> f64 {
    let facteur = 10f64.powi(decimales);
    (valeur * facteur).round() / facteur
}
