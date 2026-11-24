//! Detail d'un service : ses nodes, le graphe 24 h, la disponibilite sur 90 jours
//! et les derniers incidents.

use iced::widget::text::Wrapping;
use iced::widget::{Space, column, container, row};
use iced::{Alignment, Color, Length, Theme};

use crate::api::{DetailService, Niveau, Node, Service};

use super::Contexte;
use crate::ecran::commun::{
    self, Vue, bouton, bouton_retour, carte, defilant, ecran, entete, horloge, pastille, point,
    separateur, separateur_en_retrait, titre_section, titres,
};
use crate::ecran::format::{self, abreger, duree, il_y_a, ms, pct, pluriel};
use crate::ecran::graphiques::{Serie, graphe};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::{Page, nodes_en_ligne, rang};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, SERIES, t};
use crate::ecran::{Action, Message};

/// Debut du texte d'une ligne de node (marge, repere, espacement) : depart des filets.
const RETRAIT_NODE: f32 = 40.0;

pub fn vue<'a>(c: &Contexte<'a>, s: &'a Service) -> Vue<'a> {
    let detail = c.service.filter(|d| d.id == s.id);
    let total = s.nodes.len();
    let ok = nodes_en_ligne(s);
    let libelle = match s.etat {
        Niveau::Up => "Opérationnel".to_string(),
        Niveau::Warn if total > 1 => format!("Dégradé, {} sur {total}", pluriel(ok, "node")),
        Niveau::Warn | Niveau::Lent => "Dégradé".into(),
        Niveau::Down => "Hors ligne".into(),
        Niveau::Inconnu | Niveau::Vide => "En attente".into(),
    };
    let cible = if total == 1 {
        s.nodes[0].cible.clone()
    } else {
        s.adresse.clone()
    };
    let sous = row![
        commun::mono(abreger(&cible, 48), 13.5, theme::SECONDAIRE),
        t(
            format!(" · contrôle toutes les {} s", c.etat.intervalle),
            14
        )
        .color(theme::SECONDAIRE)
    ]
    .align_y(Alignment::Center);
    let tete = entete(vec![
        bouton_retour(),
        titres(
            s.nom.clone(),
            Some(pastille(Some(s.etat), libelle)),
            commun::sous_titre(sous.into()),
        ),
        bouton(
            Some(Icone::Relancer),
            "Relancer les contrôles",
            Some(Message::Agir(Action::Relance, s.id.clone())),
        ),
        horloge(),
    ]);

    let couleur_de = |i: usize| SERIES[i % SERIES.len()];
    let mut nodes: Vec<(usize, &Node)> = s.nodes.iter().enumerate().collect();
    nodes.sort_by_key(|(_, n)| rang(n.etat));
    let mut liste = column![];
    let nombre = nodes.len();
    for (k, (i, n)) in nodes.into_iter().enumerate() {
        if k > 0 {
            liste = liste.push(separateur_en_retrait(RETRAIT_NODE));
        }
        liste = liste.push(ligne_node(
            n,
            couleur_de(i),
            c.maintenant,
            k == 0,
            k + 1 == nombre,
        ));
    }
    let mut gauche = column![
        titre_section(
            if total > 1 { "Nodes" } else { "Node" },
            Some(format!("{ok} sur {total} en ligne"))
        ),
        container(liste).width(Length::Fill).style(theme::carte),
    ]
    .spacing(10);
    let en_ligne: Vec<&str> = s
        .nodes
        .iter()
        .filter(|n| matches!(n.etat, Niveau::Up | Niveau::Lent))
        .map(|n| n.nom.as_str())
        .collect();
    if s.etat == Niveau::Warn && total > 1 && !en_ligne.is_empty() {
        gauche = gauche.push(note_info(format!(
            "Le service reste joignable par {}. Il passera « hors ligne » seulement si plus aucune node ne répond.",
            en_ligne.join(" et ")
        )));
    }

    let mut legende = row![].spacing(16).align_y(Alignment::Center);
    for (i, n) in s.nodes.iter().enumerate() {
        legende = legende.push(
            row![
                trait_serie(couleur_de(i)),
                t(n.nom.clone(), 13).color(theme::SECONDAIRE)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    legende = legende.push(
        row![
            trait_seuil(),
            t(
                format!("lent au-delà de {}", ms(Some(c.etat.seuil_lent as f64))),
                13
            )
            .color(theme::SECONDAIRE)
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    );
    let courbe: Vue<'a> = match detail {
        None => t("Chargement…", 15).color(theme::SECONDAIRE).into(),
        Some(d) => graphe_service(s, d, c.etat.seuil_lent as f64, &couleur_de),
    };
    let droite = column![
        carte(
            column![
                t("Temps de réponse sur 24 h", 17).font(DEMI_GRAS),
                legende,
                courbe
            ]
            .spacing(10)
        ),
        carte_90_jours(detail),
        evenements(detail, c.maintenant)
    ]
    .spacing(14);
    let corps = row![
        defilant(gauche)
            .height(Length::Fill)
            .width(Length::FillPortion(5)),
        defilant(droite)
            .height(Length::Fill)
            .width(Length::FillPortion(7)),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(
        &Page::Service(s.id.clone()),
        c.etat,
        c.injoignable_depuis.is_some(),
        column![tete, corps].spacing(20),
    )
}

fn trait_serie<'a>(couleur: Color) -> Vue<'a> {
    container(Space::new())
        .width(14)
        .height(3)
        .style(move |_: &Theme| theme::boite(couleur, 2.0))
        .into()
}

/// Repere de legende du seuil « lent » : deux tirets ambre, comme la ligne du graphe.
fn trait_seuil<'a>() -> Vue<'a> {
    let tiret = || {
        container(Space::new())
            .width(6)
            .height(2)
            .style(|_: &Theme| theme::boite(theme::ton(Niveau::Warn).plein, 1.0))
    };
    row![tiret(), tiret()].spacing(3).into()
}

fn note_info<'a>(texte: String) -> Vue<'a> {
    container(
        row![
            icone(Icone::Info, 20.0, 2.0, theme::ACCENT),
            t(texte, 14).color(theme::TEXTE).width(Length::Fill)
        ]
        .spacing(12),
    )
    .padding([14, 16])
    .style(|_: &Theme| theme::boite(theme::ACCENT_FOND, 14.0))
    .into()
}

/// Une node dans la liste : teintee si elle est hors ligne ou lente.
fn ligne_node<'a>(
    n: &Node,
    couleur: Color,
    maintenant: i64,
    premiere: bool,
    derniere: bool,
) -> Vue<'a> {
    let ton = theme::ton(n.etat);
    let alerte = matches!(n.etat, Niveau::Down | Niveau::Lent);
    let (fond, secondaire) = if alerte {
        (ton.fond, ton.texte_fond)
    } else {
        (theme::SURFACE, theme::SECONDAIRE)
    };
    let lieu = if n.lieu.is_empty() {
        n.type_controle.to_uppercase()
    } else {
        n.lieu.clone()
    };
    let ident = column![
        row![
            trait_serie(couleur),
            t(abreger(&n.nom, 24), 16).font(DEMI_GRAS)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        t(lieu, 13).color(secondaire),
    ]
    .spacing(2)
    .width(Length::Fill);
    let (valeur, couleur_valeur, precision) = match n.etat {
        Niveau::Down => (
            "Hors ligne".to_string(),
            ton.texte,
            format!("depuis {}", duree((maintenant - n.depuis) as f64)),
        ),
        Niveau::Inconnu | Niveau::Vide => ("…".into(), theme::SECONDAIRE, n.info.clone()),
        Niveau::Lent => (ms(n.ms), ton.texte, format!("{}, lent", n.info)),
        _ => (ms(n.ms), theme::TEXTE, n.info.clone()),
    };
    let mesure = column![
        t(valeur, 17)
            .font(GRAS)
            .color(couleur_valeur)
            .wrapping(Wrapping::None),
        t(abreger(&precision, 34), 13)
            .color(secondaire)
            .wrapping(Wrapping::None),
    ]
    .spacing(2)
    .align_x(Alignment::End);
    let cible = container(commun::mono(abreger(&n.cible, 30), 12.5, secondaire))
        .clip(true)
        .width(Length::Fill);
    let bas: Vue<'a> = if n.etat == Niveau::Down {
        column![
            t(
                format!(
                    "{}, dernier contrôle {}",
                    n.info,
                    il_y_a(n.dernier, maintenant)
                ),
                14
            )
            .color(ton.texte_fond),
            cible,
        ]
        .spacing(4)
        .into()
    } else {
        row![
            cible,
            t(format!("contrôlé {}", il_y_a(n.dernier, maintenant)), 13)
                .color(secondaire)
                .wrapping(Wrapping::None)
        ]
        .spacing(16)
        .into()
    };
    container(
        column![
            row![point(n.etat, 10.0), ident, mesure]
                .spacing(12)
                .align_y(Alignment::Center),
            bas
        ]
        .spacing(8),
    )
    .padding([14, 18])
    .width(Length::Fill)
    .style(move |_: &Theme| theme::ligne_de_liste(fond, premiere, derniere))
    .into()
}

fn graphe_service<'a>(
    s: &Service,
    d: &DetailService,
    seuil: f64,
    couleur_de: &dyn Fn(usize) -> Color,
) -> Vue<'a> {
    let series: Vec<Serie> = s
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| Serie {
            couleur: couleur_de(i),
            valeurs: d.courbes.get(&n.id).cloned().unwrap_or_default(),
            hors_ligne: n.etat == Niveau::Down,
        })
        .collect();
    if series
        .iter()
        .all(|serie| serie.valeurs.iter().all(Option::is_none))
    {
        return container(
            t(
                "Pas encore de mesures : la courbe se remplit au fil des contrôles.",
                15,
            )
            .color(theme::SECONDAIRE),
        )
        .padding([40, 0])
        .center_x(Length::Fill)
        .into();
    }
    graphe(series, seuil)
}

