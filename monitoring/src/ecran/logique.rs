//! Regles d'affichage independantes du rendu : problemes en cours, groupes par tag,
//! notes de release, echelle du graphe.

use std::collections::HashSet;

use crate::api::{Etat, Incident, Niveau, Service};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Accueil,
    Service(String),
    Tailscale,
    Incidents,
    Maj,
    Veille,
}

impl Page {
    /// Page a ouvrir pour un incident (`tailscale` et `exit-node` ne sont pas des services).
    pub fn de_incident(incident: &Incident) -> Page {
        match incident.service.as_str() {
            "tailscale" | "exit-node" => Page::Tailscale,
            id => Page::Service(id.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Probleme {
    pub niveau: Niveau,
    pub titre: String,
    pub court: String,
    pub detail: String,
    pub depuis: i64,
    pub page: Page,
}

/// Services degrades ou hors ligne, puis alertes Tailscale.
pub fn problemes(etat: &Etat) -> Vec<Probleme> {
    let mut liste: Vec<Probleme> = etat
        .services
        .iter()
        .filter(|s| matches!(s.etat, Niveau::Warn | Niveau::Down))
        .map(|s| {
            let hs = s.etat == Niveau::Down;
            Probleme {
                niveau: s.etat,
                titre: format!(
                    "{} {}",
                    s.nom,
                    if hs {
                        "est hors ligne"
                    } else {
                        "est dégradé"
                    }
                ),
                court: format!("{} {}", s.nom, if hs { "hors ligne" } else { "dégradé" }),
                detail: s.message.clone(),
                depuis: s.depuis,
                page: Page::Service(s.id.clone()),
            }
        })
        .collect();
    let t = &etat.tailscale;
    if t.actif && t.tablette.ip.is_none() {
        liste.push(Probleme {
            niveau: Niveau::Down,
            titre: "Tailscale déconnecté".into(),
            court: "Tailscale déconnecté".into(),
            detail: "La tablette n'est plus sur le tailnet".into(),
            depuis: t.tablette.depuis,
            page: Page::Tailscale,
        });
    }
    if t.actif && t.exit_node.actif {
        liste.push(Probleme {
            niveau: Niveau::Warn,
            titre: "Exit node activé sur la tablette".into(),
            court: "Exit node activé".into(),
            detail: "Les contrôles Internet passent par l'entreprise".into(),
            depuis: t.exit_node.depuis,
            page: Page::Tailscale,
        });
    }
    liste
}

/// `Down` si l'un des elements est hors ligne, sinon `Warn`.
pub fn pire(niveaux: impl IntoIterator<Item = Niveau>) -> Niveau {
    if niveaux.into_iter().any(|n| n == Niveau::Down) {
        Niveau::Down
    } else {
        Niveau::Warn
    }
}

pub fn nodes_en_ligne(service: &Service) -> usize {
    service
        .nodes
        .iter()
        .filter(|n| matches!(n.etat, Niveau::Up | Niveau::Lent))
        .count()
}

/// `3 nodes`, ou `1/3 nodes` quand certaines ne repondent pas.
pub fn compte_nodes(service: &Service) -> String {
    let (total, ok) = (service.nodes.len(), nodes_en_ligne(service));
    if service.etat == Niveau::Inconnu || ok == total {
        let mot = if total > 1 { "nodes" } else { "node" };
        return format!("{total} {mot}");
    }
    format!("{ok}/{total} {}", if total > 1 { "nodes" } else { "node" })
}

/// Ordre d'affichage : problemes d'abord.
pub fn rang(niveau: Niveau) -> u8 {
    match niveau {
        Niveau::Down => 0,
        Niveau::Warn | Niveau::Lent => 1,
        Niveau::Inconnu | Niveau::Vide => 2,
        Niveau::Up => 3,
    }
}

pub struct Groupe<'a> {
    /// Vide pour les services sans tag.
    pub tag: &'a str,
    pub services: Vec<&'a Service>,
}

/// Services groupes par tag dans l'ordre de la config (sans tag a la fin),
/// problemes en tete de chaque groupe.
pub fn groupes(services: &[Service]) -> Vec<Groupe<'_>> {
    let mut groupes: Vec<Groupe> = Vec::new();
    for s in services {
        match groupes.iter_mut().find(|g| g.tag == s.tag) {
            Some(g) => g.services.push(s),
            None => groupes.push(Groupe {
                tag: &s.tag,
                services: vec![s],
            }),
        }
    }
    groupes.sort_by_key(|g| g.tag.is_empty());
    for g in &mut groupes {
        g.services.sort_by_key(|s| rang(s.etat));
    }
    groupes
}

/// Niveau resume d'un groupe de services.
pub fn niveau_groupe(services: &[&Service]) -> Niveau {
    if services.iter().any(|s| s.etat == Niveau::Down) {
        Niveau::Down
    } else if services.iter().any(|s| s.etat == Niveau::Warn) {
        Niveau::Warn
    } else if services.iter().all(|s| s.etat == Niveau::Inconnu) {
        Niveau::Inconnu
    } else {
        Niveau::Up
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlocNote {
    Paragraphe(String),
    Liste(Vec<String>),
}

/// Notes de release en texte brut : lignes `- ...` en liste, le reste en paragraphes
/// (titres markdown sans leurs `#`).
pub fn notes_release(texte: &str) -> Vec<BlocNote> {
    let mut blocs: Vec<BlocNote> = Vec::new();
    for ligne in texte.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let puce = ligne
            .strip_prefix("- ")
            .or_else(|| ligne.strip_prefix("* "));
        match (puce, blocs.last_mut()) {
            (Some(element), Some(BlocNote::Liste(liste))) => liste.push(element.to_string()),
            (Some(element), _) => blocs.push(BlocNote::Liste(vec![element.to_string()])),
            (None, _) => blocs.push(BlocNote::Paragraphe(
                ligne.trim_start_matches('#').trim_start().to_string(),
            )),
        }
    }
    blocs
}

/// Haut de l'axe du graphe : une valeur ronde au-dessus de `v`.
pub fn plafond(v: f64) -> f64 {
    const PALIERS: [f64; 16] = [
        10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 250.0, 400.0, 500.0, 1000.0, 2000.0, 2500.0, 5000.0,
        10000.0, 20000.0, 60000.0,
    ];
    PALIERS
        .into_iter()
        .find(|&p| v <= p)
        .unwrap_or_else(|| (v / 10000.0).ceil() * 10000.0)
}

/// Identifiants des incidents ouverts et pas encore acquittes (reveil de l'ecran).
pub fn incidents_a_signaler(etat: &Etat) -> HashSet<i64> {
    etat.incidents_ouverts
        .iter()
        .filter(|i| i.acquitte == 0)
        .map(|i| i.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::*;

    fn service(id: &str, tag: &str, etat: Niveau, nodes: &[Niveau]) -> Service {
        Service {
            id: id.into(),
            nom: id.to_uppercase(),
            adresse: String::new(),
            tag: tag.into(),
            etat,
            depuis: 100,
            message: format!("message {id}"),
            ms: None,
            spark: vec![],
            dispo30: None,
            nodes: nodes
                .iter()
                .enumerate()
                .map(|(i, &etat)| Node {
                    id: format!("{id}/{i}"),
                    nom: format!("n{i}"),
                    lieu: String::new(),
                    cible: String::new(),
                    type_controle: "tcp".into(),
                    etat,
                    ms: None,
                    info: String::new(),
                    depuis: 0,
                    dernier: None,
                })
                .collect(),
        }
    }

    fn etat(services: Vec<Service>, actif: bool, ip: Option<&str>, sortie: bool) -> Etat {
        Etat {
            maintenant: 0,
            intervalle: 30,
            seuil_lent: 1000,
            veille_minutes: 5,
            services,
            tailscale: Tailscale {
                actif,
                tablette: TabletteTailscale {
                    ip: ip.map(Into::into),
                    depuis: 50,
                },
                exit_node: ExitNode {
                    actif: sortie,
                    depuis: 60,
                },
                api: ApiTailscale {
                    configuree: false,
                    erreur: None,
                    maj: None,
                    tailnet: None,
                    appareils: None,
                },
            },
            incidents_ouverts: vec![],
            maj: Maj {
                actuelle: "dev".into(),
                depot: String::new(),
                heures: 6.0,
                statut: StatutMaj::Desactive,
                verifie: None,
                en_cours: false,
                erreur: None,
                disponible: None,
                derniere_installation: None,
            },
        }
    }

    #[test]
    fn problemes_services_puis_tailscale() {
        let e = etat(
            vec![
                service("a", "", Niveau::Up, &[Niveau::Up]),
                service("b", "", Niveau::Down, &[Niveau::Down]),
                service("c", "", Niveau::Warn, &[Niveau::Up, Niveau::Down]),
            ],
            true,
            None,
            true,
        );
        let p = problemes(&e);
        let titres: Vec<_> = p.iter().map(|p| p.titre.as_str()).collect();
        assert_eq!(
            titres,
            [
                "B est hors ligne",
                "C est dégradé",
                "Tailscale déconnecté",
                "Exit node activé sur la tablette"
            ]
        );
        assert_eq!(p[1].page, Page::Service("c".into()));
        assert_eq!(pire(p.iter().map(|p| p.niveau)), Niveau::Down);
        assert_eq!(pire([Niveau::Warn]), Niveau::Warn);
        assert!(problemes(&etat(vec![], false, None, true)).is_empty());
    }

    #[test]
    fn groupes_par_tag_problemes_en_tete() {
        let services = vec![
            service("a", "Réseau", Niveau::Up, &[Niveau::Up]),
            service("b", "", Niveau::Up, &[Niveau::Up]),
            service("c", "Web", Niveau::Up, &[Niveau::Up]),
            service("d", "Réseau", Niveau::Down, &[Niveau::Down]),
        ];
        let g = groupes(&services);
        let resume: Vec<(&str, Vec<&str>)> = g
            .iter()
            .map(|g| (g.tag, g.services.iter().map(|s| s.id.as_str()).collect()))
            .collect();
        assert_eq!(
            resume,
            [
                ("Réseau", vec!["d", "a"]),
                ("Web", vec!["c"]),
                ("", vec!["b"])
            ]
        );
        assert_eq!(niveau_groupe(&g[0].services), Niveau::Down);
        assert_eq!(niveau_groupe(&g[1].services), Niveau::Up);
    }

    #[test]
    fn compte_des_nodes() {
        assert_eq!(
            compte_nodes(&service("a", "", Niveau::Up, &[Niveau::Up, Niveau::Lent])),
            "2 nodes"
        );
        assert_eq!(
            compte_nodes(&service("a", "", Niveau::Warn, &[Niveau::Up, Niveau::Down])),
            "1/2 nodes"
        );
        assert_eq!(
            compte_nodes(&service("a", "", Niveau::Down, &[Niveau::Down])),
            "0/1 node"
        );
        assert_eq!(
            compte_nodes(&service("a", "", Niveau::Inconnu, &[Niveau::Inconnu])),
            "1 node"
        );
    }

    #[test]
    fn notes_de_release() {
        let notes = notes_release("## Modifications :\n\n- un (abc)\n* deux\nfin\n- trois\n");
        assert_eq!(
            notes,
            [
                BlocNote::Paragraphe("Modifications :".into()),
                BlocNote::Liste(vec!["un (abc)".into(), "deux".into()]),
                BlocNote::Paragraphe("fin".into()),
                BlocNote::Liste(vec!["trois".into()]),
            ]
        );
        assert!(notes_release(" \n").is_empty());
    }

    #[test]
    fn plafond_arrondi() {
        assert_eq!(plafond(8.0), 10.0);
        assert_eq!(plafond(120.0), 200.0);
        assert_eq!(plafond(60000.0), 60000.0);
        assert_eq!(plafond(61000.0), 70000.0);
    }
}
