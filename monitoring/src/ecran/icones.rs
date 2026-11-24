//! Icones au trait, dessinees dans une grille de 24 x 24 puis mises a l'echelle.

use std::f32::consts::PI;

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke, path::Builder};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::api::Niveau;

use super::theme;

#[derive(Clone, Copy, Debug)]
pub enum Icone {
    Ok,
    Warn,
    WarnPetit,
    Down,
    Attente,
    Accueil,
    Tailscale,
    Cloche,
    Lune,
    Pouls,
    Gauche,
    Droite,
    Relancer,
    Info,
    Serveur,
    Tablette,
    Ordi,
    Tel,
    Maj,
}

/// Icone de `taille` pixels, trait de `epaisseur` (unites de la grille 24 x 24).
pub fn icone<'a, M: 'a>(
    icone: Icone,
    taille: f32,
    epaisseur: f32,
    couleur: Color,
) -> Element<'a, M> {
    iced::widget::canvas(Dessin {
        icone,
        epaisseur,
        couleur,
    })
    .width(taille)
    .height(taille)
    .into()
}

/// Icone d'etat : coche, triangle, croix ou points de suspension.
pub fn icone_etat<'a, M: 'a>(niveau: Niveau, taille: f32) -> Element<'a, M> {
    let grand = taille > 20.0;
    let (forme, epaisseur) = match niveau {
        Niveau::Up => (Icone::Ok, if grand { 2.4 } else { 3.0 }),
        Niveau::Warn | Niveau::Lent if grand => (Icone::Warn, 2.2),
        Niveau::Warn | Niveau::Lent => (Icone::WarnPetit, 2.6),
        Niveau::Down => (Icone::Down, if grand { 2.4 } else { 3.0 }),
        Niveau::Inconnu | Niveau::Vide => (Icone::Attente, 3.5),
    };
    icone(forme, taille, epaisseur, theme::couleur(niveau))
}

struct Dessin {
    icone: Icone,
    epaisseur: f32,
    couleur: Color,
}

impl<M> canvas::Program<M> for Dessin {
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
        let k = limites.width.min(limites.height) / 24.0;
        let chemin = Path::new(|b| tracer(self.icone, &mut Trace { b, k }));
        cadre.stroke(
            &chemin,
            Stroke::default()
                .with_color(self.couleur)
                .with_width(self.epaisseur * k)
                .with_line_cap(LineCap::Round)
                .with_line_join(LineJoin::Round),
        );
        vec![cadre.into_geometry()]
    }
}

/// Constructeur de chemin dans la grille 24 x 24 (comme les chemins SVG d'origine).
struct Trace<'b> {
    b: &'b mut Builder,
    k: f32,
}

impl Trace<'_> {
    fn p(&self, x: f32, y: f32) -> Point {
        Point::new(x * self.k, y * self.k)
    }

    fn ligne(&mut self, points: &[(f32, f32)]) {
        for (i, &(x, y)) in points.iter().enumerate() {
            let p = self.p(x, y);
            if i == 0 {
                self.b.move_to(p)
            } else {
                self.b.line_to(p)
            }
        }
    }

    fn polygone(&mut self, points: &[(f32, f32)]) {
        self.ligne(points);
        self.b.close();
    }

    /// Point visible (trait minuscule a bouts ronds).
    fn point(&mut self, x: f32, y: f32) {
        self.ligne(&[(x, y), (x + 0.01, y)]);
    }

    fn cercle(&mut self, x: f32, y: f32, r: f32) {
        self.b.circle(self.p(x, y), r * self.k);
    }

    fn rectangle(&mut self, x: f32, y: f32, l: f32, h: f32, r: f32) {
        self.b.rounded_rectangle(
            self.p(x, y),
            Size::new(l * self.k, h * self.k),
            (r * self.k).into(),
        );
    }

    /// Arc de cercle a la maniere de la commande `A` du SVG, depuis le point courant `de`.
    fn arc(
        &mut self,
        de: (f32, f32),
        vers: (f32, f32),
        rayon: f32,
        grand: bool,
        sens_horaire: bool,
    ) {
        for (x, y) in points_arc(de, vers, rayon, grand, sens_horaire) {
            self.b.line_to(self.p(x, y));
        }
    }
}

