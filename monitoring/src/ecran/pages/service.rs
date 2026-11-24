//! Detail d'un service : ses nodes, le graphe 24 h, la disponibilite sur 90 jours
//! et les derniers incidents.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Color, Length, Theme};

use crate::api::{DetailService, Niveau, Node, Service};

use super::Contexte;
use crate::ecran::commun::{
    self, Vue, badge, defilant, ecran, entete, etiquette, mono_discret, titre_section, titres,
};
use crate::ecran::format::{self, duree, il_y_a, ms, pct, pluriel};
use crate::ecran::graphiques::{Serie, graphe};
use crate::ecran::icones::{Icone, icone, icone_etat};
use crate::ecran::logique::{Page, nodes_en_ligne, rang};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MONO, SERIES, t};
use crate::ecran::{Action, Message};

pub fn vue<'a>(c: &Contexte<'a>, s: &'a Service) -> Vue<'a> {
    let detail = c.service.filter(|d| d.id == s.id);
    let total = s.nodes.len();
    let ok = nodes_en_ligne(s);
    let libelle = match s.etat {
        Niveau::Up => "Opérationnel".to_string(),
        Niveau::Warn if total > 1 => format!("Dégradé · {ok}/{total} nodes"),
        Niveau::Warn | Niveau::Lent => "Dégradé".into(),
        Niveau::Down => "Hors ligne".into(),
        Niveau::Inconnu | Niveau::Vide => "En attente".into(),
    };
    let cible: Vue<'a> = if total == 1 {
        t(s.nodes[0].cible.clone(), 14)
            .font(MONO)
            .color(theme::DISCRET)
            .into()
    } else {
        t(s.adresse.clone(), 14).color(theme::DISCRET).into()
    };
    let sous = row![
        cible,
        t(
            format!(" · contrôle toutes les {} s", c.etat.intervalle),
            14
        )
        .color(theme::DISCRET)
    ];
    let retour =
        button(container(icone(Icone::Gauche, 26.0, 2.2, theme::TEXTE)).center(Length::Fill))
            .width(56)
            .height(56)
            .padding(0)
            .style(theme::touchable(
                Some(theme::SURFACE),
                Some(theme::BORD2),
                16.0,
                theme::TEXTE,
            ))
            .on_press(Message::Aller(Page::Accueil));
    let tete = entete(vec![
        retour.into(),
        titres(
            s.nom.clone(),
            Some(etiquette(
                Some(s.etat),
                vec![
                    icone_etat(s.etat, 16.0),
                    t(libelle, 14).font(DEMI_GRAS).into(),
                ],
            )),
            commun::sous_titre(sous.into()),
        ),
        commun::bouton(
            Some(Icone::Relancer),
            "Relancer les contrôles",
            Some(Message::Agir(Action::Relance, s.id.clone())),
        ),
    ]);

    let couleur_de = |i: usize| SERIES[i % SERIES.len()];
    let mut nodes: Vec<(usize, &Node)> = s.nodes.iter().enumerate().collect();
    nodes.sort_by_key(|(_, n)| rang(n.etat));
    let mut gauche = column![titre_section(
        if total > 1 { "Nodes" } else { "Node" },
        Some(format!("{ok} sur {total} en ligne"))
    )]
    .spacing(12);
    for (i, n) in nodes {
        gauche = gauche.push(carte_node(n, couleur_de(i), c.maintenant));
    }
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

    let mut legende = row![
        t("Temps de réponse · 24 h", 17)
            .font(DEMI_GRAS)
            .width(Length::Fill)
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    for (i, n) in s.nodes.iter().enumerate() {
        legende = legende.push(
            row![
                trait_serie(couleur_de(i)),
                t(n.nom.clone(), 13).color(theme::TEXTE2)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let courbe: Vue<'a> = match detail {
        None => t("Chargement…", 16).color(theme::DISCRET).into(),
        Some(d) => graphe_service(s, d, c.etat.seuil_lent as f64, &couleur_de),
    };
    let carte_graphe = carte(column![legende, courbe].spacing(10));
    let droite = column![
        carte_graphe,
        carte_90_jours(detail),
        evenements(detail, c.maintenant)
    ]
    .spacing(12);
    let corps = row![
        defilant(gauche)
            .height(Length::Fill)
            .width(Length::FillPortion(5)),
        container(droite)
            .width(Length::FillPortion(7))
            .height(Length::Fill),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(
        &Page::Service(s.id.clone()),
        c.etat,
        column![tete, corps].spacing(18),
    )
}

fn carte<'a>(contenu: impl Into<Vue<'a>>) -> container::Container<'a, Message> {
    container(contenu)
        .padding([16, 20])
        .width(Length::Fill)
        .style(theme::carte)
}

fn trait_serie<'a>(couleur: Color) -> Vue<'a> {
    container(Space::new())
        .width(14)
        .height(3)
        .style(move |_: &Theme| theme::boite(couleur, None, 2.0))
        .into()
}

fn note_info<'a>(texte: String) -> Vue<'a> {
    container(
        row![
            icone(Icone::Info, 20.0, 2.0, theme::ACCENT),
            t(texte, 14).color(theme::DISCRET).width(Length::Fill)
        ]
        .spacing(12),
    )
    .padding([14, 16])
    .style(|_: &Theme| theme::boite(theme::CREUX, Some(theme::BORD2), 14.0))
    .into()
}

fn carte_node<'a>(n: &Node, couleur: Color, maintenant: i64) -> Vue<'a> {
    let lieu = if n.lieu.is_empty() {
        n.type_controle.to_uppercase()
    } else {
        n.lieu.clone()
    };
    let ident = column![
        row![trait_serie(couleur), t(n.nom.clone(), 18).font(DEMI_GRAS)]
            .spacing(8)
            .align_y(Alignment::Center),
        t(lieu, 13).color(theme::DISCRET),
    ]
    .spacing(2)
    .width(Length::Fill);
    let cible = container(
        t(n.cible.clone(), 13)
            .font(MONO)
            .color(theme::DISCRET)
            .wrapping(Wrapping::None),
    )
    .clip(true)
    .width(Length::Fill);
    let (fond, bord) = theme::carte_etat(n.etat).unwrap_or((theme::SURFACE, theme::BORD));
    let contenu = if n.etat == Niveau::Down {
        let mesure = column![
            t("Hors ligne", 18).font(GRAS).color(theme::DOWN),
            t(
                format!("depuis {}", duree((maintenant - n.depuis) as f64)),
                13
            )
            .color(theme::DOWN_CLAIR),
        ]
        .spacing(2)
        .align_x(Alignment::End);
        column![
            row![badge(n.etat, 36.0, 18.0), ident, mesure]
                .spacing(12)
                .align_y(Alignment::Center),
            t(
                format!(
                    "{} · dernier contrôle {}",
                    n.info,
                    il_y_a(n.dernier, maintenant)
                ),
                14
            )
            .color(theme::DOWN_PALE),
            cible,
        ]
    } else {
        let lent = n.etat == Niveau::Lent;
        let valeur = if n.etat == Niveau::Inconnu {
            "…".into()
        } else {
            ms(n.ms)
        };
        let info = if lent {
            format!("{} · lent", n.info)
        } else {
            n.info.clone()
        };
        let mesure = column![
            t(valeur, 20)
                .font(DEMI_GRAS)
                .color(if lent { theme::WARN } else { theme::TEXTE }),
            t(info, 13).color(theme::DISCRET),
        ]
        .spacing(2)
        .align_x(Alignment::End);
        column![
            row![badge(n.etat, 36.0, 18.0), ident, mesure]
                .spacing(12)
                .align_y(Alignment::Center),
            row![
                cible,
                t(format!("contrôlé {}", il_y_a(n.dernier, maintenant)), 13).color(theme::DISCRET)
            ]
            .spacing(18),
        ]
    };
    container(contenu.spacing(10))
        .padding([16, 18])
        .width(Length::Fill)
        .style(move |_: &Theme| theme::boite(fond, Some(bord), 16.0))
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
            .color(theme::DISCRET),
        )
        .padding([40, 0])
        .center_x(Length::Fill)
        .into();
    }
    let axe = row(["il y a 24 h", "18 h", "12 h", "6 h", "maintenant"]
        .into_iter()
        .enumerate()
        .map(|(i, l)| {
            let texte = t(l, 12).color(theme::DISCRET);
            let alignement = match i {
                0 => Alignment::Start,
                4 => Alignment::End,
                _ => Alignment::Center,
            };
            container(texte)
                .width(Length::Fill)
                .align_x(alignement)
                .into()
        }))
    .padding(iced::Padding::ZERO.left(58));
    column![graphe(series, seuil), axe].into()
}

