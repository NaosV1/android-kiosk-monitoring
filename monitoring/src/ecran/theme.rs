//! Couleurs, polices et styles communs de l'ecran (tons sombres, lisibles de loin).

use iced::font::Weight;
use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Font, Theme, color};

use crate::api::Niveau;

pub const FOND: Color = color!(0x0B0F14);
pub const RAIL: Color = color!(0x0E131A);
pub const RAIL_BORD: Color = color!(0x1C2430);
pub const LOGO: Color = color!(0x16202C);
pub const SURFACE: Color = color!(0x121821);
pub const SURFACE2: Color = color!(0x1A2230);
pub const CREUX: Color = color!(0x0F141B);
pub const BORD: Color = color!(0x1E2733);
pub const BORD2: Color = color!(0x263041);
pub const TEXTE: Color = color!(0xE8EDF4);
pub const TEXTE2: Color = color!(0xC9D3E0);
pub const DISCRET: Color = color!(0x93A1B5);
pub const ACCENT: Color = color!(0x6CB6FF);
pub const ACTIF: Color = color!(0x1A2638);

pub const UP: Color = color!(0x3DD68C);
pub const WARN: Color = color!(0xF5A524);
pub const DOWN: Color = color!(0xFF5C5C);
pub const UP_FOND: Color = color!(0x12261D);
pub const WARN_FOND: Color = color!(0x2E2410);
pub const DOWN_FOND: Color = color!(0x3A1719);
pub const WARN_CARTE: Color = color!(0x1E1A12);
pub const WARN_BORD: Color = color!(0x7A5716);
pub const DOWN_CARTE: Color = color!(0x251416);
pub const DOWN_BORD: Color = color!(0x8A2E33);
pub const OK_CARTE: Color = color!(0x0F1D17);
pub const OK_BORD: Color = color!(0x1E4A35);
pub const INFO_FOND: Color = color!(0x12233A);
pub const INFO_BORD: Color = color!(0x23476E);

pub const WARN_CLAIR: Color = color!(0xF5C46B);
pub const DOWN_CLAIR: Color = color!(0xFFB3B3);
pub const DOWN_PALE: Color = color!(0xFFD0D0);
pub const GRIS_NODE: Color = color!(0x4A5568);
pub const VERT_BARRE: Color = color!(0x2E9E6B);

/// Couleurs des courbes des nodes, dans l'ordre de la config.
pub const SERIES: [Color; 8] = [
    color!(0x6CB6FF),
    color!(0xC4A5FF),
    color!(0xFF8CC6),
    color!(0x7EE0C3),
    color!(0xF2CC60),
    color!(0xA0B4FF),
    color!(0xFFB38A),
    color!(0x9BE15D),
];

pub const INTER: Font = Font::with_name("Inter");
pub const MOYEN: Font = Font {
    weight: Weight::Medium,
    ..INTER
};
pub const DEMI_GRAS: Font = Font {
    weight: Weight::Semibold,
    ..INTER
};
pub const GRAS: Font = Font {
    weight: Weight::Bold,
    ..INTER
};
pub const LEGER: Font = Font {
    weight: Weight::Light,
    ..INTER
};
pub const MONO: Font = Font::with_name("JetBrains Mono");

/// Couleur vive d'un etat (texte, icones).
pub fn couleur(niveau: Niveau) -> Color {
    match niveau {
        Niveau::Up => UP,
        Niveau::Warn | Niveau::Lent => WARN,
        Niveau::Down => DOWN,
        Niveau::Inconnu | Niveau::Vide => DISCRET,
    }
}

/// Fond discret des pastilles d'etat (badges, etiquettes).
pub fn couleur_fond(niveau: Niveau) -> Color {
    match niveau {
        Niveau::Up => UP_FOND,
        Niveau::Warn | Niveau::Lent => WARN_FOND,
        Niveau::Down => DOWN_FOND,
        Niveau::Inconnu | Niveau::Vide => SURFACE2,
    }
}

/// Fond et bord des cartes selon l'etat (`None` : carte neutre).
pub fn carte_etat(niveau: Niveau) -> Option<(Color, Color)> {
    match niveau {
        Niveau::Warn | Niveau::Lent => Some((WARN_CARTE, WARN_BORD)),
        Niveau::Down => Some((DOWN_CARTE, DOWN_BORD)),
        _ => None,
    }
}

pub fn boite(fond: Color, bord: Option<Color>, rayon: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(fond)),
        border: Border {
            color: bord.unwrap_or(Color::TRANSPARENT),
            width: if bord.is_some() { 1.0 } else { 0.0 },
            radius: rayon.into(),
        },
        ..container::Style::default()
    }
}

/// Carte standard (surface, bord discret, coins de 16).
pub fn carte(_: &Theme) -> container::Style {
    boite(SURFACE, Some(BORD), 16.0)
}

/// Element touchable : un leger eclaircissement quand le doigt appuie,
/// une opacite reduite quand il est desactive.
pub fn touchable(
    fond: Option<Color>,
    bord: Option<Color>,
    rayon: f32,
    texte: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, statut| {
        let fond = match statut {
            button::Status::Pressed => {
                Some(fond.map_or(Color::from_rgba(1.0, 1.0, 1.0, 0.08), eclaircir))
            }
            _ => fond,
        };
        let alpha = if statut == button::Status::Disabled {
            0.45
        } else {
            1.0
        };
        button::Style {
            background: fond.map(|c| {
                Background::Color(Color {
                    a: c.a * alpha,
                    ..c
                })
            }),
            text_color: Color { a: alpha, ..texte },
            border: Border {
                color: bord.map_or(Color::TRANSPARENT, |c| Color {
                    a: c.a * alpha,
                    ..c
                }),
                width: if bord.is_some() { 1.0 } else { 0.0 },
                radius: rayon.into(),
            },
            ..button::Style::default()
        }
    }
}

fn eclaircir(c: Color) -> Color {
    let f = |v: f32| (v * 1.2 + 0.02).min(1.0);
    Color {
        r: f(c.r),
        g: f(c.g),
        b: f(c.b),
        a: c.a,
    }
}

/// Texte d'une taille donnee (en pixels logiques de la maquette 1280x800).
pub fn t<'a>(
    contenu: impl text::IntoFragment<'a>,
    taille: impl Into<iced::Pixels>,
) -> iced::widget::Text<'a> {
    text(contenu).size(taille)
}

/// Morceau de texte mis en forme (police ou couleur differente au fil d'une phrase).
pub type Morceau<'a> = text::Span<'a, (), Font>;

pub fn texte_riche<'a, M: 'a>(morceaux: Vec<Morceau<'a>>) -> text::Rich<'a, (), M> {
    iced::widget::rich_text(morceaux)
}
