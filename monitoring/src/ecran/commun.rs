//! Elements partages par les pages : rail de navigation, horloge, pilule Tailscale,
//! etiquettes d'etat, boutons, messages ephemeres.

use iced::widget::{Space, button, column, container, row, scrollable, stack, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};

use crate::api::{Etat, Niveau, StatutMaj};

use super::Message;
use super::format::{self, duree, jour_long};
use super::icones::{Icone, icone, icone_etat};
use super::logique::{Page, pire};
use super::theme::{self, DEMI_GRAS, GRAS, MONO, t};

pub type Vue<'a> = Element<'a, Message>;

/// Ecran d'attente avant le premier etat du serveur.
pub fn demarrage<'a>(erreur: bool) -> Vue<'a> {
    let message = if erreur {
        "Serveur de monitoring injoignable, nouvelle tentative…"
    } else {
        "Démarrage du monitoring…"
    };
    container(t(message, 22).color(theme::DISCRET))
        .center(Length::Fill)
        .into()
}

/// Contenu d'un bouton de hauteur fixe, centre verticalement.
pub fn au_centre<'a>(contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    container(contenu).center_y(Length::Fill).into()
}

/// Squelette d'une page : rail a gauche, contenu a droite.
pub fn ecran<'a>(page: &Page, etat: &'a Etat, contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    row![
        rail(page, etat),
        container(contenu)
            .padding([24, 28])
            .width(Length::Fill)
            .height(Length::Fill)
    ]
    .into()
}

fn rail<'a>(page: &Page, etat: &'a Etat) -> Vue<'a> {
    let actif = match page {
        Page::Accueil | Page::Service(_) => Some(Page::Accueil),
        Page::Tailscale | Page::Incidents | Page::Maj => Some(page.clone()),
        Page::Veille => None,
    };
    let lien = |cible: Page,
                libelle: &'a str,
                forme: Icone,
                pastille: Option<(String, Color)>|
     -> Vue<'a> {
        let est_actif = actif.as_ref() == Some(&cible);
        let couleur = if est_actif {
            theme::TEXTE
        } else {
            theme::DISCRET
        };
        let corps = column![
            icone(forme, 26.0, 1.8, couleur),
            t(libelle, 13).font(DEMI_GRAS)
        ]
        .spacing(6)
        .align_x(Alignment::Center);
        let bouton = button(container(corps).center(Length::Fill))
            .width(80)
            .height(76)
            .padding(0)
            .style(theme::touchable(
                est_actif.then_some(theme::ACTIF),
                None,
                18.0,
                couleur,
            ))
            .on_press(Message::Aller(cible));
        match pastille {
            None => bouton.into(),
            Some((nombre, fond)) => stack![
                bouton,
                container(compteur(nombre, fond))
                    .width(80)
                    .align_x(Alignment::End)
                    .padding(Padding {
                        top: 6.0,
                        right: 10.0,
                        bottom: 0.0,
                        left: 0.0
                    }),
            ]
            .into(),
        }
    };
    let ouverts = &etat.incidents_ouverts;
    let pastille_incidents = (!ouverts.is_empty()).then(|| {
        (
            ouverts.len().to_string(),
            theme::couleur(pire(ouverts.iter().map(|i| i.niveau))),
        )
    });
    let m = &etat.maj;
    let pastille_maj = (m.disponible.is_some()
        && matches!(m.statut, StatutMaj::Prete | StatutMaj::Telechargement))
    .then(|| ("1".to_string(), theme::ACCENT));
    let logo = container(icone(Icone::Pouls, 26.0, 2.0, theme::ACCENT))
        .center(48)
        .style(|_: &Theme| theme::boite(theme::LOGO, None, 14.0));
    let contenu = column![
        logo,
        Space::new().height(14),
        lien(Page::Accueil, "Accueil", Icone::Accueil, None),
        lien(Page::Tailscale, "Tailscale", Icone::Tailscale, None),
        lien(
            Page::Incidents,
            "Incidents",
            Icone::Cloche,
            pastille_incidents
        ),
        lien(Page::Maj, "Mise à jour", Icone::Maj, pastille_maj),
        Space::new().height(Length::Fill),
        lien(Page::Veille, "Veille", Icone::Lune, None),
    ]
    .spacing(10)
    .align_x(Alignment::Center);
    let bordure = container(Space::new().width(1).height(Length::Fill))
        .style(|_: &Theme| theme::boite(theme::RAIL_BORD, None, 0.0));
    row![
        container(contenu)
            .width(103)
            .height(Length::Fill)
            .padding([24, 12])
            .style(|_: &Theme| theme::boite(theme::RAIL, None, 0.0)),
        bordure,
    ]
    .into()
}