/// Points d'un arc SVG de rayon constant (conversion en parametrage par le centre).
fn points_arc(
    de: (f32, f32),
    vers: (f32, f32),
    rayon: f32,
    grand: bool,
    sens_horaire: bool,
) -> Vec<(f32, f32)> {
    let (x1p, y1p) = ((de.0 - vers.0) / 2.0, (de.1 - vers.1) / 2.0);
    let d2 = x1p * x1p + y1p * y1p;
    let r = rayon.max(d2.sqrt());
    let coef = ((r * r - d2) / d2).max(0.0).sqrt() * if grand == sens_horaire { -1.0 } else { 1.0 };
    let (cxp, cyp) = (coef * y1p, -coef * x1p);
    let (cx, cy) = (cxp + (de.0 + vers.0) / 2.0, cyp + (de.1 + vers.1) / 2.0);
    let debut = (y1p - cyp).atan2(x1p - cxp);
    let mut ecart = (-y1p - cyp).atan2(-x1p - cxp) - debut;
    if sens_horaire && ecart < 0.0 {
        ecart += 2.0 * PI;
    } else if !sens_horaire && ecart > 0.0 {
        ecart -= 2.0 * PI;
    }
    const PAS: usize = 24;
    (1..=PAS)
        .map(|i| {
            let angle = debut + ecart * i as f32 / PAS as f32;
            (cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect()
}

fn tracer(icone: Icone, t: &mut Trace) {
    match icone {
        Icone::Ok => t.ligne(&[(5.0, 12.5), (9.5, 17.0), (19.0, 7.5)]),
        Icone::Warn => {
            t.polygone(&[(12.0, 3.5), (22.0, 20.5), (2.0, 20.5)]);
            t.ligne(&[(12.0, 10.0), (12.0, 14.5)]);
            t.point(12.0, 17.5);
        }
        Icone::WarnPetit => {
            t.polygone(&[(12.0, 3.5), (22.0, 20.5), (2.0, 20.5)]);
            t.ligne(&[(12.0, 10.0), (12.0, 14.0)]);
        }
        Icone::Down => {
            t.ligne(&[(6.0, 6.0), (18.0, 18.0)]);
            t.ligne(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icone::Attente => {
            t.point(6.0, 12.0);
            t.point(12.0, 12.0);
            t.point(18.0, 12.0);
        }
        Icone::Accueil => {
            for (x, y) in [(3.5, 3.5), (13.5, 3.5), (3.5, 13.5), (13.5, 13.5)] {
                t.rectangle(x, y, 7.0, 7.0, 1.5);
            }
        }
        Icone::Tailscale => {
            t.cercle(6.0, 12.0, 2.5);
            t.cercle(18.0, 6.0, 2.5);
            t.cercle(18.0, 18.0, 2.5);
            t.ligne(&[(8.2, 10.9), (15.8, 7.1)]);
            t.ligne(&[(8.2, 13.1), (15.8, 16.9)]);
        }
        Icone::Cloche => {
            t.ligne(&[(6.0, 16.0), (6.0, 11.0)]);
            t.arc((6.0, 11.0), (18.0, 11.0), 6.0, false, true);
            t.ligne(&[
                (18.0, 11.0),
                (18.0, 16.0),
                (19.5, 18.0),
                (4.5, 18.0),
                (6.0, 16.0),
            ]);
            t.ligne(&[(10.0, 20.5)]);
            t.arc((10.0, 20.5), (14.0, 20.5), 2.0, false, false);
        }
        Icone::Lune => {
            t.ligne(&[(20.0, 14.5)]);
            t.arc((20.0, 14.5), (9.5, 4.0), 8.0, false, true);
            t.arc((9.5, 4.0), (20.0, 14.5), 8.0, true, false);
        }
        Icone::Pouls => t.ligne(&[
            (3.0, 12.0),
            (7.0, 12.0),
            (9.5, 6.0),
            (14.5, 18.0),
            (17.0, 12.0),
            (21.0, 12.0),
        ]),
        Icone::Gauche => t.ligne(&[(15.0, 6.0), (9.0, 12.0), (15.0, 18.0)]),
        Icone::Droite => t.ligne(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
        Icone::Relancer => {
            t.ligne(&[(20.0, 11.0)]);
            t.arc((20.0, 11.0), (17.7, 16.7), 8.0, true, false);
            t.ligne(&[(20.0, 5.0), (20.0, 11.0), (14.0, 11.0)]);
        }
        Icone::Info => {
            t.cercle(12.0, 12.0, 9.0);
            t.ligne(&[(12.0, 11.0), (12.0, 16.0)]);
            t.point(12.0, 8.0);
        }
        Icone::Serveur => {
            t.rectangle(3.5, 4.0, 17.0, 7.0, 1.5);
            t.rectangle(3.5, 13.0, 17.0, 7.0, 1.5);
            t.point(7.5, 7.5);
            t.point(7.5, 16.5);
        }
        Icone::Tablette => {
            t.rectangle(4.5, 3.0, 15.0, 18.0, 2.0);
            t.ligne(&[(11.0, 18.0), (13.0, 18.0)]);
        }
        Icone::Ordi => {
            t.rectangle(3.0, 4.0, 18.0, 12.0, 1.5);
            t.ligne(&[(8.0, 20.0), (16.0, 20.0)]);
            t.ligne(&[(12.0, 16.0), (12.0, 20.0)]);
        }
        Icone::Tel => {
            t.rectangle(7.0, 2.5, 10.0, 19.0, 2.0);
            t.ligne(&[(11.0, 18.5), (13.0, 18.5)]);
        }
        Icone::Maj => {
            t.ligne(&[(12.0, 3.5), (12.0, 14.5)]);
            t.ligne(&[(7.5, 10.0), (12.0, 14.5), (16.5, 10.0)]);
            t.ligne(&[(4.5, 19.5), (19.5, 19.5)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::points_arc;

    fn proche(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }

    #[test]
    fn arc_superieur_de_la_cloche() {
        let points = points_arc((6.0, 11.0), (18.0, 11.0), 6.0, false, true);
        assert!(proche(points[11], (12.0, 5.0)), "{:?}", points[11]);
        assert!(proche(*points.last().unwrap(), (18.0, 11.0)));
    }

    #[test]
    fn arc_inferieur_du_battant() {
        let points = points_arc((10.0, 20.5), (14.0, 20.5), 2.0, false, false);
        assert!(proche(points[11], (12.0, 22.5)), "{:?}", points[11]);
    }
}
