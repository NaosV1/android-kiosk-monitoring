//! Tailscale : etat de la tablette sur le tailnet et appareils (avec une cle API).

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, span};
use iced::{Alignment, Color, Length, Theme};

use crate::api::{Appareil, Niveau};

use super::Contexte;
use crate::ecran::commun::{self, Vue, alerte, defilant, ecran, entete, horloge, point, titres};
use crate::ecran::format::{duree, il_y_a};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::Page;
use crate::ecran::theme::{self, DEMI_GRAS, MONO, t, texte_riche};

/// Un appareil vu dans les dernieres 24 h puis disparu merite une alerte.
const RECENT: i64 = 86400;

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let ts = &c.etat.tailscale;
    let api = &ts.api;
    let ip = ts.tablette.ip.as_deref();
    let etiquette_etat = match (ip, ts.actif) {
        (Some(_), _) => commun::etiquette_texte(Some(Niveau::Up), true, "Connecté"),
        (None, true) => commun::etiquette_texte(Some(Niveau::Down), true, "Déconnecté"),
        (None, false) => {
            commun::etiquette_texte(Some(Niveau::Inconnu), true, "Tablette hors tailnet")
        }
    };
    let sous: Vue<'a> = match (&api.tailnet, api.configuree) {
        (Some(tailnet), _) => texte_riche(vec![
            span(tailnet.clone()).font(MONO),
            span(format!(
                " · liste mise à jour {}",
                il_y_a(api.maj, c.maintenant)
            )),
        ])
        .size(14)
        .color(theme::DISCRET)
        .into(),
        (None, true) => t("Lecture de la liste des appareils…", 14)
            .color(theme::DISCRET)
            .into(),
        (None, false) => t("Liste des appareils : clé API non configurée", 14)
            .color(theme::DISCRET)
            .into(),
    };
    let tete = entete(vec![
        titres("Tailscale", Some(etiquette_etat), commun::sous_titre(sous)),
        horloge(),
    ]);

    let appareils: &[Appareil] = api.appareils.as_deref().unwrap_or_default();
    let recent = |a: &Appareil| !a.en_ligne && a.vu.is_some_and(|vu| c.maintenant - vu < RECENT);
    let moi = appareils.iter().find(|a| a.cette_tablette);
    let en_ligne = appareils.iter().filter(|a| a.en_ligne).count();
    let recents: Vec<&str> = appareils
        .iter()
        .filter(|a| recent(a))
        .map(|a| a.nom.as_str())
        .collect();
    let sorties: Vec<&str> = appareils
        .iter()
        .filter(|a| a.sortie)
        .map(|a| a.nom.as_str())
        .collect();

    let tablette = match (ip, ts.actif) {
        (Some(ip), _) => resume_sous(format!("{ip} · sur le tailnet"), theme::DISCRET),
        (None, true) => resume_sous("hors du tailnet".into(), theme::DOWN),
        (None, false) => resume_sous("Tailscale pas encore installé".into(), theme::DISCRET),
    };
    let (appareils_titre, appareils_sous) = match &api.appareils {
        None => (
            "—".to_string(),
            resume_sous("clé API nécessaire".into(), theme::DISCRET),
        ),
        Some(liste) => (
            format!("{en_ligne} / {} en ligne", liste.len()),
            if recents.is_empty() {
                resume_sous("aucune déconnexion récente".into(), theme::DISCRET)
            } else {
                resume_sous(format!("{} hors ligne", recents.join(", ")), theme::DOWN)
            },
        ),
    };
    let sortie = if ts.exit_node.actif {
        resume(
            "Exit node sur la tablette",
            t("Activé", 22).font(DEMI_GRAS).color(theme::WARN).into(),
            resume_sous(
                format!(
                    "Internet passe par l'entreprise depuis {}",
                    duree((c.maintenant - ts.exit_node.depuis) as f64)
                ),
                theme::WARN,
            ),
        )
    } else {
        let detail = if sorties.is_empty() {
            "Internet sort en direct".to_string()
        } else {
            format!("Internet en direct · disponible : {}", sorties.join(", "))
        };
        resume(
            "Exit node sur la tablette",
            fort("Non utilisé".into()),
            resume_sous(detail, theme::DISCRET),
        )
    };
    let cartes = row![
        resume(
            "Cette tablette",
            fort(moi.map_or("tablette".into(), |a| a.nom.clone())),
            tablette
        ),
        resume("Appareils", fort(appareils_titre), appareils_sous),
        resume(
            "Relais DERP de la tablette",
            fort(
                moi.and_then(|a| a.derp.clone())
                    .unwrap_or_else(|| "—".into())
            ),
            resume_sous("si pas de connexion directe".into(), theme::DISCRET),
        ),
        sortie,
    ]
    .spacing(14);

    let corps: Vue<'a> = if !api.configuree {
        aide()
    } else if api.appareils.is_none() {
        container(
            t("En attente de la première réponse de l'API Tailscale…", 16).color(theme::DISCRET),
        )
        .padding([16, 20])
        .width(Length::Fill)
        .style(theme::carte)
        .into()
    } else {
        tableau(appareils, c.maintenant)
    };
    let mut page = column![tete].spacing(18);
    if let Some(erreur) = &api.erreur {
        page = page.push(alerte(erreur));
    }
    ecran(&Page::Tailscale, c.etat, page.push(cartes).push(corps))
}

