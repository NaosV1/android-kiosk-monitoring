//! Mise a jour : version installee, nouvelle release, confirmation et installation.

use iced::widget::{Space, button, column, container, mouse_area, opaque, row, span};
use iced::{Alignment, Color, Length, Padding, Theme};

use crate::api::{Niveau, StatutMaj};

use super::Contexte;
use crate::ecran::commun::{
    self, Vue, alerte, bouton, bouton_plein, defilant, ecran, entete, horloge, titres,
};
use crate::ecran::format::{il_y_a, nom_version, nombre};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::{BlocNote, Page, notes_release};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MONO, t, texte_riche};
use crate::ecran::{Action, Message};

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let m = &c.etat.maj;
    let etiquette = match m.statut {
        StatutMaj::AJour => Some(commun::etiquette_texte(Some(Niveau::Up), true, "À jour")),
        StatutMaj::Prete => Some(commun::etiquette_texte(
            None,
            false,
            "Nouvelle version disponible",
        )),
        StatutMaj::Telechargement => Some(commun::etiquette_texte(None, false, "Téléchargement…")),
        StatutMaj::Erreur => Some(commun::etiquette_texte(Some(Niveau::Warn), false, "Erreur")),
        StatutMaj::Desactive => Some(commun::etiquette_texte(
            Some(Niveau::Inconnu),
            false,
            "Désactivée",
        )),
        StatutMaj::Inconnu => Some(commun::etiquette_texte(
            Some(Niveau::Inconnu),
            false,
            "Pas encore vérifiée",
        )),
        StatutMaj::Installation | StatutMaj::Redemarrage => None,
    };
    let sous: Vue<'a> = if m.depot.is_empty() {
        t(
            "Ajoute une section [mise_a_jour] avec le dépôt GitHub dans config.toml",
            14,
        )
        .color(theme::DISCRET)
        .into()
    } else {
        let verification = if m.en_cours {
            "vérification en cours…".to_string()
        } else if m.verifie.is_some() {
            format!("vérifiée {}", il_y_a(m.verifie, c.maintenant))
        } else {
            "pas encore vérifiée".into()
        };
        texte_riche(vec![
            span("Dépôt "),
            span(m.depot.clone()).font(MONO),
            span(format!(
                " · {verification} · toutes les {} h",
                nombre(m.heures)
            )),
        ])
        .size(14)
        .color(theme::DISCRET)
        .into()
    };
    let mut elements = vec![titres("Mise à jour", etiquette, commun::sous_titre(sous))];
    if !m.depot.is_empty() {
        let (libelle, message) = if m.en_cours {
            ("Vérification…", None)
        } else {
            (
                "Vérifier maintenant",
                Some(Message::Agir(Action::Verification, String::new())),
            )
        };
        elements.push(bouton(Some(Icone::Relancer), libelle, message));
    }
    elements.push(horloge());

    let installee = match &m.derniere_installation {
        Some(d) if d.version == m.actuelle => {
            format!(
                "installée {} · avant : {}",
                il_y_a(Some(d.quand), c.maintenant),
                nom_version(&d.depuis)
            )
        }
        _ if m.actuelle == "dev" => "installée à la main, hors release".into(),
        _ => "version de la release".into(),
    };
    let actuelle = carte(
        column![
            t("Version installée", 14).color(theme::DISCRET),
            t(nom_version(&m.actuelle), 34).font(GRAS),
            t(installee, 15).color(theme::DISCRET),
        ]
        .spacing(10),
    );
    let paragraphe = |texte: &'a str| t(texte, 15).color(theme::TEXTE2);
    let explication = carte(
        column![
            t("Comment ça marche", 17).font(DEMI_GRAS),
            texte_riche(vec![
                span("À chaque push sur "),
                span("main").font(MONO),
                span(", GitHub vérifie le code puis publie une release."),
            ])
            .size(15)
            .color(theme::TEXTE2),
            paragraphe("La tablette la télécharge et contrôle son empreinte, mais n'installe rien sans ton accord."),
            paragraphe(
                "Ta configuration et l'historique sont conservés. Si la nouvelle version ne démarre pas, \
                 la tablette revient seule à la précédente."
            ),
        ]
        .spacing(10),
    );

    let droite: Vue<'a> = match &m.disponible {
        Some(d) => {
            let prete = m.statut == StatutMaj::Prete;
            let mut detail = d
                .publiee
                .map(|p| format!("publiée {}", il_y_a(Some(p), c.maintenant)))
                .unwrap_or_default();
            detail.push_str(if prete {
                " · téléchargée et vérifiée (SHA-256)"
            } else {
                " · téléchargement…"
            });
            let entete_dispo = row![
                column![
                    t("Nouvelle version", 14).color(theme::DISCRET),
                    t(nom_version(&d.version), 34)
                        .font(GRAS)
                        .color(theme::ACCENT),
                    t(detail, 14).color(theme::DISCRET),
                ]
                .spacing(4)
                .width(Length::Fill),
                bouton_plein(
                    theme::ACCENT,
                    "Mettre à jour",
                    Icone::Maj,
                    true,
                    52.0,
                    prete.then(|| Message::DemanderMaj(d.version.clone())),
                ),
            ]
            .spacing(16)
            .align_y(Alignment::Center);
            let trait_ = container(Space::new().width(Length::Fill).height(1))
                .style(|_: &Theme| theme::boite(theme::BORD, None, 0.0));
            container(
                column![
                    entete_dispo,
                    trait_,
                    defilant(notes(&d.notes)).height(Length::Fill)
                ]
                .spacing(12),
            )
            .padding([16, 20])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| theme::boite(theme::SURFACE, Some(theme::INFO_BORD), 16.0))
            .into()
        }
        None => {
            let (dessin, message, detail) = if m.statut == StatutMaj::AJour {
                (
                    icone(Icone::Ok, 40.0, 2.4, theme::UP),
                    "Tu as la dernière version.",
                    Some("La prochaine vérification se fera automatiquement."),
                )
            } else if m.depot.is_empty() {
                (
                    icone(Icone::Maj, 40.0, 2.0, theme::DISCRET),
                    "Mises à jour désactivées.",
                    None,
                )
            } else {
                (
                    icone(Icone::Maj, 40.0, 2.0, theme::DISCRET),
                    "Aucune nouvelle version pour le moment.",
                    None,
                )
            };
            let mut contenu = column![dessin, t(message, 20).font(DEMI_GRAS)]
                .spacing(12)
                .align_x(Alignment::Center);
            if let Some(d) = detail {
                contenu = contenu.push(t(d, 15).color(theme::DISCRET));
            }
            container(contenu)
                .center(Length::Fill)
                .style(theme::carte)
                .into()
        }
    };

    let mut page = column![entete(elements)].spacing(18);
    if let Some(erreur) = &m.erreur {
        page = page.push(alerte(erreur));
    }
    let corps = row![
        column![actuelle, explication]
            .spacing(12)
            .width(Length::FillPortion(5)),
        container(droite)
            .width(Length::FillPortion(7))
            .height(Length::Fill),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(&Page::Maj, c.etat, page.push(corps))
}