/// Pastille numerique (rail).
fn compteur<'a>(nombre: String, fond: Color) -> Vue<'a> {
    container(t(nombre, 12).font(GRAS).color(theme::FOND))
        .center_x(Length::Shrink)
        .padding([3, 7])
        .style(move |_: &Theme| theme::boite(fond, None, 11.0))
        .into()
}

pub fn horloge<'a>() -> Vue<'a> {
    let maintenant = chrono::Local::now();
    column![
        t(format::heure(&maintenant), 34)
            .font(DEMI_GRAS)
            .line_height(1.0),
        t(jour_long(&maintenant), 13).color(theme::DISCRET),
    ]
    .spacing(3)
    .align_x(Alignment::End)
    .width(170)
    .into()
}

/// Titre de page et sa ligne de detail.
pub fn titres<'a>(titre: impl Into<String>, etiquette: Option<Vue<'a>>, sous: Vue<'a>) -> Vue<'a> {
    let mut ligne = row![t(titre.into(), 28).font(GRAS)]
        .spacing(14)
        .align_y(Alignment::Center);
    if let Some(e) = etiquette {
        ligne = ligne.push(e);
    }
    column![ligne, sous].spacing(5).width(Length::Fill).into()
}

/// Ligne de detail sous un titre (une seule ligne, coupee si trop longue).
pub fn sous_titre<'a>(contenu: Vue<'a>) -> Vue<'a> {
    container(contenu).clip(true).width(Length::Fill).into()
}

pub fn entete<'a>(elements: Vec<Vue<'a>>) -> Vue<'a> {
    row(elements)
        .spacing(18)
        .align_y(Alignment::Center)
        .height(Length::Shrink)
        .into()
}

/// Rond, carre ou anneau de couleur indiquant un etat.
pub fn point<'a>(niveau: Niveau, taille: f32) -> Vue<'a> {
    let style = move |_: &Theme| match niveau {
        Niveau::Up => theme::boite(theme::UP, None, taille / 2.0),
        Niveau::Warn | Niveau::Lent => theme::boite(theme::WARN, None, 3.0),
        Niveau::Down => container::Style {
            border: iced::Border {
                color: theme::DOWN,
                width: 2.0,
                radius: (taille / 2.0).into(),
            },
            ..container::Style::default()
        },
        Niveau::Inconnu | Niveau::Vide => theme::boite(theme::DISCRET, None, taille / 2.0),
    };
    container(Space::new())
        .width(taille)
        .height(taille)
        .style(style)
        .into()
}

/// Etiquette arrondie : `contenu` sur fond et bord de l'etat.
pub fn etiquette<'a>(niveau: Option<Niveau>, contenu: Vec<Vue<'a>>) -> Vue<'a> {
    let (fond, bord, couleur) = match niveau {
        Some(Niveau::Up) => (theme::UP_FOND, theme::OK_BORD, theme::UP),
        Some(Niveau::Warn | Niveau::Lent) => (theme::WARN_FOND, theme::WARN_BORD, theme::WARN),
        Some(Niveau::Down) => (theme::DOWN_FOND, theme::DOWN_BORD, theme::DOWN),
        Some(Niveau::Inconnu | Niveau::Vide) => (theme::SURFACE2, theme::BORD2, theme::DISCRET),
        None => (theme::INFO_FOND, theme::INFO_BORD, theme::ACCENT),
    };
    container(row(contenu).spacing(8).align_y(Alignment::Center))
        .height(32)
        .padding([0, 14])
        .center_y(32)
        .style(move |_: &Theme| container::Style {
            text_color: Some(couleur),
            ..theme::boite(fond, Some(bord), 16.0)
        })
        .into()
}

