//! Tailscale : presence de la tablette sur le tailnet, exit node utilise par la tablette, et
//! liste des appareils par l'API Tailscale quand une cle (lecture) est configuree.

use std::thread;
use std::time::{Duration, Instant};

use chrono::DateTime;
use serde_json::Value;

use crate::api::Appareil;
use crate::config::ConfigTailscale;
use crate::controles::reseau;
use crate::https;
use crate::moniteur::{Moniteur, maintenant};

/// Ne rend jamais la main : etat de la tablette toutes les 10 s, API toutes les minutes.
pub fn boucle(moniteur: &Moniteur) {
    let conf = &moniteur.conf().tailscale;
    let mut dernier_appel: Option<Instant> = None;
    loop {
        let ip = reseau::tablette_sur_tailnet();
        // exit node utilise par la tablette = Internet (ici 1.1.1.1) route dans le tunnel
        let sortie = ip.is_some() && reseau::passe_par_tailscale("1.1.1.1");
        moniteur.noter_tablette_tailscale(ip, sortie);
        if !conf.cle_api.is_empty()
            && dernier_appel.is_none_or(|d| d.elapsed() >= Duration::from_secs(60))
        {
            dernier_appel = Some(Instant::now());
            moniteur.noter_api_tailscale(interroger_api(conf));
        }
        thread::sleep(Duration::from_secs(10));
    }
}

/// Appareils du tailnet et nom du tailnet, ou message d'erreur pour l'ecran.
fn interroger_api(conf: &ConfigTailscale) -> Result<(Vec<Appareil>, Option<String>), String> {
    let url = format!(
        "https://api.tailscale.com/api/v2/tailnet/{}/devices?fields=all",
        https::encoder_segment(&conf.tailnet)
    );
    let injoignable = |raison: String| format!("API Tailscale injoignable ({raison})");
    let mut reponse = https::agent(Duration::from_secs(15), true, None)
        .get(&url)
        .header("Authorization", format!("Bearer {}", conf.cle_api))
        .header("User-Agent", "tablette-monitoring/1.0")
        .call()
        .map_err(|e| injoignable(https::raison(&e, 15.0)))?;
    match reponse.status().as_u16() {
        401 | 403 => return Err("Clé API refusée par Tailscale".into()),
        200..=299 => {}
        code => return Err(format!("API Tailscale : erreur HTTP {code}")),
    }
    let donnees: Value = serde_json::from_reader(reponse.body_mut().as_reader())
        .map_err(|e| injoignable(https::tronquer(&e.to_string(), 110)))?;
    Ok(lire_appareils(&donnees, maintenant()))
}

fn lire_appareils(donnees: &Value, t: i64) -> (Vec<Appareil>, Option<String>) {
    let mut appareils = Vec::new();
    let mut tailnet = None;
    for d in donnees
        .get("devices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let texte = |cle: &str| d.get(cle).and_then(Value::as_str).unwrap_or("").to_string();
        let liste = |cle: &str| -> Vec<String> {
            d.get(cle)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        };
        let ipv4 = liste("addresses").into_iter().find(|a| a.contains('.'));
        let vu = d
            .get("lastSeen")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|date| date.timestamp());
        let en_ligne = match d.get("connectedToControl") {
            Some(Value::Bool(b)) => *b,
            _ => vu.is_some_and(|v| t - v < 300),
        };
        let mut derp = None;
        let latences = d
            .get("clientConnectivity")
            .and_then(|c| c.get("latency"))
            .and_then(Value::as_object);
        for (region, info) in latences.into_iter().flatten() {
            if info.get("preferred").and_then(Value::as_bool) == Some(true) {
                let ms = info
                    .get("latencyMs")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    .round_ties_even();
                // code de region en capitales : « par » se lirait comme la preposition
                derp = Some(format!("{} · {ms} ms", region.to_uppercase()));
            }
        }
        let routes = liste("enabledRoutes");
        let fqdn = texte("name");
        if tailnet.is_none()
            && let Some((_, domaine)) = fqdn.split_once('.')
        {
            tailnet = Some(domaine.to_string());
        }
        let hostname = texte("hostname");
        appareils.push(Appareil {
            nom: if hostname.is_empty() {
                fqdn.split('.').next().unwrap_or("").to_string()
            } else {
                hostname
            },
            os: texte("os"),
            ip: ipv4.unwrap_or_default(),
            en_ligne,
            vu,
            derp,
            sortie: routes.iter().any(|r| r == "0.0.0.0/0"),
            sous_reseaux: routes
                .into_iter()
                .filter(|r| r != "0.0.0.0/0" && r != "::/0")
                .collect(),
            maj_dispo: d
                .get("updateAvailable")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            cette_tablette: false,
        });
    }
    (appareils, tailnet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lecture_des_appareils() {
        let donnees = json!({"devices": [
            {"name": "tablette.tail1234.ts.net", "hostname": "tablette", "os": "android",
             "addresses": ["fd7a:115c::1", "100.64.0.5"], "connectedToControl": true,
             "clientConnectivity": {"latency": {"par": {"latencyMs": 12.5, "preferred": true}, "fra": {"latencyMs": 20.0}}},
             "enabledRoutes": ["0.0.0.0/0", "::/0", "10.0.0.0/24"], "updateAvailable": true},
            {"name": "serveur.tail1234.ts.net", "os": "linux", "addresses": ["100.64.0.9"],
             "lastSeen": "2026-01-01T10:00:00Z"}
        ]});
        let t = DateTime::parse_from_rfc3339("2026-01-01T10:04:00Z")
            .unwrap()
            .timestamp();
        let (appareils, tailnet) = lire_appareils(&donnees, t);
        assert_eq!(tailnet.as_deref(), Some("tail1234.ts.net"));
        let a = &appareils[0];
        assert_eq!(
            (a.nom.as_str(), a.ip.as_str(), a.derp.as_deref()),
            ("tablette", "100.64.0.5", Some("PAR · 12 ms"))
        );
        assert!(a.en_ligne && a.sortie && a.maj_dispo);
        assert_eq!(a.sous_reseaux, ["10.0.0.0/24"]);
        let b = &appareils[1];
        assert_eq!(
            (b.nom.as_str(), b.en_ligne, b.vu),
            ("serveur", true, Some(t - 240))
        );
        assert!(!lire_appareils(&donnees, t + 120).0[1].en_ligne);
    }
}
