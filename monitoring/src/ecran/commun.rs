//! Elements partages par les pages : barre laterale, en-tete, horloge, pastilles d'etat,
//! boutons, cartes, messages ephemeres.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding, Theme};

use crate::api::{Etat, Niveau, StatutMaj};

use super::Message;
use super::format::{self, jour_long, nom_version};
use super::icones::{Icone, icone};
use super::logique::{Page, pire};
use super::theme::{self, DEMI_GRAS, GRAS, MOYEN, t};

pub type Vue<'a> = Element<'a, Message>;

const LARGEUR_BARRE: f32 = 220.0;
/// Hauteur minimale d'une cible tactile.
pub const CIBLE: f32 = 44.0;

/// Ecran d'attente avant le premier etat du serveur.
pub fn demarrage<'a>(erreur: bool) -> Vue<'a> {
    let message = if erreur {
        "Serveur de monitoring injoignable, nouvelle tentative…"
    } else {
        "Démarrage du monitoring…"
    };
    container(t(message, 20).color(theme::SECONDAIRE))
        .center(Length::Fill)
        .style(|_: &Theme| theme::boite(theme::FOND, 0.0))
        .into()
}

/// Contenu d'un bouton de hauteur fixe, centre verticalement.
pub fn au_centre<'a>(contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    container(contenu).center_y(Length::Fill).into()
}

/// Squelette d'une page : barre laterale a gauche, contenu a droite. `perime` : serveur
/// injoignable, le compteur d'incidents perd sa teinte d'etat comme le reste de l'ecran.
pub fn ecran<'a>(
    page: &Page,
    etat: &'a Etat,
    perime: bool,
    contenu: impl Into<Vue<'a>>,
) -> Vue<'a> {
    row![
        barre_laterale(page, etat, perime),
        container(contenu)
            .padding([24, 28])
            .width(Length::Fill)
            .height(Length::Fill)
    ]
    .into()
}

fn barre_laterale<'a>(page: &Page, etat: &'a Etat, perime: bool) -> Vue<'a> {
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
        let (couleur_icone, couleur_texte) = if est_actif {
            (theme::ACCENT, theme::ACCENT)
        } else {
            (theme::SECONDAIRE, theme::TEXTE)
        };
        let mut ligne = row![
            icone(forme, 22.0, 1.9, couleur_icone),
            t(libelle, 15)
                .font(if est_actif { DEMI_GRAS } else { MOYEN })
                .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        if let Some((nombre, fond)) = pastille {
            ligne = ligne.push(compteur(nombre, fond));
        }
        button(au_centre(ligne))
            .width(Length::Fill)
            .height(48)
            .padding([0, 12])
            .style(theme::touchable(
                est_actif.then_some(theme::ACCENT_FOND),
                12.0,
                couleur_texte,
            ))
            .on_press(Message::Aller(cible))
            .into()
    };
    let ouverts = &etat.incidents_ouverts;
    let pastille_incidents = (!ouverts.is_empty()).then(|| {
        let niveau = pire(ouverts.iter().map(|i| i.niveau));
        let fond = if perime {
            theme::SECONDAIRE
        } else {
            theme::ton(niveau).texte
        };
        (ouverts.len().to_string(), fond)
    });
    let m = &etat.maj;
    let pastille_maj = (m.disponible.is_some()
        && matches!(m.statut, StatutMaj::Prete | StatutMaj::Telechargement))
    .then(|| ("1".to_string(), theme::ACCENT));
    let contenu = column![
        container(t("Monitoring", 20).font(GRAS)).padding(Padding::ZERO.left(12).top(6).bottom(22)),
        lien(Page::Accueil, "Vue d'ensemble", Icone::Accueil, None),
        lien(
            Page::Incidents,
            "Incidents",
            Icone::Cloche,
            pastille_incidents
        ),
        lien(Page::Tailscale, "Tailscale", Icone::Tailscale, None),
        lien(Page::Maj, "Mise à jour", Icone::Maj, pastille_maj),
        Space::new().height(Length::Fill),
        lien(Page::Veille, "Veille", Icone::Lune, None),
        container(t(nom_version(&m.actuelle), 13).color(theme::SECONDAIRE))
            .padding(Padding::ZERO.left(12).top(6)),
    ]
    .spacing(4);
    let bordure = container(Space::new().width(1).height(Length::Fill))
        .style(|_: &Theme| theme::boite(theme::SEPARATEUR, 0.0));
    row![
        container(contenu)
            .width(LARGEUR_BARRE)
            .height(Length::Fill)
            .padding([22, 14])
            .style(|_: &Theme| theme::boite(theme::SURFACE, 0.0)),
        bordure,
    ]
    .into()
}