/// Etiquette texte (ex. « À jour », « Erreur ») ; `None` = bleu d'information.
pub fn etiquette_texte<'a>(niveau: Option<Niveau>, avec_point: bool, libelle: &'a str) -> Vue<'a> {
    let mut contenu = Vec::new();
    if let (true, Some(n)) = (avec_point, niveau) {
        contenu.push(point(n, 12.0));
    }
    contenu.push(t(libelle, 14).font(DEMI_GRAS).into());
    etiquette(niveau, contenu)
}

/// Bouton secondaire (fond de surface) avec icone.
pub fn bouton<'a>(forme: Option<Icone>, libelle: &'a str, message: Option<Message>) -> Vue<'a> {
    let mut contenu = row![].spacing(10).align_y(Alignment::Center);
    if let Some(f) = forme {
        contenu = contenu.push(icone(f, 22.0, 2.0, theme::TEXTE));
    }
    button(au_centre(contenu.push(t(libelle, 16).font(DEMI_GRAS))))
        .height(56)
        .padding([0, 20])
        .style(theme::touchable(
            Some(theme::SURFACE),
            Some(theme::BORD2),
            16.0,
            theme::TEXTE,
        ))
        .on_press_maybe(message)
        .into()
}

/// Bouton plein colore (« Voir », « Mettre à jour »), icone apres ou avant le texte.
pub fn bouton_plein<'a>(
    fond: Color,
    libelle: &'a str,
    forme: Icone,
    icone_avant: bool,
    hauteur: f32,
    message: Option<Message>,
) -> Vue<'a> {
    let dessin = icone(
        forme,
        if icone_avant { 22.0 } else { 20.0 },
        2.2,
        theme::FOND,
    );
    let texte: Vue<'a> = t(libelle, 16).font(DEMI_GRAS).into();
    let contenu = if icone_avant {
        row![dessin, texte]
    } else {
        row![texte, dessin]
    };
    button(au_centre(contenu.spacing(10).align_y(Alignment::Center)))
        .height(hauteur)
        .padding([0, 20])
        .style(theme::touchable(Some(fond), None, 14.0, theme::FOND))
        .on_press_maybe(message)
        .into()
}

pub fn pilule_tailscale<'a>(etat: &'a Etat, maintenant: i64) -> Vue<'a> {
    let ts = &etat.tailscale;
    let (niveau, libelle, couleur_libelle, detail): (Niveau, &str, Color, Option<Vue<'a>>) =
        match (&ts.tablette.ip, ts.exit_node.actif, ts.actif) {
            (Some(ip), true, _) => (
                Niveau::Warn,
                "exit node activé",
                theme::WARN,
                Some(mono_discret(ip, 13.0)),
            ),
            (Some(ip), false, _) => (
                Niveau::Up,
                "connecté",
                theme::UP,
                Some(mono_discret(ip, 13.0)),
            ),
            (None, _, true) => (
                Niveau::Down,
                "déconnecté",
                theme::DOWN,
                Some(
                    t(
                        format!("depuis {}", duree((maintenant - ts.tablette.depuis) as f64)),
                        15,
                    )
                    .color(theme::DISCRET)
                    .into(),
                ),
            ),
            (None, _, false) => (Niveau::Inconnu, "pas sur le tailnet", theme::DISCRET, None),
        };
    let mut contenu = row![
        point(niveau, 12.0),
        t("Tailscale", 15),
        t(libelle, 15).font(GRAS).color(couleur_libelle)
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if let Some(d) = detail {
        contenu = contenu.push(d);
    }
    button(au_centre(contenu))
        .height(52)
        .padding([0, 20])
        .style(theme::touchable(
            Some(theme::SURFACE),
            Some(theme::BORD2),
            26.0,
            theme::TEXTE,
        ))
        .on_press(Message::Aller(Page::Tailscale))
        .into()
}

pub fn mono_discret<'a>(
    contenu: impl text::IntoFragment<'a>,
    taille: impl Into<iced::Pixels>,
) -> Vue<'a> {
    t(contenu, taille).font(MONO).color(theme::DISCRET).into()
}

