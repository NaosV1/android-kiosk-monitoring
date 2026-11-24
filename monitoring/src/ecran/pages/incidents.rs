//! Incidents : en cours (acquittables), resolus sur 30 jours, bilan par service.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Length, Padding, Theme};

use crate::api::{Incident, Incidents, Niveau};

use super::Contexte;
use super::service::{marque, separateur};
use crate::ecran::commun::{
    Vue, au_centre, badge, bouton_plein, defilant, ecran, entete, titre_section, titres, vide,
};
use crate::ecran::format::{self, duree, jour_long, majuscule_initiale, pct};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::Page;
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MOYEN, t};
use crate::ecran::{Action, Filtre, Message};

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let ouverts = &c.etat.incidents_ouverts;
    let resolus: Vec<&Incident> = c.incidents.map_or_else(Vec::new, |d| {
        d.incidents.iter().filter(|i| i.fin.is_some()).collect()
    });
    let sous = format!(
        "30 derniers jours · {} en cours · {} résolus",
        ouverts.len(),
        c.incidents
            .map_or("…".into(), |_| resolus.len().to_string())
    );
    let tete = entete(vec![
        titres("Incidents", None, t(sous, 14).color(theme::DISCRET).into()),
        segments(c.filtre),
    ]);

    let mut gauche = column![].spacing(12);
    if c.filtre != Filtre::Resolus {
        gauche = gauche.push(titre_section("En cours", None));
        if ouverts.is_empty() {
            gauche = gauche.push(vide(t("Aucun incident en cours.", 16)));
        }
        for i in ouverts {
            gauche = gauche.push(carte_ouvert(i, c.maintenant));
        }
    }
    if c.filtre != Filtre::EnCours {
        gauche =
            gauche.push(container(titre_section("Résolus", None)).padding(Padding::ZERO.top(8)));
        gauche = gauche.push(match c.incidents {
            None => t("Chargement…", 16).color(theme::DISCRET).into(),
            Some(_) if resolus.is_empty() => {
                vide(t("Aucun incident résolu sur les 30 derniers jours.", 16))
            }
            Some(_) => liste_resolus(&resolus, c.maintenant),
        });
    }
    let droite: Vue<'a> = match c.incidents {
        None => container(t("Chargement…", 16).color(theme::DISCRET))
            .padding([16, 20])
            .width(Length::Fill)
            .style(theme::carte)
            .into(),
        Some(d) => bilan(d),
    };
    let corps = row![
        defilant(gauche)
            .height(Length::Fill)
            .width(Length::FillPortion(8)),
        container(droite)
            .width(Length::FillPortion(4))
            .height(Length::Fill),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(&Page::Incidents, c.etat, column![tete, corps].spacing(18))
}