fn carte_90_jours<'a>(detail: Option<&DetailService>) -> Vue<'a> {
    let titre = row![
        t("Disponibilité · 90 jours", 17)
            .font(DEMI_GRAS)
            .width(Length::Fill),
        t(detail.map_or("—".into(), |d| pct(d.dispo90)), 20).font(GRAS),
    ]
    .align_y(Alignment::Center);
    let barres = row(detail
        .map(|d| d.jours.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|&jour| {
            let couleur = match jour {
                Niveau::Warn | Niveau::Lent => theme::WARN,
                Niveau::Down => theme::DOWN,
                Niveau::Vide | Niveau::Inconnu => theme::BORD,
                Niveau::Up => theme::VERT_BARRE,
            };
            container(Space::new())
                .width(Length::Fill)
                .height(32)
                .style(move |_: &Theme| theme::boite(couleur, None, 2.0))
                .into()
        }))
    .spacing(2)
    .height(32);
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
            .color(theme::DISCRET)
            .width(Length::Fill),
        t(avec_incident, 12).color(theme::DISCRET),
        container(t("aujourd'hui", 12).color(theme::DISCRET))
            .width(Length::Fill)
            .align_x(Alignment::End),
    ];
    carte(column![titre, barres, legende].spacing(10)).into()
}

