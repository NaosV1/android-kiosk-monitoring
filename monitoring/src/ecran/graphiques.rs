//! Courbes des temps de reponse : mini-courbe des tuiles et graphe 24 h d'un service.

use iced::alignment::Vertical;
use iced::widget::canvas::{
    self, Frame, Geometry, LineCap, LineDash, LineJoin, Path, Stroke, Text,
};
use iced::widget::text::Alignment;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, color, mouse};

use crate::api::Niveau;

use super::logique::plafond;
use super::theme::{self, INTER};

/// Mini-courbe 24 h d'une tuile (`None` = pas de mesure sur la tranche).
pub fn sparkline<'a, M: 'a>(valeurs: &[Option<f64>], etat: Niveau) -> Element<'a, M> {
    iced::widget::canvas(Sparkline {
        valeurs: valeurs.to_vec(),
        etat,
    })
    .width(Length::Fill)
    .height(18)
    .into()
}

struct Sparkline {
    valeurs: Vec<Option<f64>>,
    etat: Niveau,
}

fn trait_plein(couleur: Color, largeur: f32) -> Stroke<'static> {
    Stroke::default()
        .with_color(couleur)
        .with_width(largeur)
        .with_line_cap(LineCap::Round)
        .with_line_join(LineJoin::Round)
}

/// Segments continus de la courbe (les trous coupent la ligne).
fn segments(valeurs: &[Option<f64>], point: impl Fn(usize, f64) -> Point) -> Vec<Vec<Point>> {
    let mut segments = Vec::new();
    let mut courant = Vec::new();
    for (i, v) in valeurs.iter().enumerate() {
        match v {
            Some(v) => courant.push(point(i, *v)),
            None if !courant.is_empty() => segments.push(std::mem::take(&mut courant)),
            None => {}
        }
    }
    if !courant.is_empty() {
        segments.push(courant);
    }
    segments
}

fn polyligne(points: &[Point]) -> Path {
    Path::new(|b| {
        for (i, p) in points.iter().enumerate() {
            if i == 0 { b.move_to(*p) } else { b.line_to(*p) }
        }
    })
}

impl<M> canvas::Program<M> for Sparkline {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        limites: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut cadre = Frame::new(renderer, limites.size());
        // reperes d'origine : 200 x 34, etire a la taille du widget
        let (kx, ky) = (limites.width / 200.0, limites.height / 34.0);
        let definies: Vec<f64> = self.valeurs.iter().flatten().copied().collect();
        if definies.is_empty() {
            let y = 31.0 * ky;
            cadre.stroke(
                &Path::line(Point::new(0.0, y), Point::new(limites.width, y)),
                Stroke {
                    line_dash: LineDash {
                        segments: &[4.0, 4.0],
                        offset: 0,
                    },
                    ..trait_plein(theme::BORD2, 2.0)
                },
            );
            return vec![cadre.into_geometry()];
        }
        let max = definies.iter().fold(0.0_f64, |a, &b| a.max(b)) * 1.25;
        let max = if max > 0.0 { max } else { 1.0 };
        let n = self.valeurs.len().saturating_sub(1).max(1) as f32;
        let couleur = match self.etat {
            Niveau::Up => color!(0x4C8C6E),
            Niveau::Inconnu | Niveau::Vide => theme::GRIS_NODE,
            etat => theme::couleur(etat),
        };
        let point = |i: usize, v: f64| {
            Point::new(
                i as f32 * 200.0 / n * kx,
                (31.0 - (v / max) as f32 * 27.0) * ky,
            )
        };
        for mut seg in segments(&self.valeurs, point) {
            if seg.len() == 1 {
                let p = seg[0];
                seg = vec![
                    Point::new(p.x - 1.5 * kx, p.y),
                    Point::new(p.x + 1.5 * kx, p.y),
                ];
            }
            cadre.stroke(&polyligne(&seg), trait_plein(couleur, 2.0));
        }
        vec![cadre.into_geometry()]
    }
}

/// Une courbe du graphe 24 h.
pub struct Serie {
    pub couleur: Color,
    pub valeurs: Vec<Option<f64>>,
    /// Node hors ligne : une croix marque sa derniere mesure.
    pub hors_ligne: bool,
}

/// Graphe des temps de reponse sur 24 h, avec la ligne du seuil « lent ».
pub fn graphe<'a, M: 'a>(series: Vec<Serie>, seuil_lent: f64) -> Element<'a, M> {
    iced::widget::canvas(Graphe { series, seuil_lent })
        .width(Length::Fill)
        .height(176)
        .into()
}