/// Badge rond contenant l'icone d'etat.
pub fn badge<'a>(niveau: Niveau, taille: f32, taille_icone: f32) -> Vue<'a> {
    container(icone_etat(niveau, taille_icone))
        .center(taille)
        .style(move |_: &Theme| theme::boite(theme::couleur_fond(niveau), None, taille / 2.0))
        .into()
}

/// Titre de section (`Nodes  2 sur 3 en ligne`).
pub fn titre_section<'a>(titre: &'a str, detail: Option<String>) -> Vue<'a> {
    let mut ligne = row![t(titre, 18).font(DEMI_GRAS)]
        .spacing(12)
        .align_y(Alignment::End);
    if let Some(d) = detail {
        ligne = ligne.push(t(d, 14).color(theme::DISCRET));
    }
    ligne.into()
}

/// Encadre pour une liste vide ou une explication.
pub fn vide<'a>(contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    container(contenu)
        .padding(28)
        .width(Length::Fill)
        .style(|_: &Theme| container::Style {
            text_color: Some(theme::DISCRET),
            ..theme::boite(Color::TRANSPARENT, Some(theme::BORD2), 16.0)
        })
        .into()
}

/// Zone qui defile au doigt ; la barre fine a droite indique qu'il y a une suite
/// (et se fait glisser si l'ecran n'envoie que des clics).
pub fn defilant<'a>(contenu: impl Into<Vue<'a>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(contenu)
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(10)
                .scroller_width(4)
                .spacing(4),
        ))
        .style(|_: &Theme, _| {
            let rail = scrollable::Rail {
                background: None,
                border: iced::Border::default(),
                scroller: scrollable::Scroller {
                    background: theme::BORD2.into(),
                    border: iced::border::rounded(2),
                },
            };
            scrollable::Style {
                container: container::Style::default(),
                vertical_rail: rail,
                horizontal_rail: rail,
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: Color::TRANSPARENT.into(),
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                    icon: Color::TRANSPARENT,
                },
            }
        })
}

/// Alerte rouge en haut d'une page (erreur d'API, de mise a jour).
pub fn alerte<'a>(texte: &'a str) -> Vue<'a> {
    container(t(texte, 15).color(theme::DOWN_PALE))
        .padding([12, 16])
        .width(Length::Fill)
        .style(|_: &Theme| theme::boite(theme::DOWN_CARTE, Some(theme::DOWN_BORD), 12.0))
        .into()
}

pub fn toast<'a>(texte: &'a str) -> Vue<'a> {
    let bulle = container(t(texte, 16).font(DEMI_GRAS))
        .padding([14, 22])
        .style(|_: &Theme| theme::boite(theme::SURFACE2, Some(theme::BORD2), 14.0));
    container(bulle)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(Padding::ZERO.bottom(28))
        .into()
}

pub fn hors_ligne<'a>() -> Vue<'a> {
    let bandeau = container(
        t("Serveur de monitoring injoignable : données figées", 15)
            .font(GRAS)
            .color(theme::FOND),
    )
    .padding(10)
    .center_x(Length::Fill)
    .style(|_: &Theme| theme::boite(theme::DOWN, None, 0.0));
    column![bandeau].height(Length::Fill).into()
}