fn evenements<'a>(detail: Option<&DetailService>, maintenant: i64) -> Vue<'a> {
    let mut liste = column![t("Derniers événements", 17).font(DEMI_GRAS)].spacing(10);
    match detail.map(|d| &d.evenements) {
        Some(evenements) if !evenements.is_empty() => {
            let aujourd_hui = format::local(maintenant);
            let mut lignes = column![];
            for (i, inc) in evenements.iter().take(6).enumerate() {
                let resolu = match inc.fin {
                    Some(fin) => format!("résolu après {}", duree((fin - inc.debut) as f64)),
                    None => "en cours".into(),
                };
                let ligne = row![
                    container(mono_discret(
                        format::quand(&format::local(inc.debut), &aujourd_hui),
                        13
                    ))
                    .width(96),
                    marque(if inc.fin.is_some() {
                        Niveau::Up
                    } else {
                        inc.niveau
                    }),
                    container(
                        t(format!("{} · {resolu}", inc.message), 15).wrapping(Wrapping::None)
                    )
                    .clip(true)
                    .width(Length::Fill),
                ]
                .spacing(14)
                .height(44)
                .align_y(Alignment::Center);
                if i > 0 {
                    lignes = lignes.push(separateur());
                }
                lignes = lignes.push(ligne);
            }
            liste = liste.push(lignes);
        }
        _ => liste = liste.push(t("Aucun incident enregistré.", 15).color(theme::DISCRET)),
    }
    carte(defilant(liste).height(Length::Fill))
        .height(Length::Fill)
        .into()
}

pub fn separateur<'a>() -> Vue<'a> {
    container(Space::new().width(Length::Fill).height(1))
        .style(|_: &Theme| theme::boite(theme::BORD, None, 0.0))
        .into()
}

/// Repere d'un incident : rond vert (resolu), anneau rouge, carre orange.
pub fn marque<'a>(niveau: Niveau) -> Vue<'a> {
    commun::point(niveau, 12.0)
}