fn carte<'a>(contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    container(contenu)
        .padding([16, 20])
        .width(Length::Fill)
        .style(theme::carte)
        .into()
}

fn notes<'a>(texte: &str) -> Vue<'a> {
    let blocs = notes_release(texte);
    if blocs.is_empty() {
        return t("Pas de notes pour cette version.", 15)
            .color(theme::DISCRET)
            .into();
    }
    let mut colonne = column![].spacing(8);
    for bloc in blocs {
        colonne = colonne.push(match bloc {
            BlocNote::Paragraphe(p) => Vue::from(t(p, 15).color(theme::TEXTE2)),
            BlocNote::Liste(elements) => column(elements.into_iter().map(|e| {
                row![
                    t("•", 15).color(theme::TEXTE2),
                    t(e, 15).color(theme::TEXTE2).width(Length::Fill)
                ]
                .spacing(10)
                .into()
            }))
            .spacing(4)
            .padding(Padding::ZERO.left(6))
            .into(),
        });
    }
    colonne.into()
}

/// Fenetre de confirmation ; toucher a cote l'annule.
pub fn confirmation<'a>(version: &str, actuelle: &str) -> Vue<'a> {
    let paragraphe = |texte: String| t(texte, 16).color(theme::TEXTE2);
    let actions = row![
        Space::new().width(Length::Fill),
        button(commun::au_centre(t("Annuler", 16).font(DEMI_GRAS)))
            .height(52)
            .padding([0, 20])
            .style(theme::touchable(
                Some(theme::SURFACE),
                Some(theme::BORD2),
                14.0,
                theme::TEXTE
            ))
            .on_press(Message::AnnulerMaj),
        bouton_plein(
            theme::ACCENT,
            "Oui, mettre à jour",
            Icone::Maj,
            true,
            52.0,
            Some(Message::ConfirmerMaj)
        ),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let fenetre = container(
        column![
            t(format!("Installer {} ?", nom_version(version)), 24).font(GRAS),
            paragraphe(
                "L'appli va redémarrer, ce qui prend une vingtaine de secondes. \
                 Ta configuration et l'historique sont conservés."
                    .to_string()
            ),
            paragraphe(format!(
                "Si la nouvelle version ne démarre pas, la tablette revient automatiquement à {}.",
                nom_version(actuelle)
            )),
            container(actions).padding(Padding::ZERO.top(10)),
        ]
        .spacing(14),
    )
    .width(600)
    .padding([28, 30])
    .style(|_: &Theme| theme::boite(theme::SURFACE, Some(theme::BORD2), 20.0));
    let fond = container(opaque(fenetre))
        .center(Length::Fill)
        .style(|_: &Theme| theme::boite(Color::from_rgba8(4, 6, 9, 0.78), None, 0.0));
    opaque(mouse_area(fond).on_press(Message::AnnulerMaj))
}

/// Ecran affiche pendant l'installation, jusqu'au redemarrage de l'appli.
pub fn installation<'a>(version: String) -> Vue<'a> {
    let contenu = column![
        icone(Icone::Maj, 54.0, 2.2, theme::ACCENT),
        t(format!("Installation de {}…", nom_version(&version)), 28).font(GRAS),
        t("L'appli redémarre, l'écran va se recharger tout seul.", 17).color(theme::DISCRET),
    ]
    .spacing(16)
    .align_x(Alignment::Center);
    opaque(
        container(contenu)
            .center(Length::Fill)
            .style(|_: &Theme| theme::boite(Color::from_rgba8(11, 15, 20, 0.96), None, 0.0)),
    )
}