fn segments<'a>(actif: Filtre) -> Vue<'a> {
    let segment = |filtre: Filtre, libelle: &'a str| -> Vue<'a> {
        let choisi = filtre == actif;
        button(au_centre(t(libelle, 16).font(DEMI_GRAS)))
            .height(48)
            .padding([0, 22])
            .style(theme::touchable(
                choisi.then_some(theme::ACTIF),
                None,
                14.0,
                if choisi { theme::TEXTE } else { theme::DISCRET },
            ))
            .on_press(Message::Filtrer(filtre))
            .into()
    };
    container(
        row![
            segment(Filtre::Tous, "Tous"),
            segment(Filtre::EnCours, "En cours"),
            segment(Filtre::Resolus, "Résolus")
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding(4)
    .style(|_: &Theme| theme::boite(theme::SURFACE, Some(theme::BORD2), 18.0))
    .into()
}

fn carte_ouvert<'a>(i: &Incident, maintenant: i64) -> Vue<'a> {
    let hs = i.niveau == Niveau::Down;
    let (fond, bord, clair) = if hs {
        (theme::DOWN_CARTE, theme::DOWN_BORD, theme::DOWN_CLAIR)
    } else {
        (theme::WARN_CARTE, theme::WARN_BORD, theme::WARN_CLAIR)
    };
    let acquitte = i.acquitte != 0;
    let textes = column![
        t(
            format!("{} · {}", i.nom, if hs { "hors ligne" } else { "dégradé" }),
            20
        )
        .font(GRAS),
        t(
            format!(
                "Depuis {} ({})",
                format::heure(&format::local(i.debut)),
                duree((maintenant - i.debut) as f64)
            ),
            15
        )
        .color(clair),
        t(
            format!("{}{}", i.message, if acquitte { " · acquitté" } else { "" }),
            14
        )
        .color(theme::TEXTE2),
    ]
    .spacing(4)
    .width(Length::Fill);
    let haut = row![badge(i.niveau, 48.0, 26.0), textes].spacing(16);
    let mut actions = row![bouton_plein(
        theme::couleur(i.niveau),
        "Voir",
        Icone::Droite,
        false,
        52.0,
        Some(Message::Aller(Page::de_incident(i)))
    )]
    .spacing(12);
    if !acquitte {
        let acquitter = button(au_centre(
            row![
                icone(Icone::Ok, 20.0, 2.2, clair),
                t("Acquitter", 16).font(DEMI_GRAS)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ))
        .height(52)
        .padding([0, 20])
        .style(theme::touchable(None, Some(bord), 14.0, clair))
        .on_press(Message::Agir(Action::Acquittement, i.id.to_string()));
        actions = actions.push(acquitter);
    }
    container(column![haut, container(actions).padding(Padding::ZERO.left(64))].spacing(14))
        .padding([18, 20])
        .width(Length::Fill)
        .style(move |_: &Theme| theme::boite(fond, Some(bord), 18.0))
        .into()
}

fn liste_resolus<'a>(resolus: &[&Incident], maintenant: i64) -> Vue<'a> {
    let aujourd_hui = format::local(maintenant);
    let hier = aujourd_hui - chrono::Duration::days(1);
    let mut liste = column![];
    let mut jour_courant = String::new();
    for (k, i) in resolus.iter().enumerate() {
        let debut = format::local(i.debut);
        let libelle = if debut.date_naive() == aujourd_hui.date_naive() {
            "aujourd'hui".to_string()
        } else if debut.date_naive() == hier.date_naive() {
            format!("hier · {}", jour_long(&debut))
        } else {
            jour_long(&debut)
        };
        if libelle != jour_courant {
            liste = liste.push(
                container(
                    t(majuscule_initiale(&libelle), 13)
                        .font(MOYEN)
                        .color(theme::DISCRET),
                )
                .padding(Padding {
                    top: 12.0,
                    bottom: 4.0,
                    left: 0.0,
                    right: 0.0,
                }),
            );
            jour_courant = libelle;
        } else if k > 0 {
            liste = liste.push(separateur());
        }
        let fin = i.fin.unwrap_or(maintenant);
        let ligne = row![
            marque(i.niveau),
            container(
                t(i.nom.clone(), 15)
                    .font(DEMI_GRAS)
                    .wrapping(Wrapping::None)
            )
            .clip(true)
            .width(128),
            container(
                t(i.message.clone(), 15)
                    .color(theme::TEXTE2)
                    .wrapping(Wrapping::None)
            )
            .clip(true)
            .width(Length::Fill),
            t(
                format!(
                    "{} → {}",
                    format::heure(&debut),
                    format::heure(&format::local(fin))
                ),
                13
            )
            .font(theme::MONO)
            .color(theme::DISCRET),
            container(t(duree((fin - i.debut) as f64), 15).font(DEMI_GRAS))
                .width(64)
                .align_x(Alignment::End),
        ]
        .spacing(14)
        .height(50)
        .align_y(Alignment::Center);
        liste = liste.push(ligne);
    }
    container(liste)
        .padding(Padding {
            top: 6.0,
            right: 20.0,
            bottom: 10.0,
            left: 20.0,
        })
        .width(Length::Fill)
        .style(theme::carte)
        .into()
}

fn bilan<'a>(d: &Incidents) -> Vue<'a> {
    let b = &d.bilan;
    let mini = |valeur: String, libelle: &'a str| -> Vue<'a> {
        container(
            column![
                t(valeur, 24).font(GRAS),
                t(libelle, 13).color(theme::DISCRET)
            ]
            .spacing(2),
        )
        .padding([12, 14])
        .width(Length::Fill)
        .style(|_: &Theme| theme::boite(theme::CREUX, None, 12.0))
        .into()
    };
    let resume = container(
        column![
            t("Bilan · 30 jours", 17).font(DEMI_GRAS),
            column![
                t(pct(b.dispo), 44).font(GRAS),
                t("disponibilité moyenne des services", 14).color(theme::DISCRET)
            ],
            row![
                mini(
                    b.nombre.to_string(),
                    if b.nombre > 1 {
                        "incidents"
                    } else {
                        "incident"
                    }
                ),
                mini(
                    if b.duree > 0 {
                        duree(b.duree as f64)
                    } else {
                        "0".into()
                    },
                    "de perturbation"
                ),
            ]
            .spacing(12),
        ]
        .spacing(14),
    )
    .padding([16, 20])
    .width(Length::Fill)
    .style(theme::carte);

    let max = b
        .par_service
        .iter()
        .map(|x| x.duree)
        .max()
        .unwrap_or(0)
        .max(1);
    let mut par_service = column![t("Par service", 17).font(DEMI_GRAS)].spacing(14);
    if b.par_service.is_empty() {
        par_service = par_service.push(t("Rien à signaler.", 15).color(theme::DISCRET));
    }
    for x in &b.par_service {
        let couleur = if x.en_cours {
            theme::WARN
        } else {
            theme::DISCRET
        };
        let lib = row![
            container(t(x.nom.clone(), 14).wrapping(Wrapping::None))
                .clip(true)
                .width(Length::Fill),
            t(
                format!(
                    "{}{}",
                    duree(x.duree as f64),
                    if x.en_cours { ", en cours" } else { "" }
                ),
                14
            )
            .color(couleur),
        ]
        .spacing(10);
        let part = ((x.duree as f64 / max as f64) * 1000.0)
            .round()
            .clamp(4.0, 1000.0) as u16;
        let barre = if x.en_cours {
            theme::WARN
        } else {
            iced::color!(0x4C86C6)
        };
        let piste = row![
            container(Space::new())
                .width(Length::FillPortion(part))
                .height(10)
                .style(move |_: &Theme| theme::boite(barre, None, 5.0)),
            Space::new().width(Length::FillPortion(1000 - part.min(999))),
        ];
        let piste = container(piste)
            .width(Length::Fill)
            .style(|_: &Theme| theme::boite(theme::CREUX, None, 5.0));
        par_service = par_service.push(column![lib, piste].spacing(6));
    }
    let par_service = container(defilant(par_service).height(Length::Fill))
        .padding([16, 20])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::carte);
    column![resume, par_service]
        .spacing(12)
        .height(Length::Fill)
        .into()
}