/// Pastille numerique de la barre laterale.
fn compteur<'a>(nombre: String, fond: Color) -> Vue<'a> {
    container(t(nombre, 13).font(GRAS).color(theme::SURFACE))
        .center_x(Length::Shrink)
        .padding([2, 8])
        .style(move |_: &Theme| theme::boite(fond, 11.0))
        .into()
}

pub fn horloge<'a>() -> Vue<'a> {
    let maintenant = chrono::Local::now();
    column![
        t(format::heure(&maintenant), 30)
            .font(DEMI_GRAS)
            .line_height(1.0),
        t(jour_long(&maintenant), 13).color(theme::SECONDAIRE),
    ]
    .spacing(4)
    .align_x(Alignment::End)
    .width(170)
    .into()
}

/// Titre de page, pastille d'etat eventuelle, et sa ligne de detail.
pub fn titres<'a>(titre: impl Into<String>, pastille: Option<Vue<'a>>, sous: Vue<'a>) -> Vue<'a> {
    let mut ligne = row![t(titre.into(), 28).font(GRAS).wrapping(Wrapping::None)]
        .spacing(14)
        .align_y(Alignment::Center);
    if let Some(p) = pastille {
        ligne = ligne.push(p);
    }
    column![ligne, sous].spacing(4).width(Length::Fill).into()
}

/// Ligne de detail sous un titre (une seule ligne, coupee si trop longue).
pub fn sous_titre<'a>(contenu: Vue<'a>) -> Vue<'a> {
    container(contenu).clip(true).width(Length::Fill).into()
}

/// Texte secondaire d'une ligne, coupe s'il deborde.
pub fn ligne_secondaire<'a>(texte: impl Into<String>, taille: f32) -> Vue<'a> {
    sous_titre(
        t(texte.into(), taille)
            .color(theme::SECONDAIRE)
            .wrapping(Wrapping::None)
            .into(),
    )
}

pub fn entete<'a>(elements: Vec<Vue<'a>>) -> Vue<'a> {
    row(elements)
        .spacing(16)
        .align_y(Alignment::Center)
        .height(Length::Shrink)
        .into()
}

/// Repere d'etat, avec une forme par etat pour ne pas dependre de la couleur seule :
/// rond plein (en ligne), carre (degrade ou lent), anneau (hors ligne), rond gris (inconnu).
pub fn point<'a>(niveau: Niveau, taille: f32) -> Vue<'a> {
    point_teinte(niveau, taille, theme::couleur(niveau))
}

/// Repere d'etat dans une teinte imposee (palette de nuit de la veille).
pub fn point_teinte<'a>(niveau: Niveau, taille: f32, plein: Color) -> Vue<'a> {
    let style = move |_: &Theme| match niveau {
        Niveau::Warn | Niveau::Lent => theme::boite(plein, taille * 0.25),
        Niveau::Down => container::Style {
            border: iced::Border {
                color: plein,
                width: (taille * 0.22).max(2.0),
                radius: (taille / 2.0).into(),
            },
            ..container::Style::default()
        },
        _ => theme::boite(plein, taille / 2.0),
    };
    container(Space::new())
        .width(taille)
        .height(taille)
        .style(style)
        .into()
}

/// Capsule d'etat : repere et libelle sur le fond teinte de l'etat ; `None` = information (bleu).
pub fn pastille<'a>(niveau: Option<Niveau>, libelle: impl Into<String>) -> Vue<'a> {
    let (fond, couleur) = match niveau {
        Some(n) => (theme::ton(n).fond, theme::ton(n).texte),
        None => (theme::ACCENT_FOND, theme::ACCENT),
    };
    let mut contenu = row![].spacing(8).align_y(Alignment::Center);
    if let Some(n) = niveau {
        contenu = contenu.push(point(n, 9.0));
    }
    container(contenu.push(t(libelle.into(), 14).font(DEMI_GRAS).color(couleur)))
        .height(30)
        .padding([0, 12])
        .center_y(30)
        .style(move |_: &Theme| theme::boite(fond, 15.0))
        .into()
}

