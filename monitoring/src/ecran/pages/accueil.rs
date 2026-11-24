//! Accueil : le verdict, quatre chiffres, puis une carte par service, groupees par tag
//! (les groupes qui contiennent un probleme en tete).

use iced::widget::text::Wrapping;
use iced::widget::{Space, button, column, container, row, span};
use iced::{Alignment, Length, Theme};

use crate::api::{Etat, Niveau, Service};

use super::Contexte;
use crate::ecran::Message;
use crate::ecran::commun::{
    self, Valeur, Vue, defilant, ecran, entete, horloge, lien, point, titres, tuile,
};
use crate::ecran::format::{self, abreger, duree, il_y_a, ms, pct, pluriel};
use crate::ecran::graphiques::sparkline;
use crate::ecran::icones::{Icone, icone, icone_etat};
use crate::ecran::logique::{self, Page, compte_nodes, groupes, problemes};
use crate::ecran::theme::{self, DEMI_GRAS, GRAS, t, texte_riche};

const COLONNES: usize = 3;
/// Caracteres d'un nom de service avant « … » (la latence tient a droite).
const NOM_MAX: usize = 20;

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
            let mut bloc = column![].spacing(10);
            if avec_titres {
                bloc = bloc.push(entete_groupe(
                    if g.tag.is_empty() { "Autres" } else { g.tag },
                    &g.services,
                    perime,
                ));
            }
            bloc.push(grille(&g.services, instant, perime)).into()
        });
        defilant(
            column(blocs)
                .spacing(22)
                .padding(iced::Padding::ZERO.bottom(8)),
        )
        .height(Length::Fill)
        .into()
    };
    ecran(
        &Page::Accueil,
        e,
        c.injoignable_depuis.is_some(),
        column![tete, verdict(c), chiffres(c), contenu].spacing(18),
    )
}

/// Etat general : tout va bien, un probleme, plusieurs, ou affichage perime.
fn verdict<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    if let Some(recu) = c.injoignable_depuis {
        return cadre_verdict(
            Niveau::Inconnu,
            icone_etat(Niveau::Inconnu, 22.0),
            "État inconnu : le serveur ne répond plus".into(),
            format!(
                "Dernier état reçu à {} ; les services affichés ci-dessous datent de ce moment.",
                format::heure(&format::local(recu))
            ),
            None,
        );
    }
    if e.services.is_empty() {
        return cadre_verdict(
            Niveau::Inconnu,
            icone(Icone::Info, 22.0, 2.0, theme::ACCENT),
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
            icone_etat(Niveau::Inconnu, 22.0),
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
                icone_etat(Niveau::Up, 22.0),
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
            icone_etat(p.niveau, 22.0),
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
                icone_etat(niveau, 22.0),
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
        .center(44)
        .style(move |_: &Theme| theme::boite(fond_rond, 22.0));
    let textes = column![
        t(titre, 22)
            .font(GRAS)
            .color(couleur_titre)
            .wrapping(Wrapping::None),
        commun::sous_titre(
            t(detail, 15)
                .color(couleur_detail)
                .wrapping(Wrapping::None)
                .into()
        ),
    ]
    .spacing(2);
    let mut ligne = row![rond, container(textes).width(Length::Fill).clip(true)]
        .spacing(16)
        .align_y(Alignment::Center);
    if let Some(page) = cible {
        ligne = ligne.push(lien("Voir le détail", couleur_titre, Message::Aller(page)));
    }
    container(ligne)
        .padding([14, 18])
        .width(Length::Fill)
        .style(move |_: &Theme| theme::boite(fond, 16.0))
        .into()
}

/// Disponibilite, temps de reponse, incidents en cours, Tailscale.
fn chiffres<'a>(c: &Contexte<'a>) -> Vue<'a> {
    let e = c.etat;
    let perime = c.injoignable_depuis.is_some();
    let dispos: Vec<f64> = e.services.iter().filter_map(|s| s.dispo30).collect();
    let dispo = if dispos.is_empty() {
        tuile(
            "Disponibilité sur 30 jours",
            Valeur::texte("—"),
            "pas encore assez d'historique".into(),
            None,
        )
    } else {
        let moyenne = dispos.iter().sum::<f64>() / dispos.len() as f64;
        tuile(
            "Disponibilité sur 30 jours",
            Valeur::avec_unite(pct(Some(moyenne)).trim_end_matches(" %").to_string(), "%"),
            format!("moyenne de {}", pluriel(dispos.len(), "service")),
            None,
        )
    };
    let latences: Vec<f64> = e
        .services
        .iter()
        .filter(|s| matches!(s.etat, Niveau::Up | Niveau::Warn))
        .filter_map(|s| s.ms)
        .collect();
    let latence = tuile(
        "Temps de réponse moyen",
        if latences.is_empty() {
            Valeur::texte("—")
        } else {
            let moyenne = latences.iter().sum::<f64>() / latences.len() as f64;
            Valeur::avec_unite(ms(Some(moyenne)).trim_end_matches(" ms").to_string(), "ms")
        },
        format!("lent au-delà de {}", ms(Some(e.seuil_lent as f64))),
        None,
    );
    let ouverts = &e.incidents_ouverts;
    let incidents = if ouverts.is_empty() {
        tuile(
            "Incidents en cours",
            Valeur::texte("0"),
            "rien à signaler".into(),
            Some(Message::Aller(Page::Incidents)),
        )
    } else {
        let niveau = logique::pire(ouverts.iter().map(|i| i.niveau));
        let non_vus = ouverts.iter().filter(|i| i.acquitte == 0).count();
        tuile(
            "Incidents en cours",
            Valeur::texte(ouverts.len().to_string()).couleur(if perime {
                theme::TEXTE
            } else {
                theme::ton(niveau).texte
            }),
            if non_vus > 0 {
                format!("{non_vus} à acquitter")
            } else {
                "tous acquittés".into()
            },
            Some(Message::Aller(Page::Incidents)),
        )
    };
    row![
        dispo,
        latence,
        incidents,
        tailscale(e, c.injoignable_depuis.unwrap_or(c.maintenant), perime)
    ]
    .spacing(14)
    .into()
}

