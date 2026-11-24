//! Ecran de veille, facon horloge de chevet : fond noir, grande horloge et etat general.
//! Toucher l'ecran ouvre le tableau de bord.

use iced::widget::{Space, button, column, container, row, stack};
use iced::{Alignment, Color, Length, Padding, Theme};

use crate::api::Niveau;

use super::Contexte;
use crate::ecran::Message;
use crate::ecran::commun::{Vue, point_teinte};
use crate::ecran::format::{self, duree, jour_long, majuscule_initiale, pluriel};
use crate::ecran::icones::icone_etat_teinte;
use crate::ecran::logique::{Page, pire, problemes};
use crate::ecran::theme::{self, DEMI_GRAS, LEGER, nuit, t};

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let pbs = problemes(e);
    let (niveau, titre, sous) = match pbs.as_slice() {
        [p] => (
            p.niveau,
            p.court.clone(),
            format!(
                "{}, depuis {}",
                p.detail,
                duree((c.maintenant - p.depuis) as f64)
            ),
        ),
        [] => {
            let nodes: usize = e.services.iter().map(|s| s.nodes.len()).sum();
            let tailscale = if e.tailscale.tablette.ip.is_some() {
                ", Tailscale connecté"
            } else {
                ""
            };
            (
                Niveau::Up,
                "Tout est opérationnel".to_string(),
                format!(
                    "{}, {}{tailscale}",
                    pluriel(e.services.len(), "service"),
                    pluriel(nodes, "node")
                ),
            )
        }
        plusieurs => (
            pire(plusieurs.iter().map(|p| p.niveau)),
            format!("{} problèmes en cours", plusieurs.len()),
            plusieurs
                .iter()
                .map(|p| p.court.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ),
    };
    let couleur_etat = couleur_nuit(niveau);
    let maintenant = format::local(c.maintenant);
    let horloge = column![
        t(format::heure(&maintenant), 176)
            .font(LEGER)
            .line_height(1.0)
            .color(nuit::TEXTE),
        t(majuscule_initiale(&jour_long(&maintenant)), 24).color(nuit::SECONDAIRE),
    ]
    .spacing(12)
    .align_x(Alignment::Center);
    let statut = column![
        row![
            icone_etat_teinte(niveau, 32.0, couleur_etat),
            t(titre, 30).font(DEMI_GRAS).color(if niveau == Niveau::Up {
                nuit::TEXTE
            } else {
                couleur_etat
            })
        ]
        .spacing(14)
        .align_y(Alignment::Center),
        container(t(sous, 18).color(nuit::SECONDAIRE).center()).max_width(1000),
    ]
    .spacing(10)
    .align_x(Alignment::Center);
    let centre = container(
        column![horloge, statut]
            .spacing(48)
            .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .padding(Padding::ZERO.top(40));

    let points = row(e.services.iter().map(|s| {
        let probleme = matches!(s.etat, Niveau::Warn | Niveau::Down);
        row![
            point_teinte(s.etat, 10.0, couleur_nuit(s.etat)),
            t(s.nom.as_str(), 14).color(if probleme {
                couleur_nuit(s.etat)
            } else {
                nuit::SECONDAIRE
            })
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }))
    .spacing(28)
    .wrap()
    .vertical_spacing(14)
    .align_x(Alignment::Center);
    let pied = column![
        container(points).center_x(Length::Fill),
        t("Touche l'écran pour ouvrir le tableau de bord", 14).color(nuit::DISCRET),
    ]
    .spacing(22)
    .align_x(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        right: 56.0,
        bottom: 40.0,
        left: 56.0,
    });

    let mut couches: Vec<Vue<'a>> = vec![
        column![centre, pied]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    ];
    if niveau != Niveau::Up {
        couches.push(
            container(Space::new())
                .width(Length::Fill)
                .height(6)
                .style(move |_: &Theme| theme::boite(couleur_etat, 0.0))
                .into(),
        );
    }
    button(stack(couches))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(theme::touchable(Some(nuit::FOND), 0.0, nuit::TEXTE))
        .on_press(Message::Aller(Page::Accueil))
        .into()
}

/// Teinte d'un etat sur fond noir.
fn couleur_nuit(niveau: Niveau) -> Color {
    match niveau {
        Niveau::Up => nuit::UP,
        Niveau::Warn | Niveau::Lent => nuit::WARN,
        Niveau::Down => nuit::DOWN,
        Niveau::Inconnu | Niveau::Vide => nuit::SECONDAIRE,
    }
}
