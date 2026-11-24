//! Accueil : le verdict et trois chiffres sur un bandeau, puis une carte par service, groupees
//! par tag (les groupes qui contiennent un probleme en tete). Douze services sur trois tags
//! tiennent sur l'ecran sans defiler ; au-dela, la grille defile.

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, span};
use iced::{Alignment, Length, Theme};

use crate::api::{Etat, Niveau, Service};

use super::Contexte;
use crate::ecran::Message;
use crate::ecran::commun::{self, Valeur, Vue, defilant, ecran, entete, horloge, point, titres};
use crate::ecran::format::{self, abreger, duree, il_y_a, ms, pct, pluriel};
use crate::ecran::graphiques::sparkline;
use crate::ecran::icones::{Icone, icone, icone_etat};
use crate::ecran::logique::{self, Page, compte_nodes, groupes, problemes};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, t, texte_riche};

const COLONNES: usize = 4;
/// Caracteres d'un nom de service avant « … » (la latence tient a droite).
const NOM_MAX: usize = 16;
/// Ligne d'adresse d'une carte (maquette 1280 x 800) : largeur utile et chasse moyenne de ses
/// deux polices, pour abreger l'adresse avant qu'elle ne bute sur le nombre de nodes.
const LARGEUR_LIGNE_ADRESSE: f32 = 209.0;
const CHASSE_MONO_12: f32 = 7.2;
const CHASSE_INTER_12_5: f32 = 6.3;
/// Hauteur du bandeau : un verdict sur deux lignes y tient.
const HAUTEUR_BANDEAU: f32 = 76.0;
const LARGEUR_CHIFFRE: f32 = 164.0;

pub fn vue<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let nodes = e.services.iter().map(|s| s.nodes.len()).sum::<usize>();
    let services_ok = e.services.iter().filter(|s| s.etat == Niveau::Up).count();
    let nodes_ok = e
        .services
        .iter()
        .flat_map(|s| &s.nodes)
        .filter(|n| matches!(n.etat, Niveau::Up | Niveau::Lent))
        .count();
    // l'accord suit le premier nombre : « 1 service sur 7 opérationnel »
    let sous = format!(
        "{} sur {} opérationnel{}, {} sur {nodes} en ligne · contrôle toutes les {} s",
        pluriel(services_ok, "service"),
        e.services.len(),
        if services_ok > 1 { "s" } else { "" },
        pluriel(nodes_ok, "node"),
        e.intervalle
    );
    let tete = entete(vec![
        titres("Vue d'ensemble", None, commun::ligne_secondaire(sous, 14.0)),
        horloge(),
    ]);
    let contenu: Vue<'a> = if e.services.is_empty() {
        commun::vide(
            texte_riche(vec![
                span("Aucun service pour l'instant. Ouvre "),
                span("/root/monitoring/config.toml")
                    .font(theme::MONO)
                    .color(theme::TEXTE),
                span(" en SSH (port 2222), ajoute tes services, puis lance "),
                span("kiosk-restart").font(theme::MONO).color(theme::TEXTE),
                span("."),
            ])
            .size(15),
        )
    } else {
        // serveur injoignable : etat fige au dernier recu, affiche sans teinte d'etat
        let perime = c.injoignable_depuis.is_some();
        let instant = c.injoignable_depuis.unwrap_or(c.maintenant);
        let gs = groupes(&e.services);
        let avec_titres = gs.len() > 1 || gs.first().is_some_and(|g| !g.tag.is_empty());
        let blocs = gs.into_iter().map(|g| {
            let mut bloc = column![].spacing(8);
            if avec_titres {
                bloc = bloc.push(entete_groupe(
                    if g.tag.is_empty() { "Autres" } else { g.tag },
                    &g.services,
                    perime,
                ));
            }
            bloc.push(grille(&g.services, instant, perime)).into()
        });
        defilant(column(blocs).spacing(12))
            .height(Length::Fill)
            .into()
    };
    ecran(
        &Page::Accueil,
        e,
        c.injoignable_depuis.is_some(),
        column![
            tete,
            row![verdict(c), chiffres(c)]
                .spacing(12)
                .height(HAUTEUR_BANDEAU),
            contenu
        ]
        .spacing(12),
    )
}

