//! Controle HTTP(S) : code attendu, texte attendu, champs JSON attendus.

use std::time::{Duration, Instant};

use serde_json::Value;

use super::{Echec, Resultat, chemin, dns, ms_depuis, verifier_route};
use crate::config::{Config, ConfigNode, ControleHttp};
use crate::https::{self, AdresseImposee};

const TAILLE_LUE: u64 = 256 * 1024;

pub(super) fn controler(node: &ConfigNode, h: &ControleHttp, conf: &Config) -> Resultat {
    let verifier = node
        .verifier_certificat
        .unwrap_or(conf.verifier_certificats);
    let debut = Instant::now();
    let mut ip = h.ip.clone();
    if let Some(serveur) = &node.resolveur {
        match dns::resoudre_par(&h.hote, serveur, conf.delai) {
            Ok(ips) => ip = Some(ips[0].to_string()),
            Err(Echec(m)) => return Resultat::echec(m),
        }
    }
    if node.par_tailscale {
        let cible = match &ip {
            Some(ip) => ip.clone(),
            None => match dns::premiere_ipv4(&h.hote) {
                Ok(a) => a.to_string(),
                Err(Echec(m)) => return Resultat::echec(m),
            },
        };
        if let Err(Echec(m)) = verifier_route(node, &cible) {
            return Resultat::echec(m);
        }
    }
    // avec un resolveur, l'adresse obtenue aide a comprendre un echec
    let precision = match (&node.resolveur, &ip) {
        (Some(_), Some(ip)) => format!(" ({ip})"),
        _ => String::new(),
    };

    let adresse = ip.clone().map(|adresse| AdresseImposee {
        hote: h.hote.clone(),
        adresse,
    });
    let agent = https::agent(Duration::from_secs_f64(conf.delai), verifier, adresse);
    let (code, corps) = match agent
        .get(&h.url)
        .header("User-Agent", "tablette-monitoring/1.0")
        .call()
    {
        Ok(mut reponse) => {
            let code = reponse.status().as_u16();
            // comme urllib : le corps n'est lu que pour une reponse 2xx
            let corps = if (200..300).contains(&code) {
                match https::lire_debut(&mut reponse, TAILLE_LUE) {
                    Ok(c) => c,
                    Err(e) => {
                        return Resultat::echec(https::raison_io(&e, conf.delai) + &precision);
                    }
                }
            } else {
                Vec::new()
            };
            (code, corps)
        }
        Err(e) => return Resultat::echec(https::raison(&e, conf.delai) + &precision),
    };
    let voie = ip
        .as_deref()
        .map(|ip| chemin(node, None, ip))
        .unwrap_or_default();
    let ms = ms_depuis(debut);

    let code_ok = match h.code {
        Some(attendu) => code == attendu,
        None => (200..400).contains(&code),
    };
    if !code_ok {
        let message = match h.code {
            Some(attendu) => format!("HTTP {code} (attendu {attendu})"),
            None => format!("Erreur HTTP {code}"),
        };
        return Resultat::echec_mesure(ms, message + &voie);
    }
    if let Some(texte) = &h.contient
        && !corps.windows(texte.len()).any(|w| w == texte.as_bytes())
    {
        return Resultat::echec_mesure(ms, format!("Réponse inattendue (texte absent){voie}"));
    }
    if let Some(attendu) = &h.json {
        let Ok(recu) = serde_json::from_slice::<Value>(&corps) else {
            return Resultat::echec_mesure(
                ms,
                format!("HTTP {code} mais la réponse n'est pas du JSON{voie}"),
            );
        };
        if let Some(ecart) = ecart_json(attendu, &recu, "") {
            return Resultat::echec_mesure(ms, ecart);
        }
        return Resultat::reussi(ms, format!("HTTP {code} · JSON conforme{voie}"));
    }
    Resultat::reussi(ms, format!("HTTP {code}{voie}"))
}

/// Premier champ de `attendu` absent ou different dans `recu` (`None` si tout correspond).
fn ecart_json(attendu: &toml::Table, recu: &Value, parent: &str) -> Option<String> {
    for (cle, valeur) in attendu {
        let chemin = if parent.is_empty() {
            cle.clone()
        } else {
            format!("{parent}.{cle}")
        };
        let Some(v) = recu.as_object().and_then(|o| o.get(cle)) else {
            return Some(format!("{chemin} absent de la réponse"));
        };
        if let toml::Value::Table(sous_table) = valeur {
            if let Some(e) = ecart_json(sous_table, v, &chemin) {
                return Some(e);
            }
            continue;
        }
        let valeur = toml_vers_json(valeur);
        if !egales(v, &valeur) {
            return Some(format!(
                "{chemin} = {} (attendu {valeur})",
                https::tronquer(&v.to_string(), 40)
            ));
        }
    }
    None
}

/// Egalite JSON ou 1 == 1.0, mais un booleen n'est jamais egal a un nombre.
fn egales(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (x.as_i64(), y.as_i64()) {
            (Some(x), Some(y)) => x == y,
            _ => x.as_f64() == y.as_f64(),
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| egales(a, b))
        }
        _ => a == b,
    }
}

fn toml_vers_json(valeur: &toml::Value) -> Value {
    match valeur {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::from(*i),
        toml::Value::Float(f) => Value::from(*f),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_vers_json).collect()),
        toml::Value::Table(t) => Value::Object(
            t.iter()
                .map(|(k, v)| (k.clone(), toml_vers_json(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn table(texte: &str) -> toml::Table {
        texte.parse().unwrap()
    }

    #[test]
    fn json_conforme() {
        let attendu = table("status = \"ok\"\ndatabase.connected = true\nn = 1");
        let recu = json!({"status": "ok", "database": {"connected": true, "x": 2}, "n": 1.0, "autre": null});
        assert_eq!(ecart_json(&attendu, &recu, ""), None);
    }

    #[test]
    fn json_ecarts_dans_l_ordre_de_la_config() {
        let attendu = table("status = \"ok\"\ndatabase.connected = true");
        let cas = [
            (
                json!({"status": "ko", "database": {"connected": false}}),
                "status = \"ko\" (attendu \"ok\")",
            ),
            (
                json!({"status": "ok", "database": {"connected": 1}}),
                "database.connected = 1 (attendu true)",
            ),
            (
                json!({"status": "ok", "database": "en panne"}),
                "database.connected absent de la réponse",
            ),
            (json!(["status"]), "status absent de la réponse"),
        ];
        for (recu, message) in cas {
            assert_eq!(ecart_json(&attendu, &recu, "").as_deref(), Some(message));
        }
    }

    #[test]
    fn valeur_recue_tronquee_a_40_caracteres() {
        let attendu = table("v = \"court\"");
        let recu = json!({"v": "x".repeat(100)});
        let message = ecart_json(&attendu, &recu, "").unwrap();
        assert_eq!(
            message,
            format!("v = \"{} (attendu \"court\")", "x".repeat(39))
        );
    }
}
