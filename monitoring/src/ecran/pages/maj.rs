//! Mise a jour : version installee, nouvelle release, confirmation et installation.

use iced::widget::{Space, column, container, mouse_area, opaque, row, span};
use iced::{Alignment, Color, Length, Padding, Theme};

use crate::api::{Niveau, StatutMaj};

use super::Contexte;
use crate::ecran::commun::{
    self, Vue, alerte, bouton, bouton_neutre, bouton_principal, carte, defilant, ecran, entete,
    horloge, pastille, separateur, titres,
};
use crate::ecran::format::{il_y_a, nom_version, nombre};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::{BlocNote, Page, notes_release};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MONO, MOYEN, t, texte_riche};
use crate::ecran::{Action, Message};

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let m = &c.etat.maj;
    let etat = match m.statut {
        StatutMaj::AJour => Some(pastille(Some(Niveau::Up), "À jour")),
        StatutMaj::Prete => Some(pastille(None, "Nouvelle version disponible")),
        StatutMaj::Telechargement => Some(pastille(None, "Téléchargement…")),
        StatutMaj::Erreur => Some(pastille(Some(Niveau::Warn), "Erreur")),
        StatutMaj::Desactive => Some(pastille(Some(Niveau::Inconnu), "Désactivée")),
        StatutMaj::Inconnu => Some(pastille(Some(Niveau::Inconnu), "Pas encore vérifiée")),
        StatutMaj::Installation | StatutMaj::Redemarrage => None,
    };
    let sous: Vue<'a> = if m.depot.is_empty() {
        t(
            "Ajoute une section [mise_a_jour] avec le dépôt GitHub dans config.toml",
            14,
        )
        .color(theme::SECONDAIRE)
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
                ", {verification} · toutes les {} h",
                nombre(m.heures)
            )),
        ])
        .size(14)
        .color(theme::SECONDAIRE)
        .into()
    };
    let mut elements = vec![titres("Mise à jour", etat, commun::sous_titre(sous))];
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
        Some(d) if d.version == m.actuelle => format!(
            "installée {} · avant : {}",
            il_y_a(Some(d.quand), c.maintenant),
            nom_version(&d.depuis)
        ),
        _ if m.actuelle == "dev" => "installée à la main, hors release".into(),
        _ => "version de la release".into(),
    };
    let actuelle = carte(
        column![
            t("Version installée", 14)
                .font(MOYEN)
                .color(theme::SECONDAIRE),
            t(nom_version(&m.actuelle), 34).font(GRAS),
            t(installee, 14).color(theme::SECONDAIRE),
        ]
        .spacing(6),
    );
    let paragraphe = |texte: &'a str| t(texte, 15).color(theme::SECONDAIRE);
    let explication = carte(
        column![
            t("Comment ça marche", 17).font(DEMI_GRAS),
            texte_riche(vec![
                span("À chaque push sur "),
                span("main").font(MONO),
                span(", GitHub vérifie le code puis publie une release."),
            ])
            .size(15)
            .color(theme::SECONDAIRE),
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
                    t("Nouvelle version", 14)
                        .font(MOYEN)
                        .color(theme::SECONDAIRE),
                    t(nom_version(&d.version), 34)
                        .font(GRAS)
                        .color(theme::ACCENT),
                    t(detail, 14).color(theme::SECONDAIRE),
                ]
                .spacing(6)
                .width(Length::Fill),
                bouton_principal(
                    Some(Icone::Maj),
                    "Mettre à jour",
                    prete.then(|| Message::DemanderMaj(d.version.clone())),
                ),
            ]
            .spacing(16)
            .align_y(Alignment::Center);
            carte(column![entete_dispo, separateur(), notes(&d.notes)].spacing(14)).into()
        }
        None => {
            let (dessin, message, detail) = if m.statut == StatutMaj::AJour {
                (
                    icone(Icone::Ok, 40.0, 2.4, theme::ton(Niveau::Up).texte),
                    "Tu as la dernière version.",
                    Some("La prochaine vérification se fera automatiquement."),
                )
            } else if m.depot.is_empty() {
                (
                    icone(Icone::Maj, 40.0, 2.0, theme::SECONDAIRE),
                    "Mises à jour désactivées.",
                    None,
                )
            } else {
                (
                    icone(Icone::Maj, 40.0, 2.0, theme::SECONDAIRE),
                    "Aucune nouvelle version pour le moment.",
                    None,
                )
            };
            let mut contenu = column![dessin, t(message, 19).font(DEMI_GRAS)]
                .spacing(12)
                .align_x(Alignment::Center);
            if let Some(d) = detail {
                contenu = contenu.push(t(d, 15).color(theme::SECONDAIRE));
            }
            container(contenu)
                .padding([40, 20])
                .center_x(Length::Fill)
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
            .spacing(14)
            .width(Length::FillPortion(5)),
        // les notes peuvent etre longues : la colonne defile, la carte suit son contenu
        defilant(droite)
            .width(Length::FillPortion(7))
            .height(Length::Fill),
    ]
    .spacing(20)
    .height(Length::Fill);
    ecran(
        &Page::Maj,
        c.etat,
        c.injoignable_depuis.is_some(),
        page.push(corps),
    )
}

