//! Tailscale : etat de la tablette sur le tailnet et appareils (avec une cle API).

use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, span};
use iced::{Alignment, Length, Theme};

use crate::api::{Appareil, Niveau};

use super::Contexte;
use crate::ecran::commun::{
    self, Valeur, Vue, alerte, carte, defilant, ecran, entete, horloge, pastille, point,
    separateur, separateur_en_retrait, titres, tuile,
};
use crate::ecran::format::{duree, il_y_a};
use crate::ecran::icones::{Icone, icone};
use crate::ecran::logique::Page;
use crate::ecran::theme::{self, DEMI_GRAS, MONO, t, texte_riche};

/// Un appareil vu dans les dernieres 24 h puis disparu merite une alerte.
const RECENT: i64 = 86400;
/// Debut du texte d'une ligne d'appareil (marge, pictogramme, espacement) : depart des filets.
const RETRAIT_LIGNE: f32 = 64.0;

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let ts = &c.etat.tailscale;
    let api = &ts.api;
    let ip = ts.tablette.ip.as_deref();
    let etat = match (ip, ts.actif) {
        (Some(_), _) => pastille(Some(Niveau::Up), "Connecté"),
        (None, true) => pastille(Some(Niveau::Down), "Déconnecté"),
        (None, false) => pastille(Some(Niveau::Inconnu), "Tablette hors tailnet"),
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
        .color(theme::SECONDAIRE)
        .into(),
        (None, true) => t("Lecture de la liste des appareils…", 14)
            .color(theme::SECONDAIRE)
            .into(),
        (None, false) => t("Liste des appareils : clé API non configurée", 14)
            .color(theme::SECONDAIRE)
            .into(),
    };
    let tete = entete(vec![
        titres("Tailscale", Some(etat), commun::sous_titre(sous)),
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
    let rouge = theme::ton(Niveau::Down).texte;

    let tablette = match (ip, ts.actif) {
        (Some(ip), _) => tuile(
            "Cette tablette",
            Valeur::texte(moi.map_or("tablette".into(), |a| a.nom.clone())),
            format!("{ip}, sur le tailnet"),
            None,
        ),
        (None, true) => tuile(
            "Cette tablette",
            Valeur::texte("Hors du tailnet").couleur(rouge),
            format!(
                "depuis {}",
                duree((c.maintenant - ts.tablette.depuis) as f64)
            ),
            None,
        ),
        (None, false) => tuile(
            "Cette tablette",
            Valeur::texte("—"),
            "Tailscale pas encore installé".into(),
            None,
        ),
    };
    let liste_appareils = match &api.appareils {
        None => tuile(
            "Appareils en ligne",
            Valeur::texte("—"),
            "clé API nécessaire".into(),
            None,
        ),
        Some(liste) => tuile(
            "Appareils en ligne",
            Valeur::avec_unite(en_ligne.to_string(), format!("sur {}", liste.len())).couleur(
                if recents.is_empty() {
                    theme::TEXTE
                } else {
                    rouge
                },
            ),
            if recents.is_empty() {
                "aucune déconnexion récente".into()
            } else {
                format!("hors ligne : {}", recents.join(", "))
            },
            None,
        ),
    };
    let derp = tuile(
        "Relais DERP de la tablette",
        Valeur::texte(
            moi.and_then(|a| a.derp.clone())
                .unwrap_or_else(|| "—".into()),
        ),
        "si pas de connexion directe".into(),
        None,
    );
    let sortie = if ts.exit_node.actif {
        tuile(
            "Exit node sur la tablette",
            Valeur::texte("Activé").couleur(theme::ton(Niveau::Warn).texte),
            format!(
                "Internet passe par l'entreprise depuis {}",
                duree((c.maintenant - ts.exit_node.depuis) as f64)
            ),
            None,
        )
    } else {
        tuile(
            "Exit node sur la tablette",
            Valeur::texte("Non utilisé"),
            if sorties.is_empty() {
                "Internet sort en direct".into()
            } else {
                format!("disponible : {}", sorties.join(", "))
            },
            None,
        )
    };
    let tuiles = row![tablette, liste_appareils, derp, sortie].spacing(14);

    let corps: Vue<'a> = if !api.configuree {
        aide()
    } else if api.appareils.is_none() {
        carte(
            t("En attente de la première réponse de l'API Tailscale…", 15).color(theme::SECONDAIRE),
        )
        .into()
    } else {
        tableau(appareils, c.maintenant)
    };
    let mut page = column![tete].spacing(18);
    if let Some(erreur) = &api.erreur {
        page = page.push(alerte(erreur));
    }
    let defile = defilant(
        column![tuiles, corps]
            .spacing(18)
            .padding(iced::Padding::ZERO.bottom(8)),
    )
    .height(Length::Fill);
    ecran(
        &Page::Tailscale,
        c.etat,
        c.injoignable_depuis.is_some(),
        page.push(defile),
    )
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
            14
        )
        .color(theme::SECONDAIRE),
    ]
    .spacing(10);
    carte(etapes).into()
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
const LARGEUR_ETAT: f32 = 140.0;