/// Etat general : tout va bien, un probleme, plusieurs, ou affichage perime.
fn verdict<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    if let Some(recu) = c.injoignable_depuis {
        return cadre_verdict(
            Niveau::Inconnu,
            icone_etat(Niveau::Inconnu, 20.0),
            "État inconnu : le serveur ne répond plus".into(),
            format!(
                "Dernier état reçu à {} ; les services datent de ce moment.",
                format::heure(&format::local(recu))
            ),
            None,
        );
    }
    if e.services.is_empty() {
        return cadre_verdict(
            Niveau::Inconnu,
            icone(Icone::Info, 20.0, 2.0, theme::ACCENT),
            "Aucun service configuré".into(),
            "Ajoute tes services dans /root/monitoring/config.toml, puis lance kiosk-restart."
                .into(),
            None,
        );
    }
    let pbs = problemes(e);
    match pbs.as_slice() {
        [] if e.services.iter().all(|s| s.etat == Niveau::Inconnu) => cadre_verdict(
            Niveau::Inconnu,
            icone_etat(Niveau::Inconnu, 20.0),
            "Premiers contrôles en cours…".into(),
            "Les résultats arrivent dans quelques secondes.".into(),
            None,
        ),
        [] => {
            let nodes = e.services.iter().map(|s| s.nodes.len()).sum::<usize>();
            let dernier = e
                .services
                .iter()
                .flat_map(|s| &s.nodes)
                .filter_map(|n| n.dernier)
                .max();
            cadre_verdict(
                Niveau::Up,
                icone_etat(Niveau::Up, 20.0),
                "Tout est opérationnel".into(),
                format!(
                    "{} et {} répondent normalement, dernier contrôle {}",
                    pluriel(e.services.len(), "service"),
                    pluriel(nodes, "node"),
                    il_y_a(dernier, c.maintenant)
                ),
                None,
            )
        }
        [p] => cadre_verdict(
            p.niveau,
            icone_etat(p.niveau, 20.0),
            p.titre.clone(),
            format!(
                "{}, depuis {}",
                p.detail,
                duree((c.maintenant - p.depuis) as f64)
            ),
            Some(p.page.clone()),
        ),
        plusieurs => {
            let niveau = logique::pire(plusieurs.iter().map(|p| p.niveau));
            cadre_verdict(
                niveau,
                icone_etat(niveau, 20.0),
                format!("{} problèmes en cours", plusieurs.len()),
                plusieurs
                    .iter()
                    .map(|p| p.court.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                Some(Page::Incidents),
            )
        }
    }
}