fn notes<'a>(texte: &str) -> Vue<'a> {
    let blocs = notes_release(texte);
    if blocs.is_empty() {
        return t("Pas de notes pour cette version.", 15)
            .color(theme::SECONDAIRE)
            .into();
    }
    let mut colonne = column![].spacing(10);
    for bloc in blocs {
        colonne = colonne.push(match bloc {
            BlocNote::Paragraphe(p) => Vue::from(t(p, 15).font(DEMI_GRAS)),
            BlocNote::Liste(elements) => column(elements.into_iter().map(|e| {
                row![
                    container(
                        container(Space::new().width(5).height(5))
                            .style(|_: &Theme| theme::boite(theme::SECONDAIRE, 2.5))
                    )
                    .padding(Padding::ZERO.top(8)),
                    t(e, 15).width(Length::Fill)
                ]
                .spacing(10)
                .into()
            }))
            .spacing(6)
            .into(),
        });
    }
    colonne.into()
}

/// Feuille de confirmation ; toucher a cote l'annule.
pub fn confirmation<'a>(version: &str, actuelle: &str) -> Vue<'a> {
    let paragraphe = |texte: String| t(texte, 15).color(theme::SECONDAIRE);
    let actions = row![
        Space::new().width(Length::Fill),
        bouton_neutre("Annuler", Message::AnnulerMaj),
        bouton_principal(
            Some(Icone::Maj),
            "Oui, mettre à jour",
            Some(Message::ConfirmerMaj)
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let fenetre = container(
        column![
            t(format!("Installer {} ?", nom_version(version)), 22).font(GRAS),
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
        .spacing(12),
    )
    .width(560)
    .padding([26, 28])
    .style(|_: &Theme| theme::boite(theme::SURFACE, 20.0));
    let fond = container(opaque(fenetre))
        .center(Length::Fill)
        .style(|_: &Theme| theme::boite(Color::from_rgba(0.0, 0.0, 0.0, 0.32), 0.0));
    opaque(mouse_area(fond).on_press(Message::AnnulerMaj))
}

/// Ecran affiche pendant l'installation, jusqu'au redemarrage de l'appli.
pub fn installation<'a>(version: String) -> Vue<'a> {
    let contenu = column![
        container(icone(Icone::Maj, 30.0, 2.2, theme::ACCENT))
            .center(64)
            .style(|_: &Theme| theme::boite(theme::ACCENT_FOND, 32.0)),
        t(format!("Installation de {}…", nom_version(&version)), 26).font(GRAS),
        t("L'appli redémarre, l'écran va se recharger tout seul.", 16).color(theme::SECONDAIRE),
    ]
    .spacing(14)
    .align_x(Alignment::Center);
    opaque(
        container(contenu)
            .center(Length::Fill)
            .style(|_: &Theme| theme::boite(theme::FOND, 0.0)),
    )
}
