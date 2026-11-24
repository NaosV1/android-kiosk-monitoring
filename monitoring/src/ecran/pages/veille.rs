//! Ecran de veille : grande horloge et etat general, lisibles de loin.
//! Toucher l'ecran ouvre le tableau de bord.

use iced::widget::{Space, button, column, container, row, stack};
use iced::{Alignment, Color, Length, Padding, Theme, color};

use crate::api::Niveau;

use super::Contexte;
use crate::ecran::Message;
use crate::ecran::commun::Vue;
use crate::ecran::format::{self, duree, jour_long, majuscule_initiale, pluriel};
use crate::ecran::icones::icone_etat;
use crate::ecran::logique::{Page, pire, problemes};
use crate::ecran::theme::{self, DEMI_GRAS, LEGER, t};

const FOND: Color = color!(0x06080B);
const CLAIR: Color = color!(0xDCE3EC);
const GRIS: Color = color!(0x7D8A9C);
const SOMBRE: Color = color!(0x5E6B7D);

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let pbs = problemes(e);
    let (niveau, titre, sous) = match pbs.as_slice() {
        [p] => (
            p.niveau,
            p.court.clone(),
            format!(
                "{} · depuis {}",
                p.detail,
                duree((c.maintenant - p.depuis) as f64)
            ),
        ),
        [] => {
            let nodes: usize = e.services.iter().map(|s| s.nodes.len()).sum();
            let tailscale = if e.tailscale.tablette.ip.is_some() {
                " · Tailscale connecté"
            } else {
                ""
            };
            (
                Niveau::Up,
                "Tout est opérationnel".to_string(),
                format!(
                    "{} · {}{tailscale}",
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
                .join(" · "),
        ),
    };
    let maintenant = format::local(c.maintenant);
    let horloge = column![
        t(format::heure(&maintenant), 184)
            .font(LEGER)
            .line_height(1.0)
            .color(CLAIR),
        t(majuscule_initiale(&jour_long(&maintenant)), 26).color(GRIS),
    ]
    .spacing(10)
    .align_x(Alignment::Center);
    let couleur_titre = if niveau == Niveau::Up {
        CLAIR
    } else {
        theme::couleur(niveau)
    };
    let statut = column![
        row![
            icone_etat(niveau, 36.0),
            t(titre, 34).font(DEMI_GRAS).color(couleur_titre)
        ]
        .spacing(14)
        .align_y(Alignment::Center),
        container(t(sous, 19).color(GRIS).center()).max_width(1000),
    ]
    .spacing(10)
    .align_x(Alignment::Center);
    let centre = container(
        column![horloge, statut]
            .spacing(44)
            .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .padding(Padding::ZERO.top(40));

    let points = row(e.services.iter().map(|s| {
        let couleur = if matches!(s.etat, Niveau::Warn | Niveau::Down) {
            theme::couleur(s.etat)
        } else {
            GRIS
        };
        row![point_service(s.etat), t(s.nom.as_str(), 14).color(couleur)]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
    }))
    .spacing(30)
    .wrap()
    .vertical_spacing(14)
    .align_x(Alignment::Center);
    let pied = column![
        container(points).center_x(Length::Fill),
        t("Touche l'écran pour ouvrir le tableau de bord", 14).color(SOMBRE),
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
        let barre = theme::couleur(niveau);
        couches.push(
            container(Space::new())
                .width(Length::Fill)
                .height(6)
                .style(move |_: &Theme| theme::boite(barre, None, 0.0))
                .into(),
        );
    }
    button(stack(couches))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(0)
        .style(theme::touchable(Some(FOND), None, 0.0, CLAIR))
        .on_press(Message::Aller(Page::Accueil))
        .into()
}

fn point_service<'a>(etat: Niveau) -> Vue<'a> {
    container(Space::new())
        .width(12)
        .height(12)
        .style(move |_: &Theme| match etat {
            Niveau::Warn | Niveau::Lent => theme::boite(theme::WARN, None, 3.0),
            Niveau::Down => iced::widget::container::Style {
                border: iced::Border {
                    color: theme::DOWN,
                    width: 2.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            },
            Niveau::Inconnu | Niveau::Vide => theme::boite(theme::GRIS_NODE, None, 6.0),
            Niveau::Up => theme::boite(theme::VERT_BARRE, None, 6.0),
        })
        .into()
}