fn contenu_bouton<'a>(forme: Option<Icone>, libelle: &'a str, couleur: Color) -> Vue<'a> {
    let mut contenu = row![].spacing(8).align_y(Alignment::Center);
    if let Some(f) = forme {
        contenu = contenu.push(icone(f, 20.0, 2.0, couleur));
    }
    au_centre(contenu.push(t(libelle, 15).font(DEMI_GRAS)))
}

/// Bouton secondaire, teinte de bleu.
pub fn bouton<'a>(forme: Option<Icone>, libelle: &'a str, message: Option<Message>) -> Vue<'a> {
    button(contenu_bouton(forme, libelle, theme::ACCENT))
        .height(CIBLE)
        .padding([0, 16])
        .style(theme::touchable(
            Some(theme::ACCENT_FOND),
            12.0,
            theme::ACCENT,
        ))
        .on_press_maybe(message)
        .into()
}

/// Bouton principal, plein bleu.
pub fn bouton_principal<'a>(
    forme: Option<Icone>,
    libelle: &'a str,
    message: Option<Message>,
) -> Vue<'a> {
    button(contenu_bouton(forme, libelle, theme::SURFACE))
        .height(CIBLE)
        .padding([0, 18])
        .style(theme::touchable(Some(theme::ACCENT), 12.0, theme::SURFACE))
        .on_press_maybe(message)
        .into()
}

/// Bouton neutre (Annuler).
pub fn bouton_neutre<'a>(libelle: &'a str, message: Message) -> Vue<'a> {
    button(contenu_bouton(None, libelle, theme::TEXTE))
        .height(CIBLE)
        .padding([0, 18])
        .style(theme::touchable(Some(theme::CREUX), 12.0, theme::TEXTE))
        .on_press(message)
        .into()
}

/// Lien d'action dans un bandeau ou une carte teintee : texte et chevron dans la teinte.
pub fn lien<'a>(libelle: &'a str, couleur: Color, message: Message) -> Vue<'a> {
    button(au_centre(
        row![
            t(libelle, 15).font(DEMI_GRAS),
            icone(Icone::Droite, 18.0, 2.2, couleur)
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    ))
    .height(CIBLE)
    .padding([0, 12])
    .style(theme::touchable(None, 10.0, couleur))
    .on_press(message)
    .into()
}

pub fn bouton_retour<'a>() -> Vue<'a> {
    button(container(icone(Icone::Gauche, 22.0, 2.2, theme::TEXTE)).center(Length::Fill))
        .width(CIBLE)
        .height(CIBLE)
        .padding(0)
        .style(theme::touchable(
            Some(theme::SURFACE),
            CIBLE / 2.0,
            theme::TEXTE,
        ))
        .on_press(Message::Aller(Page::Accueil))
        .into()
}

/// Carte blanche aux marges standard.
pub fn carte<'a>(contenu: impl Into<Vue<'a>>) -> container::Container<'a, Message> {
    container(contenu)
        .padding([16, 20])
        .width(Length::Fill)
        .style(theme::carte)
}

/// Titre d'un bloc de page (`Nodes · 2 sur 3 en ligne`).
pub fn titre_section<'a>(titre: impl Into<String>, detail: Option<String>) -> Vue<'a> {
    let mut ligne = row![t(titre.into(), 17).font(DEMI_GRAS)]
        .spacing(10)
        .align_y(Alignment::End);
    if let Some(d) = detail {
        ligne = ligne.push(t(d, 14).color(theme::SECONDAIRE));
    }
    ligne.into()
}

/// Filet entre deux lignes d'une liste.
pub fn separateur<'a>() -> Vue<'a> {
    container(Space::new().width(Length::Fill).height(1))
        .style(|_: &Theme| theme::boite(theme::SEPARATEUR, 0.0))
        .into()
}

/// Filet d'une liste groupee : il commence au debut du texte, pas au bord de la carte.
pub fn separateur_en_retrait<'a>(retrait: f32) -> Vue<'a> {
    container(separateur())
        .padding(Padding::ZERO.left(retrait))
        .into()
}

/// Carte d'une liste vide ou d'une explication.
pub fn vide<'a>(contenu: impl Into<Vue<'a>>) -> Vue<'a> {
    container(contenu)
        .padding([20, 20])
        .width(Length::Fill)
        .style(|th: &Theme| container::Style {
            text_color: Some(theme::SECONDAIRE),
            ..theme::carte(th)
        })
        .into()
}

/// Valeur chiffree : libelle, grand chiffre, legende ; touchable si un message est donne.
/// Valeur d'une tuile : le nombre en grand, l'unite eventuelle plus petite, en gris.
pub struct Valeur {
    pub nombre: String,
    pub unite: Option<String>,
    pub couleur: Color,
}

