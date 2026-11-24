//! Incidents : en cours (acquittables), resolus sur 30 jours, bilan par service.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Length, Padding, Theme};

use crate::api::{Incident, Incidents, Niveau};

use super::Contexte;
use crate::ecran::commun::{
    self, CIBLE, Vue, au_centre, carte, defilant, ecran, entete, horloge, point, separateur,
    titre_section, titres, vide,
};
use crate::ecran::format::{self, duree, jour_long, majuscule_initiale, pct};
use crate::ecran::icones::{Icone, icone, icone_etat};
use crate::ecran::logique::Page;
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MOYEN, t};
use crate::ecran::{Action, Filtre, Message};

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let ouverts = &c.etat.incidents_ouverts;
    let resolus: Vec<&Incident> = c.incidents.map_or_else(Vec::new, |d| {
        d.incidents.iter().filter(|i| i.fin.is_some()).collect()
    });
    let sous = format!(
        "{} en cours, {} résolus sur les 30 derniers jours",
        ouverts.len(),
        c.incidents
            .map_or("…".into(), |_| resolus.len().to_string())
    );
    let tete = entete(vec![
        titres("Incidents", None, commun::ligne_secondaire(sous, 14.0)),
        segments(c.filtre),
        horloge(),
    ]);

    let mut gauche = column![].spacing(10);
    if c.filtre != Filtre::Resolus {
        gauche = gauche.push(titre_section("En cours", None));
        if ouverts.is_empty() {
            gauche = gauche.push(vide(t("Aucun incident en cours.", 15)));
        }
        for i in ouverts {
            gauche = gauche.push(carte_ouvert(i, c.maintenant));
        }
    }
    if c.filtre != Filtre::EnCours {
        gauche =
            gauche.push(container(titre_section("Résolus", None)).padding(Padding::ZERO.top(12)));
        gauche = gauche.push(match c.incidents {
            None => t("Chargement…", 15).color(theme::SECONDAIRE).into(),
            Some(_) if resolus.is_empty() => {
                vide(t("Aucun incident résolu sur les 30 derniers jours.", 15))
            }
            Some(_) => liste_resolus(&resolus, c.maintenant),
        });
    }
    let droite: Vue<'a> = match c.incidents {
        None => carte(t("Chargement…", 15).color(theme::SECONDAIRE)).into(),
        Some(d) => bilan(d, ouverts),
    };
    let corps = row![
        defilant(gauche.padding(Padding::ZERO.bottom(8)))
            .height(Length::Fill)
            .width(Length::FillPortion(8)),
        container(droite)
            .width(Length::FillPortion(4))
            .height(Length::Fill),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(
        &Page::Incidents,
        c.etat,
        c.injoignable_depuis.is_some(),
        column![tete, corps].spacing(20),
    )
}

/// Controle segmente : Tous, En cours, Resolus.
fn segments<'a>(actif: Filtre) -> Vue<'a> {
    let segment = |filtre: Filtre, libelle: &'a str| -> Vue<'a> {
        let choisi = filtre == actif;
        button(
            container(t(libelle, 15).font(if choisi { DEMI_GRAS } else { MOYEN }))
                .center(Length::Fill),
        )
        .width(112)
        .height(CIBLE)
        .padding(0)
        .style(theme::touchable(
            choisi.then_some(theme::SURFACE),
            10.0,
            if choisi {
                theme::TEXTE
            } else {
                theme::SECONDAIRE
            },
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
        .spacing(2),
    )
    .padding(3)
    .style(|_: &Theme| theme::boite(theme::PISTE, 13.0))
    .into()
}

fn bouton_carte<'a>(contenu: Vue<'a>, couleur: iced::Color, message: Message) -> Vue<'a> {
    button(au_centre(contenu))
        .height(CIBLE)
        .padding([0, 16])
        .style(theme::touchable(Some(theme::SURFACE), 12.0, couleur))
        .on_press(message)
        .into()
}