/// `perime` : etat d'avant la perte du serveur, montre sans teinte et date de `maintenant`.
fn tailscale<'a>(e: &'a Etat, maintenant: i64, perime: bool) -> Vue<'a> {
    let ts = &e.tailscale;
    let teinte = |niveau: Niveau| {
        if perime {
            theme::TEXTE
        } else {
            theme::ton(niveau).texte
        }
    };
    // la couleur n'est donnee qu'a un probleme ; « connecte » reste neutre
    let (valeur, legende) = match (&ts.tablette.ip, ts.exit_node.actif, ts.actif) {
        (Some(ip), true, _) => (
            Valeur::texte("Exit node actif").couleur(teinte(Niveau::Warn)),
            format!("{ip}, Internet passe par le tunnel"),
        ),
        (Some(ip), false, _) => (Valeur::texte("Connecté"), ip.clone()),
        (None, _, true) => (
            Valeur::texte("Déconnecté").couleur(teinte(Niveau::Down)),
            format!("depuis {}", duree((maintenant - ts.tablette.depuis) as f64)),
        ),
        (None, _, false) => (
            Valeur::texte("Hors tailnet"),
            "surveillance désactivée".into(),
        ),
    };
    tuile(
        "Tailscale",
        valeur,
        legende,
        Some(Message::Aller(Page::Tailscale)),
    )
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
        row(cases).spacing(12).into()
    });
    column(lignes).spacing(12).into()
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
        point(teinte, 10.0),
        container(
            t(abreger(&s.nom, NOM_MAX), 16)
                .font(DEMI_GRAS)
                .color(theme::TEXTE)
                .wrapping(Wrapping::None)
        )
        .clip(true)
        .width(Length::Fill),
        t(latence, 15)
            .font(DEMI_GRAS)
            .color(couleur_latence)
            .wrapping(Wrapping::None),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let adresse = container(commun::mono(abreger(&s.adresse, 36), 12.5, secondaire))
        .clip(true)
        .width(Length::Fill);
    let bas = row![
        t(compte_nodes(s), 13)
            .color(secondaire)
            .wrapping(Wrapping::None),
        Space::new().width(Length::Fill),
        t(note, 13)
            .font(theme::MOYEN)
            .color(couleur_note)
            .wrapping(Wrapping::None),
    ]
    .align_y(Alignment::Center);
    let contenu = column![haut, adresse, sparkline(&s.spark, teinte), bas].spacing(8);
    button(contenu)
        .width(Length::Fill)
        .padding([14, 16])
        .style(theme::touchable(Some(fond), 14.0, theme::TEXTE))
        .on_press(Message::Aller(Page::Service(s.id.clone())))
        .into()
}