fn fort<'a>(texte: String) -> Vue<'a> {
    container(t(texte, 22).font(DEMI_GRAS).wrapping(Wrapping::None))
        .clip(true)
        .width(Length::Fill)
        .into()
}

fn resume_sous<'a>(texte: String, couleur: Color) -> Vue<'a> {
    container(t(texte, 13).color(couleur).wrapping(Wrapping::None))
        .clip(true)
        .width(Length::Fill)
        .into()
}

fn resume<'a>(libelle: &'a str, valeur: Vue<'a>, sous: Vue<'a>) -> Vue<'a> {
    container(column![t(libelle, 14).color(theme::DISCRET), valeur, sous].spacing(6))
        .padding([16, 20])
        .width(Length::FillPortion(1))
        .style(theme::carte)
        .into()
}

fn code<'a>(texte: &'a str) -> theme::Morceau<'a> {
    span(texte)
        .font(MONO)
        .color(theme::TEXTE)
        .size(14)
        .background(theme::CREUX)
        .padding([2, 6])
        .border(iced::border::rounded(6))
}

fn aide<'a>() -> Vue<'a> {
    let etapes = column![
        t("Afficher tous les appareils du tailnet", 17)
            .font(DEMI_GRAS)
            .color(theme::TEXTE),
        texte_riche(vec![
            span("1. Dans la console Tailscale, ouvre "),
            span("Settings › Keys").font(theme::GRAS),
            span(" et génère une clé API ("),
            code("tskey-api-…"),
            span(")."),
        ]),
        texte_riche(vec![
            span("2. En SSH sur la tablette (port 2222), ouvre "),
            code("/root/monitoring/config.toml"),
            span(" et colle-la dans "),
            code("cle_api = \"…\""),
            span(", section "),
            code("[tailscale]"),
            span("."),
        ]),
        texte_riche(vec![
            span("3. Lance "),
            code("kiosk-restart"),
            span(". La liste apparaît ici en moins d'une minute.")
        ]),
        t(
            "Une clé API expire au bout de 90 jours au maximum : il faudra la renouveler.",
            15
        )
        .color(theme::DISCRET),
    ]
    .spacing(10);
    container(etapes)
        .padding([16, 20])
        .width(Length::Fill)
        .style(|th: &Theme| iced::widget::container::Style {
            text_color: Some(theme::TEXTE2),
            ..theme::carte(th)
        })
        .into()
}

/// Ordre d'affichage : disparus recemment, cette tablette, en ligne, le reste.
fn trier(appareils: &[Appareil], maintenant: i64) -> Vec<&Appareil> {
    let rang = |a: &Appareil| {
        if !a.en_ligne && a.vu.is_some_and(|vu| maintenant - vu < RECENT) {
            0
        } else if a.cette_tablette {
            1
        } else if a.en_ligne {
            2
        } else {
            3
        }
    };
    let mut tri: Vec<&Appareil> = appareils.iter().collect();
    tri.sort_by(|a, b| {
        rang(a)
            .cmp(&rang(b))
            .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
    });
    tri
}

const COLONNES: [(&str, u16); 4] = [
    ("Appareil", 9),
    ("Adresse Tailscale", 4),
    ("Relais DERP préféré", 4),
    ("Dernière activité", 4),
];

fn cellule<'a>(contenu: impl Into<Vue<'a>>, portion: Option<u16>) -> Vue<'a> {
    let largeur = portion.map_or(Length::Fixed(150.0), Length::FillPortion);
    container(contenu)
        .padding([0, 18])
        .width(largeur)
        .clip(true)
        .into()
}

