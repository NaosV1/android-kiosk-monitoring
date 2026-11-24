//! Couleurs, polices et styles communs de l'ecran.
//!
//! Fond gris clair, cartes blanches, une seule couleur d'action (bleu). Le vert, l'ambre et le
//! rouge sont reserves aux etats. Les couleurs de texte tiennent un contraste d'au moins 4,5:1 sur
//! le fond ou elles sont posees, y compris sur les fonds teintes des etats.

use iced::font::Weight;
use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Font, Theme, color};

use crate::api::Niveau;

pub const FOND: Color = color!(0xF2F2F6);
pub const SURFACE: Color = color!(0xFFFFFF);
/// Zone en creux dans une carte (pistes, valeurs secondaires).
pub const CREUX: Color = color!(0xF2F2F6);
pub const PISTE: Color = color!(0xE6E6EB);
pub const SEPARATEUR: Color = color!(0xE5E5EA);

pub const TEXTE: Color = color!(0x1C1C1E);
pub const SECONDAIRE: Color = color!(0x636366);
/// Elements inactifs ou decoratifs uniquement (contraste insuffisant pour du texte courant).
pub const TERTIAIRE: Color = color!(0xAEAEB2);

pub const ACCENT: Color = color!(0x2563EB);
pub const ACCENT_FOND: Color = color!(0xEAF1FE);

/// Courbes des nodes, dans l'ordre de la config : quatre familles nettement separees (cyan, violet,
/// ardoise, encre), loin du vert, de l'ambre et du rouge des etats et du bleu d'action.
pub const SERIES: [Color; 4] = [
    color!(0x0891B2),
    color!(0x9333EA),
    color!(0x64748B),
    color!(0x1C1C1E),
];

/// Ecran de veille, facon horloge de chevet.
pub mod nuit {
    use iced::{Color, color};

    pub const FOND: Color = color!(0x000000);
    pub const TEXTE: Color = color!(0xF2F2F7);
    pub const SECONDAIRE: Color = color!(0x98989F);
    pub const DISCRET: Color = color!(0x8E8E93);
    pub const UP: Color = color!(0x30D158);
    pub const WARN: Color = color!(0xFF9F0A);
    pub const DOWN: Color = color!(0xFF453A);
}

/// Les teintes d'un etat.
#[derive(Clone, Copy)]
pub struct Ton {
    /// Pastilles, barres, icones.
    pub plein: Color,
    /// Texte sur fond blanc.
    pub texte: Color,
    /// Fond teinte (bandeau, carte en alerte).
    pub fond: Color,
    /// Texte secondaire sur ce fond teinte.
    pub texte_fond: Color,
}

pub fn ton(niveau: Niveau) -> Ton {
    match niveau {
        Niveau::Up => Ton {
            plein: color!(0x34C759),
            texte: color!(0x1B7F3B),
            fond: color!(0xE8F6EC),
            texte_fond: color!(0x17602E),
        },
        Niveau::Warn | Niveau::Lent => Ton {
            plein: color!(0xFF9F0A),
            texte: color!(0xA15C00),
            fond: color!(0xFFF4E3),
            texte_fond: color!(0x774400),
        },
        Niveau::Down => Ton {
            plein: color!(0xFF3B30),
            texte: color!(0xC8261B),
            fond: color!(0xFDECEA),
            texte_fond: color!(0x8F1D14),
        },
        Niveau::Inconnu | Niveau::Vide => Ton {
            plein: TERTIAIRE,
            texte: SECONDAIRE,
            fond: CREUX,
            texte_fond: SECONDAIRE,
        },
    }
}

/// Couleur pleine d'un etat (pastilles, icones).
pub fn couleur(niveau: Niveau) -> Color {
    ton(niveau).plein
}

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
/// Adresses, IP, chemins : des donnees, pas un decor.
pub const MONO: Font = Font::with_name("JetBrains Mono");

pub fn boite(fond: Color, rayon: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(fond)),
        border: Border {
            radius: rayon.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Ligne d'une liste dans une carte : seuls ses coins au bord de la carte sont arrondis
/// (le decoupage d'iced est rectangulaire, un fond teinte deborderait sinon).
pub fn ligne_de_liste(fond: Color, premiere: bool, derniere: bool) -> container::Style {
    let mut rayon = iced::border::Radius::new(0);
    if premiere {
        rayon = rayon.top(16);
    }
    if derniere {
        rayon = rayon.bottom(16);
    }
    container::Style {
        background: Some(Background::Color(fond)),
        border: Border {
            radius: rayon,
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// Carte blanche : la seule elevation de l'interface (pas de bordure, pas d'ombre).
pub fn carte(_: &Theme) -> container::Style {
    boite(SURFACE, 16.0)
}

/// Element touchable : un fond un peu plus sombre sous le doigt, attenue s'il est desactive.
pub fn touchable(
    fond: Option<Color>,
    rayon: f32,
    texte: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, statut| {
        let fond = match statut {
            button::Status::Pressed => {
                Some(fond.map_or(Color::from_rgba(0.0, 0.0, 0.0, 0.06), assombrir))
            }
            _ => fond,
        };
        let alpha = if statut == button::Status::Disabled {
            0.4
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
                radius: rayon.into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

fn assombrir(c: Color) -> Color {
    let f = |v: f32| v * 0.9;
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