fn cadre_verdict<'a>(
    niveau: Niveau,
    dessin: Vue<'a>,
    titre: String,
    detail: String,
    cible: Option<Page>,
) -> Vue<'a> {
    let ton = theme::ton(niveau);
    let (fond, fond_rond, couleur_titre, couleur_detail) = match niveau {
        Niveau::Inconnu | Niveau::Vide => (
            theme::SURFACE,
            theme::CREUX,
            theme::TEXTE,
            theme::SECONDAIRE,
        ),
        _ => (ton.fond, theme::SURFACE, ton.texte, ton.texte_fond),
    };
    let rond = container(dessin)
        .center(40)
        .style(move |_: &Theme| theme::boite(fond_rond, 20.0));
    // le detail passe a la ligne : le bandeau en garde deux
    let textes = column![
        t(titre, 18)
            .font(GRAS)
            .color(couleur_titre)
            .wrapping(Wrapping::None),
        t(detail, 14).color(couleur_detail),
    ]
    .spacing(2);
    let mut ligne = row![rond, container(textes).width(Length::Fill).clip(true)]
        .spacing(14)
        .align_y(Alignment::Center);
    // un probleme : tout le verdict ouvre son detail
    let Some(page) = cible else {
        return container(ligne)
            .center_y(Length::Fill)
            .width(Length::Fill)
            .padding([0, 18])
            .style(move |_: &Theme| theme::boite(fond, 16.0))
            .into();
    };
    ligne = ligne.push(icone(Icone::Droite, 18.0, 2.2, couleur_titre));
    button(container(ligne).center_y(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([0, 18])
        .style(theme::touchable(Some(fond), 16.0, couleur_titre))
        .on_press(Message::Aller(page))
        .into()
}

/// Disponibilite et temps de reponse moyens, Tailscale de la tablette.
fn chiffres<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let dispos: Vec<f64> = e.services.iter().filter_map(|s| s.dispo30).collect();
    let dispo = if dispos.is_empty() {
        Valeur::texte("—")
    } else {
        let moyenne = dispos.iter().sum::<f64>() / dispos.len() as f64;
        Valeur::avec_unite(pct(Some(moyenne)).trim_end_matches(" %").to_string(), "%")
    };
    let latences: Vec<f64> = e
        .services
        .iter()
        .filter(|s| matches!(s.etat, Niveau::Up | Niveau::Warn))
        .filter_map(|s| s.ms)
        .collect();
    let latence = if latences.is_empty() {
        Valeur::texte("—")
    } else {
        let moyenne = latences.iter().sum::<f64>() / latences.len() as f64;
        Valeur::avec_unite(ms(Some(moyenne)).trim_end_matches(" ms").to_string(), "ms")
    };
    row![
        chiffre("Disponibilité 30 j", dispo, None),
        chiffre("Réponse moyenne", latence, None),
        tailscale(e, c.injoignable_depuis.is_some()),
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}

/// Chiffre du bandeau : un libelle et une valeur, sans legende.
fn chiffre<'a>(libelle: &'a str, valeur: Valeur, message: Option<Message>) -> Vue<'a> {
    let contenu = container(
        column![
            t(libelle, 12.5)
                .font(theme::MOYEN)
                .color(theme::SECONDAIRE)
                .wrapping(Wrapping::None),
            commun::valeur_chiffree(valeur, 22.0),
        ]
        .spacing(3),
    )
    .center_y(Length::Fill);
    match message {
        Some(m) => button(contenu)
            .width(LARGEUR_CHIFFRE)
            .height(Length::Fill)
            .padding([0, 16])
            .style(theme::touchable(Some(theme::SURFACE), 16.0, theme::TEXTE))
            .on_press(m)
            .into(),
        None => container(contenu)
            .width(LARGEUR_CHIFFRE)
            .height(Length::Fill)
            .padding([0, 16])
            .style(theme::carte)
            .into(),
    }
}

/// `perime` : etat d'avant la perte du serveur, montre sans teinte.
fn tailscale<'a>(e: &'a Etat, perime: bool) -> Vue<'a> {
    let ts = &e.tailscale;
    let teinte = |niveau: Niveau| {
        if perime {
            theme::TEXTE
        } else {
            theme::ton(niveau).texte
        }
    };
    // la couleur n'est donnee qu'a un probleme ; « connecte » reste neutre
    let valeur = match (&ts.tablette.ip, ts.exit_node.actif, ts.actif) {
        (Some(_), true, _) => Valeur::texte("Exit node").couleur(teinte(Niveau::Warn)),
        (Some(_), false, _) => Valeur::texte("Connecté"),
        (None, _, true) => Valeur::texte("Déconnecté").couleur(teinte(Niveau::Down)),
        (None, _, false) => Valeur::texte("Hors tailnet"),
    };
    chiffre("Tailscale", valeur, Some(Message::Aller(Page::Tailscale)))
}