fn cellule<'a>(contenu: impl Into<Vue<'a>>, portion: Option<u16>) -> Vue<'a> {
    let largeur = portion.map_or(Length::Fixed(LARGEUR_ETAT), Length::FillPortion);
    container(contenu)
        .padding([0, 16])
        .width(largeur)
        .clip(true)
        .into()
}

fn tableau<'a>(appareils: &'a [Appareil], maintenant: i64) -> Vue<'a> {
    let mut entete = row![].height(40).align_y(Alignment::Center);
    for (titre, portion) in COLONNES {
        entete = entete.push(cellule(
            t(titre, 13).font(theme::MOYEN).color(theme::SECONDAIRE),
            Some(portion),
        ));
    }
    entete = entete.push(cellule(
        t("État", 13).font(theme::MOYEN).color(theme::SECONDAIRE),
        None,
    ));
    let mut lignes = column![entete, separateur()];
    let tries = trier(appareils, maintenant);
    let nombre = tries.len();
    for (k, a) in tries.into_iter().enumerate() {
        if k > 0 {
            lignes = lignes.push(separateur_en_retrait(RETRAIT_LIGNE));
        }
        lignes = lignes.push(ligne(a, maintenant, k + 1 == nombre));
    }
    container(lignes)
        .width(Length::Fill)
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
        (theme::ACCENT_FOND, theme::ACCENT)
    } else {
        (theme::CREUX, theme::SECONDAIRE)
    };
    container(t(texte, 12).font(DEMI_GRAS).color(couleur))
        .padding([2, 8])
        .style(move |_: &Theme| theme::boite(fond, 8.0))
        .into()
}

fn ligne<'a>(a: &'a Appareil, maintenant: i64, derniere: bool) -> Vue<'a> {
    let alerte = !a.en_ligne && a.vu.is_some_and(|vu| maintenant - vu < RECENT);
    let rouge = theme::ton(Niveau::Down);
    let (fond_icone, couleur_icone) = if a.cette_tablette {
        (theme::ACCENT_FOND, theme::ACCENT)
    } else if alerte {
        (theme::SURFACE, rouge.texte)
    } else {
        (theme::CREUX, theme::SECONDAIRE)
    };
    let secondaire = if alerte {
        rouge.texte_fond
    } else {
        theme::SECONDAIRE
    };
    let pictogramme = container(icone(icone_appareil(a), 20.0, 1.8, couleur_icone))
        .center(36)
        .style(move |_: &Theme| theme::boite(fond_icone, 10.0));
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
        format!("{}, {}", a.os, a.sous_reseaux.join(", "))
    };
    let appareil = row![
        pictogramme,
        column![nom, t(os, 13).color(secondaire)].spacing(1)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let vu = if a.en_ligne {
        "maintenant".to_string()
    } else if a.vu.is_some() {
        il_y_a(a.vu, maintenant)
    } else {
        "—".into()
    };
    // la couleur ne signale qu'un probleme : « En ligne » reste neutre, son repere est vert
    let (etat, niveau, couleur_etat) = if a.en_ligne {
        ("En ligne", Niveau::Up, theme::TEXTE)
    } else if alerte {
        ("Hors ligne", Niveau::Down, rouge.texte)
    } else {
        ("Hors ligne", Niveau::Inconnu, theme::SECONDAIRE)
    };
    let rangee = row![
        cellule(appareil, Some(COLONNES[0].1)),
        cellule(
            commun::mono(a.ip.as_str(), 13.5, theme::TEXTE),
            Some(COLONNES[1].1)
        ),
        cellule(
            t(a.derp.clone().unwrap_or_else(|| "—".into()), 14).wrapping(Wrapping::None),
            Some(COLONNES[2].1)
        ),
        cellule(
            t(vu, 14)
                .color(if alerte { rouge.texte } else { secondaire })
                .wrapping(Wrapping::None),
            Some(COLONNES[3].1)
        ),
        cellule(
            row![
                point(niveau, 9.0),
                t(etat, 14).font(DEMI_GRAS).color(couleur_etat)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            None
        ),
    ]
    .height(58)
    .align_y(Alignment::Center);
    container(rangee)
        .width(Length::Fill)
        .style(move |_: &Theme| {
            theme::ligne_de_liste(
                if alerte { rouge.fond } else { theme::SURFACE },
                false,
                derniere,
            )
        })
        .into()
}
