//! Accueil : etat general, puis une tuile par service, groupees par tag.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, span};
use iced::{Alignment, Element, Length, Theme};

use crate::api::{Niveau, Service};

use super::Contexte;
use crate::ecran::Message;
use crate::ecran::commun::{
    self, Vue, badge, defilant, ecran, entete, horloge, pilule_tailscale, titres,
};
use crate::ecran::format::{duree, il_y_a, ms, pct, pluriel};
use crate::ecran::graphiques::sparkline;
use crate::ecran::icones::{Icone, icone, icone_etat};
use crate::ecran::logique::{
    self, Page, Probleme, compte_nodes, groupes, niveau_groupe, problemes,
};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, MOYEN, t, texte_riche};

const COLONNES: usize = 4;

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let nodes: Vec<_> = e.services.iter().flat_map(|s| &s.nodes).collect();
    let nodes_ok = nodes
        .iter()
        .filter(|n| matches!(n.etat, Niveau::Up | Niveau::Lent))
        .count();
    let services_ok = e.services.iter().filter(|s| s.etat == Niveau::Up).count();
    let sous = format!(
        "{services_ok}/{} services opérationnels · {nodes_ok}/{} nodes en ligne · contrôle toutes les {} s",
        e.services.len(),
        nodes.len(),
        e.intervalle
    );
    let tete = entete(vec![
        titres(
            "Vue d'ensemble",
            None,
            commun::sous_titre(t(sous, 14).color(theme::DISCRET).into()),
        ),
        pilule_tailscale(e, c.maintenant),
        horloge(),
    ]);
    let contenu: Vue<'a> = if e.services.is_empty() {
        commun::vide(texte_riche(vec![
            span("Aucun service pour l'instant. Ouvre "),
            span("/root/monitoring/config.toml")
                .font(theme::MONO)
                .color(theme::TEXTE2),
            span(" en SSH (port 2222), ajoute tes services, puis lance "),
            span("kiosk-restart").font(theme::MONO).color(theme::TEXTE2),
            span("."),
        ]))
    } else {
        let gs = groupes(&e.services);
        let avec_titres = gs.len() > 1 || gs.first().is_some_and(|g| !g.tag.is_empty());
        let blocs = gs.into_iter().map(|g| {
            let mut bloc = column![].spacing(9);
            if avec_titres {
                bloc = bloc.push(entete_groupe(
                    if g.tag.is_empty() { "Autres" } else { g.tag },
                    &g.services,
                ));
            }
            bloc.push(grille(&g.services, c.maintenant)).into()
        });
        defilant(
            column(blocs)
                .spacing(16)
                .padding(iced::Padding::ZERO.bottom(4)),
        )
        .height(Length::Fill)
        .into()
    };
    ecran(
        &Page::Accueil,
        e,
        column![tete, bandeau(c), contenu].spacing(14),
    )
}

fn bandeau<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    if e.services.is_empty() {
        return cadre_bandeau(
            Niveau::Inconnu,
            icone(Icone::Info, 26.0, 2.0, theme::ACCENT),
            "Aucun service configuré".into(),
            "Ajoute tes services dans /root/monitoring/config.toml, puis lance kiosk-restart."
                .into(),
            None,
        );
    }
    let pbs = problemes(e);
    match pbs.as_slice() {
        [] if e.services.iter().all(|s| s.etat == Niveau::Inconnu) => cadre_bandeau(
            Niveau::Inconnu,
            icone_etat(Niveau::Inconnu, 22.0),
            "Premiers contrôles en cours…".into(),
            "Les résultats arrivent dans quelques secondes.".into(),
            None,
        ),
        [] => {
            let nodes: Vec<_> = e.services.iter().flat_map(|s| &s.nodes).collect();
            let dernier = nodes.iter().filter_map(|n| n.dernier).max();
            cadre_bandeau(
                Niveau::Up,
                icone_etat(Niveau::Up, 22.0),
                "Tout est opérationnel".into(),
                format!(
                    "{} et {} répondent normalement · dernier contrôle {}",
                    pluriel(e.services.len(), "service"),
                    pluriel(nodes.len(), "node"),
                    il_y_a(dernier, c.maintenant)
                ),
                None,
            )
        }
        [p] => cadre_bandeau(
            p.niveau,
            icone_etat(p.niveau, 22.0),
            p.titre.clone(),
            format!(
                "{} · depuis {}",
                p.detail,
                duree((c.maintenant - p.depuis) as f64)
            ),
            Some(p.page.clone()),
        ),
        plusieurs => {
            let niveau = logique::pire(plusieurs.iter().map(|p| p.niveau));
            let courts: Vec<&str> = plusieurs
                .iter()
                .map(|p: &Probleme| p.court.as_str())
                .collect();
            cadre_bandeau(
                niveau,
                icone_etat(niveau, 22.0),
                format!("{} problèmes en cours", plusieurs.len()),
                courts.join(" · "),
                Some(Page::Incidents),
            )
        }
    }
}