impl Valeur {
    pub fn texte(nombre: impl Into<String>) -> Valeur {
        Valeur {
            nombre: nombre.into(),
            unite: None,
            couleur: theme::TEXTE,
        }
    }

    pub fn avec_unite(nombre: impl Into<String>, unite: impl Into<String>) -> Valeur {
        Valeur {
            nombre: nombre.into(),
            unite: Some(unite.into()),
            couleur: theme::TEXTE,
        }
    }

    pub fn couleur(self, couleur: Color) -> Valeur {
        Valeur { couleur, ..self }
    }
}

pub fn tuile<'a>(
    libelle: &'a str,
    valeur: Valeur,
    legende: String,
    message: Option<Message>,
) -> Vue<'a> {
    let mut chiffre = row![
        t(valeur.nombre, 26)
            .font(GRAS)
            .color(valeur.couleur)
            .wrapping(Wrapping::None)
    ]
    .spacing(4)
    .align_y(Alignment::End);
    if let Some(unite) = valeur.unite {
        chiffre = chiffre.push(
            container(t(unite, 17).font(DEMI_GRAS).color(theme::SECONDAIRE))
                .padding(Padding::ZERO.bottom(3)),
        );
    }
    let contenu = column![
        t(libelle, 13).font(MOYEN).color(theme::SECONDAIRE),
        container(chiffre).clip(true),
        ligne_secondaire(legende, 13.0),
    ]
    .spacing(4);
    match message {
        Some(m) => button(contenu)
            .width(Length::FillPortion(1))
            .padding([14, 18])
            .style(theme::touchable(Some(theme::SURFACE), 16.0, theme::TEXTE))
            .on_press(m)
            .into(),
        None => container(contenu)
            .width(Length::FillPortion(1))
            .padding([14, 18])
            .style(theme::carte)
            .into(),
    }
}

/// Zone qui defile au doigt ; la barre fine a droite indique qu'il y a une suite
/// (et se fait glisser si l'ecran n'envoie que des clics).
pub fn defilant<'a>(contenu: impl Into<Vue<'a>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(contenu)
        .direction(scrollable::Direction::Vertical(
            // barre posee sur le contenu (comme sur tablette) : la largeur utile ne change pas
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .style(|_: &Theme, _| {
            let rail = scrollable::Rail {
                background: None,
                border: iced::Border::default(),
                scroller: scrollable::Scroller {
                    background: theme::TERTIAIRE.into(),
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

/// Message d'erreur en haut d'une page (API Tailscale, mise a jour).
pub fn alerte<'a>(texte: &'a str) -> Vue<'a> {
    let ton = theme::ton(Niveau::Down);
    container(
        row![
            icone(Icone::Info, 20.0, 2.0, ton.texte),
            t(texte, 15).color(ton.texte_fond).width(Length::Fill)
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(move |_: &Theme| theme::boite(ton.fond, 14.0))
    .into()
}

pub fn toast<'a>(texte: &'a str) -> Vue<'a> {
    let bulle = container(t(texte, 15).font(DEMI_GRAS).color(theme::SURFACE))
        .padding([12, 20])
        .style(|_: &Theme| {
            theme::boite(
                Color {
                    a: 0.92,
                    ..theme::TEXTE
                },
                22.0,
            )
        });
    container(bulle)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(Padding::ZERO.bottom(28))
        .into()
}

/// Bandeau du serveur injoignable : depuis quand l'affichage est fige, et la reprise automatique.
pub fn hors_ligne<'a>(recu: i64, periode: u64) -> Vue<'a> {
    let depuis = format::heure(&format::local(recu));
    container(
        t(
            format!(
                "Serveur de monitoring injoignable : affichage figé depuis {depuis}, nouvelle tentative toutes les {periode} s"
            ),
            14,
        )
        .font(DEMI_GRAS)
        .color(theme::SURFACE),
    )
    .padding(9)
    .center_x(Length::Fill)
    .style(|_: &Theme| theme::boite(theme::ton(Niveau::Down).texte, 0.0))
    .into()
}

/// Texte en police a chasse fixe (adresse, IP, chemin).
pub fn mono<'a>(contenu: impl text::IntoFragment<'a>, taille: f32, couleur: Color) -> Vue<'a> {
    t(contenu, taille)
        .font(theme::MONO)
        .color(couleur)
        .wrapping(Wrapping::None)
        .into()
}