fn carte_90_jours<'a>(detail: Option<&DetailService>) -> Vue<'a> {
    let titre = row![
        t("Disponibilité sur 90 jours", 17)
            .font(DEMI_GRAS)
            .width(Length::Fill),
        t(detail.map_or("—".into(), |d| pct(d.dispo90)), 22).font(GRAS),
    ]
    .align_y(Alignment::Center);
    let barres = row(detail
        .map(|d| d.jours.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|&jour| {
            let couleur = match jour {
                // les jours sans incident restent discrets : la couleur vive signale les problemes
                Niveau::Up => Color {
                    a: 0.45,
                    ..theme::couleur(Niveau::Up)
                },
                Niveau::Vide | Niveau::Inconnu => theme::PISTE,
                autre => theme::couleur(autre),
            };
            container(Space::new())
                .width(Length::Fill)
                .height(30)
                .style(move |_: &Theme| theme::boite(couleur, 2.0))
                .into()
        }))
    .spacing(2)
    .height(30);
    let avec_incident = detail
        .map(|d| {
            format!(
                "{} avec incident",
                pluriel(d.jours_avec_incident.max(0) as usize, "jour")
            )
        })
        .unwrap_or_default();
    let legende = row![
        t("il y a 90 jours", 12)
            .color(theme::SECONDAIRE)
            .width(Length::Fill),
        t(avec_incident, 12).color(theme::SECONDAIRE),
        container(t("aujourd'hui", 12).color(theme::SECONDAIRE))
            .width(Length::Fill)
            .align_x(Alignment::End),
    ];
    carte(column![titre, barres, legende].spacing(10)).into()
}