fn entete_groupe<'a>(nom: &str, services: &[&Service], perime: bool) -> Vue<'a> {
    let teinte = |niveau: Niveau| {
        if perime {
            theme::SECONDAIRE
        } else {
            theme::ton(niveau).texte
        }
    };
    let hs = services.iter().filter(|s| s.etat == Niveau::Down).count();
    let degrades = services.iter().filter(|s| s.etat == Niveau::Warn).count();
    let mut resume = row![].spacing(0);
    let mut problemes = Vec::new();
    if hs > 0 {
        problemes.push((format!("{hs} hors ligne"), teinte(Niveau::Down)));
    }
    if degrades > 0 {
        problemes.push((pluriel(degrades, "dégradé"), teinte(Niveau::Warn)));
    }
    for (i, (texte, couleur)) in problemes.iter().enumerate() {
        if i > 0 {
            resume = resume.push(t(", ", 14).color(theme::SECONDAIRE));
        }
        resume = resume.push(t(texte.clone(), 14).color(*couleur));
    }
    let total = pluriel(services.len(), "service");
    resume = resume.push(
        t(
            if problemes.is_empty() {
                total
            } else {
                format!(" sur {total}")
            },
            14,
        )
        .color(theme::SECONDAIRE),
    );
    row![t(nom.to_string(), 17).font(DEMI_GRAS), resume]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
}

fn grille<'a>(services: &[&Service], maintenant: i64, perime: bool) -> Vue<'a> {
    let lignes = services.chunks(COLONNES).map(|ligne| {
        let mut cases: Vec<Vue<'a>> = ligne
            .iter()
            .map(|s| carte_service(s, maintenant, perime))
            .collect();
        while cases.len() < COLONNES {
            cases.push(Space::new().width(Length::Fill).into());
        }
        row(cases).spacing(10).into()
    });
    column(lignes).spacing(10).into()
}

/// `perime` : etat d'avant la perte du serveur, montre sans teinte (le texte reste lisible).
fn carte_service<'a>(s: &Service, maintenant: i64, perime: bool) -> Vue<'a> {
    let k = s.etat;
    let teinte = if perime { Niveau::Inconnu } else { k };
    let ton = theme::ton(teinte);
    let alerte = matches!(teinte, Niveau::Warn | Niveau::Down);
    let (fond, secondaire) = if alerte {
        (ton.fond, ton.texte_fond)
    } else {
        (theme::SURFACE, theme::SECONDAIRE)
    };
    let (latence, couleur_latence) = match k {
        Niveau::Down => ("Hors ligne".to_string(), ton.texte),
        Niveau::Inconnu => ("…".to_string(), theme::SECONDAIRE),
        Niveau::Warn => (ms(s.ms), ton.texte),
        _ => (ms(s.ms), theme::TEXTE),
    };
    let (note, couleur_note) = match k {
        Niveau::Up => (
            s.dispo30
                .map(|d| format!("{} sur 30 j", pct(Some(d))))
                .unwrap_or_default(),
            secondaire,
        ),
        Niveau::Inconnu => ("en attente".into(), secondaire),
        _ => (
            format!("depuis {}", duree((maintenant - s.depuis) as f64)),
            ton.texte,
        ),
    };
    let haut = row![
        point(teinte, 9.0),
        container(
            t(abreger(&s.nom, NOM_MAX), 15)
                .font(DEMI_GRAS)
                .color(theme::TEXTE)
                .wrapping(Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        t(latence, 14)
            .font(DEMI_GRAS)
            .color(couleur_latence)
            .wrapping(Wrapping::None),
    ]
    .spacing(9)
    .align_y(Alignment::Center);
    let nodes = compte_nodes(s);
    let adresse_max = ((LARGEUR_LIGNE_ADRESSE - CHASSE_INTER_12_5 * nodes.chars().count() as f32)
        / CHASSE_MONO_12) as usize;
    let milieu = row![
        container(commun::mono(
            abreger(&s.adresse, adresse_max),
            12.0,
            secondaire
        ))
        .clip(true)
        .width(Length::Fill),
        t(nodes, 12.5).color(secondaire).wrapping(Wrapping::None),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let bas = row![
        sparkline(&s.spark, teinte),
        t(note, 12.5)
            .font(theme::MOYEN)
            .color(couleur_note)
            .wrapping(Wrapping::None),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let contenu = column![haut, milieu, bas].spacing(5);
    button(contenu)
        .width(Length::Fill)
        .padding([10, 13])
        .style(theme::touchable(Some(fond), 14.0, theme::TEXTE))
        .on_press(Message::Aller(Page::Service(s.id.clone())))
        .into()
}