struct Graphe {
    series: Vec<Serie>,
    seuil_lent: f64,
}

/// Valeur de l'axe : `500 ms`, `2,5 s`.
fn graduation(v: f64) -> String {
    if v >= 1000.0 {
        format!("{} s", super::format::nombre(v / 1000.0))
    } else {
        format!("{} ms", super::format::nombre(v))
    }
}

impl<M> canvas::Program<M> for Graphe {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        limites: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut cadre = Frame::new(renderer, limites.size());
        // reperes d'origine : 640 x 186, mis a l'echelle sans deformation
        let k = (limites.width / 640.0).min(limites.height / 186.0);
        let p = |x: f32, y: f32| Point::new(x * k, y * k);
        let valeurs: Vec<f64> = self
            .series
            .iter()
            .flat_map(|s| s.valeurs.iter().flatten().copied())
            .collect();
        let max = plafond(valeurs.iter().fold(0.0_f64, |a, &b| a.max(b)) * 1.2);
        let (x0, x1, yb, yh) = (60.0, 632.0, 170.0, 10.0);
        let nb = self
            .series
            .iter()
            .map(|s| s.valeurs.len())
            .max()
            .unwrap_or(0)
            .saturating_sub(1)
            .max(1) as f32;
        let x = |i: usize| x0 + i as f32 * (x1 - x0) / nb;
        let y = |v: f64| yb - (v.min(max) / max) as f32 * (yb - yh);

        for niveau in [0.0, max / 2.0, max] {
            let yy = y(niveau);
            cadre.stroke(
                &Path::line(p(x0, yy), p(636.0, yy)),
                trait_plein(theme::BORD, 1.0),
            );
            cadre.fill_text(Text {
                content: if niveau == 0.0 {
                    "0".into()
                } else {
                    graduation(niveau)
                },
                position: p(52.0, yy),
                color: theme::DISCRET,
                size: (12.0 * k).into(),
                font: INTER,
                align_x: Alignment::Right,
                align_y: Vertical::Center,
                ..Text::default()
            });
        }
        if self.seuil_lent < max {
            let ys = y(self.seuil_lent);
            cadre.stroke(
                &Path::line(p(x0, ys), p(636.0, ys)),
                Stroke {
                    line_dash: LineDash {
                        segments: &[6.0 * k, 5.0 * k],
                        offset: 0,
                    },
                    line_cap: LineCap::Butt,
                    ..trait_plein(theme::WARN_BORD, 1.5)
                },
            );
            cadre.fill_text(Text {
                content: format!("seuil « lent » {}", graduation(self.seuil_lent)),
                position: p(x0 + 4.0, ys - 3.0),
                color: theme::WARN,
                size: (12.0 * k).into(),
                font: INTER,
                align_y: Vertical::Bottom,
                ..Text::default()
            });
        }
        for serie in &self.series {
            for mut seg in segments(&serie.valeurs, |i, v| p(x(i), y(v))) {
                if seg.len() == 1 {
                    seg.push(Point::new(seg[0].x + 2.0 * k, seg[0].y));
                }
                cadre.stroke(&polyligne(&seg), trait_plein(serie.couleur, 2.0));
            }
            let derniere = serie.valeurs.iter().rposition(Option::is_some);
            if let (true, Some(i)) = (serie.hors_ligne, derniere) {
                let centre = p(x(i), y(serie.valeurs[i].unwrap_or_default()));
                let cercle = Path::circle(centre, 7.0 * k);
                cadre.fill(&cercle, theme::DOWN_CARTE);
                cadre.stroke(&cercle, trait_plein(theme::DOWN, 2.0));
                let d = 3.5 * k;
                for (a, b) in [((-d, -d), (d, d)), ((d, -d), (-d, d))] {
                    let ligne = Path::line(
                        Point::new(centre.x + a.0, centre.y + a.1),
                        Point::new(centre.x + b.0, centre.y + b.1),
                    );
                    cadre.stroke(&ligne, trait_plein(theme::DOWN, 2.0));
                }
            }
        }
        vec![cadre.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graduations() {
        assert_eq!(graduation(500.0), "500 ms");
        assert_eq!(graduation(12.5), "12,5 ms");
        assert_eq!(graduation(2500.0), "2,5 s");
        assert_eq!(graduation(10000.0), "10 s");
    }

    #[test]
    fn les_trous_coupent_la_courbe() {
        let segs = segments(&[Some(1.0), Some(2.0), None, None, Some(3.0)], |i, v| {
            Point::new(i as f32, v as f32)
        });
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[1], vec![Point::new(4.0, 3.0)]);
    }
}