fn tableau<'a>(appareils: &'a [Appareil], maintenant: i64) -> Vue<'a> {
    let mut entete = row![].height(44).align_y(Alignment::Center);
    for (titre, portion) in COLONNES {
        entete = entete.push(cellule(
            t(titre, 13).font(theme::MOYEN).color(theme::DISCRET),
            Some(portion),
        ));
    }
    entete = entete.push(cellule(
        t("État", 13).font(theme::MOYEN).color(theme::DISCRET),
        None,
    ));
    let mut lignes = column![];
    for a in trier(appareils, maintenant) {
        lignes = lignes
            .push(super::service::separateur())
            .push(ligne(a, maintenant));
    }
    let contenu = column![entete, defilant(lignes).height(Length::Fill)];
    container(contenu)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::carte)
        .into()
}

fn icone_appareil(a: &Appareil) -> Icone {
    let os = a.os.to_lowercase();
    if a.cette_tablette {
        Icone::Tablette
    } else if os.contains("android") || os.contains("ios") {
        Icone::Tel
    } else if os.contains("windows") || os.contains("macos") {
        Icone::Ordi
    } else {
        Icone::Serveur
    }
}

fn puce<'a>(texte: &'a str, moi: bool) -> Vue<'a> {
    let (fond, couleur) = if moi {
        (theme::ACTIF, iced::color!(0x9CCBFF))
    } else {
        (theme::SURFACE2, theme::TEXTE2)
    };
    container(t(texte, 12).font(DEMI_GRAS).color(couleur))
        .padding([2, 8])
        .style(move |_: &Theme| theme::boite(fond, None, 8.0))
        .into()
}

fn ligne<'a>(a: &'a Appareil, maintenant: i64) -> Vue<'a> {
    let alerte = !a.en_ligne && a.vu.is_some_and(|vu| maintenant - vu < RECENT);
    let (fond_icone, couleur_icone) = if a.cette_tablette {
        (theme::ACTIF, theme::ACCENT)
    } else if alerte {
        (theme::DOWN_FOND, iced::color!(0xFF8A8A))
    } else {
        (theme::SURFACE2, theme::TEXTE2)
    };
    let pictogramme = container(icone(icone_appareil(a), 20.0, 1.8, couleur_icone))
        .center(34)
        .style(move |_: &Theme| theme::boite(fond_icone, None, 10.0));
    let mut nom = row![t(a.nom.as_str(), 15).font(DEMI_GRAS)]
        .spacing(6)
        .align_y(Alignment::Center);
    if a.cette_tablette {
        nom = nom.push(puce("cette tablette", true));
    }
    if a.sortie {
        nom = nom.push(puce("nœud de sortie", false));
    }
    if !a.sous_reseaux.is_empty() {
        nom = nom.push(puce("routeur de sous-réseau", false));
    }
    let os = if a.sous_reseaux.is_empty() {
        a.os.clone()
    } else {
        format!("{} · {}", a.os, a.sous_reseaux.join(", "))
    };
    let appareil = row![pictogramme, column![nom, t(os, 13).color(theme::DISCRET)]]
        .spacing(12)
        .align_y(Alignment::Center);
    let vu = if a.en_ligne {
        "maintenant".to_string()
    } else if a.vu.is_some() {
        il_y_a(a.vu, maintenant)
    } else {
        "—".into()
    };
    let (etat, niveau, fond, couleur) = if a.en_ligne {
        ("En ligne", Niveau::Up, theme::UP_FOND, theme::UP)
    } else if alerte {
        ("Hors ligne", Niveau::Down, theme::DOWN_FOND, theme::DOWN)
    } else {
        (
            "Hors ligne",
            Niveau::Inconnu,
            theme::SURFACE2,
            theme::DISCRET,
        )
    };
    let pastille = container(
        row![
            point(niveau, 10.0),
            t(etat, 14).font(DEMI_GRAS).color(couleur)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .height(30)
    .padding([0, 12])
    .center_y(30)
    .style(move |_: &Theme| theme::boite(fond, None, 15.0));
    let rangee = row![
        cellule(appareil, Some(COLONNES[0].1)),
        cellule(
            t(a.ip.as_str(), 14).font(MONO).color(theme::TEXTE2),
            Some(COLONNES[1].1)
        ),
        cellule(
            t(a.derp.clone().unwrap_or_else(|| "—".into()), 15).color(theme::TEXTE2),
            Some(COLONNES[2].1)
        ),
        cellule(
            t(vu, 15).color(if alerte { theme::DOWN } else { theme::DISCRET }),
            Some(COLONNES[3].1)
        ),
        cellule(pastille, None),
    ]
    .height(54)
    .align_y(Alignment::Center);
    let fond_ligne = if alerte {
        Some(iced::color!(0x1D1214))
    } else {
        None
    };
    container(rangee)
        .width(Length::Fill)
        .style(move |_: &Theme| match fond_ligne {
            Some(f) => theme::boite(f, None, 0.0),
            None => iced::widget::container::Style::default(),
        })
        .into()
}