fn evenements<'a>(detail: Option<&DetailService>, maintenant: i64) -> Vue<'a> {
    let mut liste = column![t("Derniers événements", 17).font(DEMI_GRAS)].spacing(8);
    match detail.map(|d| &d.evenements) {
        Some(evenements) if !evenements.is_empty() => {
            let aujourd_hui = format::local(maintenant);
            let mut lignes = column![];
            for (i, inc) in evenements.iter().take(6).enumerate() {
                // l'issue est toujours lisible : seul le message est abrege
                let issue = match inc.fin {
                    Some(fin) => format!("résolu après {}", duree((fin - inc.debut) as f64)),
                    None => "en cours".into(),
                };
                let ligne = row![
                    container(commun::mono(
                        format::quand(&format::local(inc.debut), &aujourd_hui),
                        13.0,
                        theme::SECONDAIRE
                    ))
                    .width(100),
                    point(
                        if inc.fin.is_some() {
                            Niveau::Up
                        } else {
                            inc.niveau
                        },
                        10.0
                    ),
                    container(t(abreger(&inc.message, 30), 15).wrapping(Wrapping::None))
                        .clip(true)
                        .width(Length::Fill),
                    t(issue, 14)
                        .color(theme::SECONDAIRE)
                        .wrapping(Wrapping::None),
                ]
                .spacing(12)
                .height(commun::CIBLE)
                .align_y(Alignment::Center);
                if i > 0 {
                    lignes = lignes.push(separateur());
                }
                lignes = lignes.push(ligne);
            }
            liste = liste.push(lignes);
        }
        _ => liste = liste.push(t("Aucun incident enregistré.", 15).color(theme::SECONDAIRE)),
    }
    carte(liste).into()
}