fn cadre_bandeau<'a>(
    niveau: Niveau,
    dessin: Vue<'a>,
    titre: String,
    detail: String,
    lien: Option<Page>,
) -> Vue<'a> {
    let (fond, bord, fond_icone) = match niveau {
        Niveau::Up => (theme::OK_CARTE, theme::OK_BORD, theme::UP_FOND),
        Niveau::Warn | Niveau::Lent => (theme::WARN_CARTE, theme::WARN_BORD, theme::WARN_FOND),
        Niveau::Down => (theme::DOWN_CARTE, theme::DOWN_BORD, theme::DOWN_FOND),
        Niveau::Inconnu | Niveau::Vide => (theme::SURFACE, theme::BORD, theme::SURFACE2),
    };
    let couleur_titre = if matches!(niveau, Niveau::Inconnu | Niveau::Vide) {
        theme::TEXTE
    } else {
        theme::couleur(niveau)
    };
    let rond = container(dessin)
        .center(38)
        .style(move |_: &Theme| theme::boite(fond_icone, None, 19.0));
    let textes = row![
        t(titre, 18)
            .font(GRAS)
            .color(couleur_titre)
            .wrapping(Wrapping::None),
        container(t(detail, 14).color(theme::TEXTE2).wrapping(Wrapping::None))
            .clip(true)
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let mut ligne = row![rond, container(textes).width(Length::Fill).clip(true)]
        .spacing(14)
        .align_y(Alignment::Center);
    if let Some(page) = lien {
        let voir = button(commun::au_centre(
            row![
                t("Voir le détail", 15).font(DEMI_GRAS),
                icone(Icone::Droite, 20.0, 2.2, theme::FOND)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ))
        .height(40)
        .padding([0, 16])
        .style(theme::touchable(
            Some(theme::couleur(niveau)),
            None,
            12.0,
            theme::FOND,
        ))
        .on_press(Message::Aller(page));
        ligne = ligne.push(voir);
    }
    container(ligne)
        .padding([10, 16])
        .style(move |_: &Theme| theme::boite(fond, Some(bord), 16.0))
        .into()
}

fn entete_groupe<'a>(nom: &str, services: &[&Service]) -> Vue<'a> {
    let niveau = niveau_groupe(services);
    let hs = services.iter().filter(|s| s.etat == Niveau::Down).count();
    let degrades = services.iter().filter(|s| s.etat == Niveau::Warn).count();
    let mut resume = row![].spacing(0);
    let mut morceaux: Vec<(String, iced::Color)> = Vec::new();
    if hs > 0 {
        morceaux.push((format!("{hs} hors ligne"), theme::DOWN));
    }
    if degrades > 0 {
        morceaux.push((pluriel(degrades, "dégradé"), theme::WARN));
    }
    if morceaux.is_empty() {
        resume = resume.push(t(pluriel(services.len(), "service"), 13).color(theme::DISCRET));
    } else {
        morceaux.push((format!("{} au total", services.len()), theme::DISCRET));
        for (i, (texte, couleur)) in morceaux.into_iter().enumerate() {
            if i > 0 {
                resume = resume.push(t(" · ", 13).color(theme::DISCRET));
            }
            resume = resume.push(t(texte, 13).color(couleur));
        }
    }
    let pastille = container(Space::new())
        .width(8)
        .height(8)
        .style(move |_: &Theme| {
            let rayon = if niveau == Niveau::Warn { 2.0 } else { 4.0 };
            theme::boite(theme::couleur(niveau), None, rayon)
        });
    let trait_ = container(Space::new().width(Length::Fill).height(1))
        .style(|_: &Theme| theme::boite(theme::BORD2, None, 0.0));
    row![
        pastille,
        t(nom.to_uppercase(), 13).font(GRAS).color(theme::TEXTE2),
        resume,
        trait_
    ]
    .spacing(10)
    .height(20)
    .align_y(Alignment::Center)
    .into()
}

fn grille<'a>(services: &[&Service], maintenant: i64) -> Vue<'a> {
    let lignes = services.chunks(COLONNES).map(|ligne| {
        let mut cases: Vec<Vue<'a>> = ligne.iter().map(|s| tuile(s, maintenant)).collect();
        while cases.len() < COLONNES {
            cases.push(Space::new().width(Length::Fill).into());
        }
        row(cases).spacing(12).into()
    });
    column(lignes).spacing(12).into()
}

fn tuile<'a>(s: &Service, maintenant: i64) -> Vue<'a> {
    let k = s.etat;
    let latence = match k {
        Niveau::Down => "Hors ligne".to_string(),
        Niveau::Inconnu => "…".to_string(),
        _ => ms(s.ms),
    };
    let note = match k {
        Niveau::Up => s.dispo30.map(|d| pct(Some(d))).unwrap_or_default(),
        Niveau::Inconnu => "en attente".into(),
        _ => format!("depuis {}", duree((maintenant - s.depuis) as f64)),
    };
    let couleur_latence = match k {
        Niveau::Warn | Niveau::Down => theme::couleur(k),
        Niveau::Inconnu => theme::DISCRET,
        _ => theme::TEXTE2,
    };
    let couleur_note = if matches!(k, Niveau::Warn | Niveau::Down) {
        theme::couleur(k)
    } else {
        theme::DISCRET
    };
    let taille_nom = if s.nom.chars().count() > 14 {
        14.5
    } else {
        16.0
    };
    let haut = row![
        badge(k, 24.0, 16.0),
        container(
            t(s.nom.clone(), taille_nom)
                .font(DEMI_GRAS)
                .wrapping(Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        t(latence, 14)
            .font(DEMI_GRAS)
            .color(couleur_latence)
            .wrapping(Wrapping::None),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let hote = container(
        t(s.adresse.clone(), 12)
            .font(theme::MONO)
            .color(theme::DISCRET)
            .wrapping(Wrapping::None),
    )
    .clip(true)
    .width(Length::Fill)
    .height(15);
    let mut bas = row![].spacing(5).align_y(Alignment::Center);
    for n in &s.nodes {
        bas = bas.push(carre_node(n.etat));
    }
    let bas = bas
        .push(
            container(
                t(compte_nodes(s), 12.5)
                    .color(theme::DISCRET)
                    .wrapping(Wrapping::None),
            )
            .padding(iced::Padding::ZERO.left(4)),
        )
        .push(
            container(sparkline(&s.spark, k))
                .padding(iced::Padding::ZERO.left(6).right(8))
                .width(Length::Fill),
        )
        .push(
            t(note, 12.5)
                .font(MOYEN)
                .color(couleur_note)
                .wrapping(Wrapping::None),
        );
    let (fond, bord) = theme::carte_etat(k).unwrap_or((theme::SURFACE, theme::BORD));
    let contenu: Element<'a, Message> = column![haut, hote, bas].spacing(6).into();
    button(contenu)
        .width(Length::Fill)
        .padding([12, 15])
        .style(theme::touchable(Some(fond), Some(bord), 14.0, theme::TEXTE))
        .on_press(Message::Aller(Page::Service(s.id.clone())))
        .into()
}

/// Petit carre d'une node : plein (en ligne), orange (lent), creux rouge (hors ligne), gris.
fn carre_node<'a>(etat: Niveau) -> Vue<'a> {
    container(Space::new())
        .width(12)
        .height(12)
        .style(move |_: &Theme| match etat {
            Niveau::Down => iced::widget::container::Style {
                border: iced::Border {
                    color: theme::DOWN,
                    width: 2.0,
                    radius: 3.0.into(),
                },
                ..Default::default()
            },
            Niveau::Lent => theme::boite(theme::WARN, None, 3.0),
            Niveau::Inconnu | Niveau::Vide => theme::boite(theme::GRIS_NODE, None, 3.0),
            _ => theme::boite(theme::UP, None, 3.0),
        })
        .into()
}