fn carte_ouvert<'a>(i: &Incident, maintenant: i64) -> Vue<'a> {
    let ton = theme::ton(i.niveau);
    let hs = i.niveau == Niveau::Down;
    let acquitte = i.acquitte != 0;
    let textes = column![
        t(
            format!("{} {}", i.nom, if hs { "hors ligne" } else { "dégradé" }),
            18
        )
        .font(GRAS)
        .color(ton.texte),
        t(
            format!(
                "Depuis {} ({}){}",
                format::heure(&format::local(i.debut)),
                duree((maintenant - i.debut) as f64),
                if acquitte { ", acquitté" } else { "" }
            ),
            14
        )
        .color(ton.texte_fond),
        t(i.message.clone(), 14).color(theme::TEXTE),
    ]
    .spacing(3)
    .width(Length::Fill);
    let rond = container(icone_etat(i.niveau, 22.0))
        .center(40)
        .style(|_: &Theme| theme::boite(theme::SURFACE, 20.0));
    let mut actions = row![bouton_carte(
        row![
            t("Voir", 15).font(DEMI_GRAS),
            icone(Icone::Droite, 18.0, 2.2, theme::TEXTE)
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into(),
        theme::TEXTE,
        Message::Aller(Page::de_incident(i)),
    )]
    .spacing(10);
    if !acquitte {
        actions = actions.push(bouton_carte(
            row![
                icone(Icone::Ok, 18.0, 2.4, ton.texte),
                t("Acquitter", 15).font(DEMI_GRAS)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
            ton.texte,
            Message::Agir(Action::Acquittement, i.id.to_string()),
        ));
    }
    container(
        column![
            row![rond, textes].spacing(14),
            container(actions).padding(Padding::ZERO.left(54)),
        ]
        .spacing(12),
    )
    .padding([16, 18])
    .width(Length::Fill)
    .style(move |_: &Theme| theme::boite(ton.fond, 16.0))
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
            format!("hier, {}", jour_long(&debut))
        } else {
            jour_long(&debut)
        };
        if libelle != jour_courant {
            liste = liste.push(
                container(
                    t(majuscule_initiale(&libelle), 13)
                        .font(DEMI_GRAS)
                        .color(theme::SECONDAIRE),
                )
                .padding(Padding::ZERO.top(if k == 0 { 4.0 } else { 14.0 }).bottom(2)),
            );
            jour_courant = libelle;
        } else {
            liste = liste.push(separateur());
        }
        let fin = i.fin.unwrap_or(maintenant);
        let ligne = row![
            point(i.niveau, 10.0),
            container(
                t(i.nom.clone(), 15)
                    .font(DEMI_GRAS)
                    .wrapping(Wrapping::None)
            )
            .clip(true)
            .width(140),
            container(
                t(i.message.clone(), 15)
                    .color(theme::SECONDAIRE)
                    .wrapping(Wrapping::None)
            )
            .clip(true)
            .width(Length::Fill),
            commun::mono(
                format!(
                    "{} → {}",
                    format::heure(&debut),
                    format::heure(&format::local(fin))
                ),
                13.0,
                theme::SECONDAIRE
            ),
            container(t(duree((fin - i.debut) as f64), 15).font(DEMI_GRAS))
                .width(70)
                .align_x(Alignment::End),
        ]
        .spacing(12)
        .height(48)
        .align_y(Alignment::Center);
        liste = liste.push(ligne);
    }
    container(liste)
        .padding(Padding {
            top: 8.0,
            right: 20.0,
            bottom: 8.0,
            left: 20.0,
        })
        .width(Length::Fill)
        .style(theme::carte)
        .into()
}

/// Disponibilite moyenne, nombre et duree des incidents, puis les services les plus perturbes.
fn bilan<'a>(d: &Incidents, ouverts: &[Incident]) -> Vue<'a> {
    let b = &d.bilan;
    let chiffre = |valeur: String, libelle: &'a str| -> Vue<'a> {
        column![
            t(valeur, 22).font(GRAS),
            t(libelle, 13).color(theme::SECONDAIRE)
        ]
        .spacing(2)
        .width(Length::Fill)
        .into()
    };
    let filet_vertical = container(Space::new().width(1).height(40))
        .style(|_: &Theme| theme::boite(theme::SEPARATEUR, 0.0));
    let resume = carte(
        column![
            t("Bilan des 30 derniers jours", 17).font(DEMI_GRAS),
            column![
                t(pct(b.dispo), 40).font(GRAS),
                t("disponibilité moyenne des services", 14).color(theme::SECONDAIRE)
            ]
            .spacing(2),
            separateur(),
            row![
                chiffre(
                    b.nombre.to_string(),
                    if b.nombre > 1 {
                        "incidents"
                    } else {
                        "incident"
                    }
                ),
                filet_vertical,
                container(chiffre(
                    if b.duree > 0 {
                        duree(b.duree as f64)
                    } else {
                        "0".into()
                    },
                    "de perturbation"
                ))
                .padding(iced::Padding::ZERO.left(16))
                .width(Length::Fill),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(14),
    );

    let max = b
        .par_service
        .iter()
        .map(|x| x.duree)
        .max()
        .unwrap_or(0)
        .max(1);
    let mut par_service = column![t("Par service", 17).font(DEMI_GRAS)].spacing(14);
    if b.par_service.is_empty() {
        par_service = par_service.push(t("Rien à signaler.", 15).color(theme::SECONDAIRE));
    }
    for x in &b.par_service {
        // en cours : la teinte du niveau de l'incident ouvert ; resolu : neutre
        let niveau_en_cours = x.en_cours.then(|| {
            ouverts
                .iter()
                .find(|i| i.service == x.service)
                .map_or(Niveau::Warn, |i| i.niveau)
        });
        let (couleur_texte, couleur_barre) = match niveau_en_cours {
            Some(n) => (theme::ton(n).texte, theme::ton(n).plein),
            None => (theme::SECONDAIRE, theme::TERTIAIRE),
        };
        let lib = row![
            container(t(x.nom.clone(), 14).font(MOYEN).wrapping(Wrapping::None))
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
            .color(couleur_texte),
        ]
        .spacing(10);
        let part = ((x.duree as f64 / max as f64) * 1000.0)
            .round()
            .clamp(4.0, 1000.0) as u16;
        let piste = container(row![
            container(Space::new())
                .width(Length::FillPortion(part))
                .height(8)
                .style(move |_: &Theme| theme::boite(couleur_barre, 4.0)),
            Space::new().width(Length::FillPortion(1000 - part.min(999))),
        ])
        .width(Length::Fill)
        .style(|_: &Theme| theme::boite(theme::PISTE, 4.0));
        par_service = par_service.push(column![lib, piste].spacing(6));
    }
    defilant(column![resume, carte(par_service)].spacing(14))
        .height(Length::Fill)
        .into()
}
