//! Ecran tactile plein ecran : client de l'API du serveur, rafraichi toutes les 5 s.
//!
//! Maquette en 1280 x 800 pixels logiques, agrandie a la taille de la fenetre
//! (x1,5 sur l'ecran 1920 x 1200 de la tablette).

mod client;
mod commun;
mod format;
mod graphiques;
mod icones;
mod logique;
mod pages;
mod taches;
mod theme;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use iced::widget::{column, stack};
use iced::{Color, Element, Size, Subscription, Task, event, mouse, touch, window};

use crate::api::{DetailService, Etat, Incidents, StatutMaj};

use client::Client;
use logique::Page;
use taches::{apres, en_fond, minuterie};

const LARGEUR: f32 = 1280.0;
const HAUTEUR: f32 = 800.0;
const RAFRAICHISSEMENT: Duration = Duration::from_secs(5);
/// Au-dela, le detail d'un service ou la liste des incidents est recharge.
const FRAICHEUR: Duration = Duration::from_secs(30);
const DUREE_TOAST: Duration = Duration::from_millis(2500);

pub fn lancer(url: String) -> iced::Result {
    iced::application(move || Ecran::new(url.clone()), Ecran::update, Ecran::view)
        .title("Monitoring")
        .subscription(Ecran::subscription)
        .scale_factor(|ecran: &Ecran| ecran.echelle)
        .style(|ecran: &Ecran, _| iced::theme::Style {
            background_color: ecran.fond(),
            text_color: theme::TEXTE,
        })
        .default_font(theme::INTER)
        .window_size(Size::new(LARGEUR, HAUTEUR))
        .run()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filtre {
    Tous,
    EnCours,
    Resolus,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Verification,
    Relance,
    Acquittement,
}

#[derive(Clone, Debug)]
pub enum Message {
    Tic,
    Rafraichir,
    /// Rafraichit aussi le detail de la page, meme recent.
    Recharger,
    EtatRecu(Result<Box<Etat>, String>),
    ServiceRecu(Result<DetailService, String>),
    IncidentsRecus(Result<Incidents, String>),
    Aller(Page),
    Filtrer(Filtre),
    Agir(Action, String),
    ActionTerminee(Action, Result<(), String>),
    DemanderMaj(String),
    AnnulerMaj,
    ConfirmerMaj,
    InstallationDemandee(String, Result<(), String>),
    AfficherToast(String),
    FinToast(u64),
    Toucher,
    Fenetre(Size),
}

pub struct Ecran {
    client: Client,
    /// Facteur d'agrandissement de la maquette 1280 x 800.
    echelle: f32,
    page: Page,
    etat: Option<Etat>,
    /// Le dernier rafraichissement a echoue.
    erreur: bool,
    service: Option<(DetailService, Instant)>,
    incidents: Option<(Incidents, Instant)>,
    filtre: Filtre,
    derniere_action: Instant,
    /// Incidents ouverts non acquittes deja vus (`None` avant le premier etat).
    incidents_connus: Option<HashSet<i64>>,
    toast: Option<(u64, String)>,
    /// Version dont l'installation attend une confirmation (fenetre ouverte).
    confirmer_maj: Option<String>,
    /// Version dont l'installation a ete demandee.
    maj_envoyee: Option<String>,
    toast_maj_vu: bool,
    /// Reception du dernier etat (secondes Unix).
    recu: i64,
    /// Bascule a chaque donnee recue du serveur : voir `fond`.
    fond_bascule: bool,
}

impl Ecran {
    fn new(url: String) -> (Ecran, Task<Message>) {
        let ecran = Ecran {
            client: Client::new(url),
            echelle: 1.0,
            page: Page::Accueil,
            etat: None,
            erreur: false,
            service: None,
            incidents: None,
            filtre: Filtre::Tous,
            derniere_action: Instant::now(),
            incidents_connus: None,
            toast: None,
            confirmer_maj: None,
            maj_envoyee: None,
            toast_maj_vu: false,
            recu: 0,
            fond_bascule: false,
        };
        let premier = ecran.rafraichir();
        (ecran, premier)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            minuterie(RAFRAICHISSEMENT).map(|()| Message::Tic),
            event::listen_with(|evenement, _, _| match evenement {
                iced::Event::Mouse(mouse::Event::ButtonPressed(_))
                | iced::Event::Touch(touch::Event::FingerPressed { .. }) => Some(Message::Toucher),
                _ => None,
            }),
            window::events().filter_map(|(_, evenement)| match evenement {
                window::Event::Opened { size, .. } | window::Event::Resized(size) => {
                    Some(Message::Fenetre(size))
                }
                _ => None,
            }),
        ])
    }

    /// Fond de la fenetre, alterne entre deux teintes indiscernables a chaque donnee recue.
    ///
    /// Le moteur de rendu logiciel d'iced 0.14 ne repeint que les zones qu'il juge modifiees, et ce
    /// calcul rate des textes quand une liste change de forme (groupes reordonnes, lignes
    /// ajoutees) : un bout de l'ancien texte restait affiche indefiniment
    /// (https://github.com/iced-rs/iced/issues/3554). Un fond different l'oblige a tout repeindre,
    /// une fois par reception (toutes les 5 s) ; toucher et faire defiler gardent le repeint
    /// partiel.
    fn fond(&self) -> Color {
        let mut fond = theme::FOND;
        if self.fond_bascule {
            fond.b -= 1.0 / 255.0;
        }
        fond
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        if matches!(
            message,
            Message::EtatRecu(Ok(_)) | Message::ServiceRecu(Ok(_)) | Message::IncidentsRecus(Ok(_))
        ) {
            self.fond_bascule = !self.fond_bascule;
        }
        match message {
            Message::Tic => {
                self.peut_etre_en_veille();
                return self.rafraichir();
            }
            Message::Rafraichir => return self.rafraichir(),
            Message::Recharger => return Task::batch([self.rafraichir(), self.charger_page(true)]),
            Message::EtatRecu(Ok(etat)) => return self.etat_recu(*etat),
            Message::EtatRecu(Err(_)) => {
                self.erreur = true;
                return self.charger_page(false);
            }
            Message::ServiceRecu(Ok(detail)) => self.service = Some((detail, Instant::now())),
            Message::IncidentsRecus(Ok(liste)) => self.incidents = Some((liste, Instant::now())),
            Message::ServiceRecu(Err(_)) | Message::IncidentsRecus(Err(_)) => {}
            Message::Aller(page) => {
                self.derniere_action = Instant::now();
                self.page = page;
                return self.charger_page(true);
            }
            Message::Filtrer(filtre) => self.filtre = filtre,
            Message::Agir(action, cible) => return self.agir(action, cible),
            Message::ActionTerminee(action, resultat) => {
                return self.action_terminee(action, resultat);
            }
            Message::DemanderMaj(version) => self.confirmer_maj = Some(version),
            Message::AnnulerMaj => self.confirmer_maj = None,
            Message::ConfirmerMaj => {
                if let Some(version) = self.confirmer_maj.take() {
                    let client = self.client.clone();
                    let demandee = version.clone();
                    return en_fond(
                        move || client.installer(&demandee),
                        move |r| Message::InstallationDemandee(version.clone(), r),
                    );
                }
            }
            Message::InstallationDemandee(version, Ok(())) => self.maj_envoyee = Some(version),
            Message::InstallationDemandee(_, Err(erreur)) => return self.afficher_toast(erreur),
            Message::AfficherToast(texte) => return self.afficher_toast(texte),
            Message::FinToast(id) => {
                if self
                    .toast
                    .as_ref()
                    .is_some_and(|(courant, _)| *courant == id)
                {
                    self.toast = None;
                }
            }
            Message::Toucher => self.derniere_action = Instant::now(),
            Message::Fenetre(taille) => {
                // la taille recue est deja divisee par l'echelle en cours
                let (largeur, hauteur) =
                    (taille.width * self.echelle, taille.height * self.echelle);
                self.echelle = (largeur / LARGEUR).min(hauteur / HAUTEUR).clamp(0.5, 4.0);
            }
        }
        Task::none()
    }

    fn rafraichir(&self) -> Task<Message> {
        let client = self.client.clone();
        en_fond(move || client.etat().map(Box::new), Message::EtatRecu)
    }

    fn etat_recu(&mut self, etat: Etat) -> Task<Message> {
        self.erreur = false;
        self.recu = format::maintenant();
        let a_signaler = logique::incidents_a_signaler(&etat);
        if let Some(connus) = &self.incidents_connus
            && a_signaler.iter().any(|id| !connus.contains(id))
        {
            self.derniere_action = Instant::now();
            if self.page == Page::Veille {
                self.page = Page::Accueil;
            }
            perimer(&mut self.incidents);
        }
        self.incidents_connus = Some(a_signaler);

        let m = &etat.maj;
        if self.maj_envoyee.is_some() && m.statut == StatutMaj::Prete && m.erreur.is_some() {
            self.maj_envoyee = None;
        }
        let mut taches = vec![self.charger_page(false)];
        if let Some(installee) = &m.derniere_installation
            && !self.toast_maj_vu
            && installee.version == m.actuelle
            && format::maintenant() - installee.quand < 180
        {
            self.toast_maj_vu = true;
            let texte = format!(
                "Mise à jour installée : {}",
                format::nom_version(&m.actuelle)
            );
            taches.push(apres(
                Duration::from_millis(500),
                Message::AfficherToast(texte),
            ));
        }
        self.etat = Some(etat);
        Task::batch(taches)
    }

    /// Detail du service ou incidents de la page affichee, si absents ou trop anciens.
    fn charger_page(&self, force: bool) -> Task<Message> {
        let perime =
            |charge: Option<&Instant>| force || charge.is_none_or(|t| t.elapsed() > FRAICHEUR);
        let client = self.client.clone();
        match &self.page {
            Page::Service(id) => {
                let courant = self
                    .service
                    .as_ref()
                    .filter(|(d, _)| &d.id == id)
                    .map(|(_, t)| t);
                if perime(courant) {
                    let id = id.clone();
                    return en_fond(move || client.service(&id), Message::ServiceRecu);
                }
            }
            Page::Incidents if perime(self.incidents.as_ref().map(|(_, t)| t)) => {
                return en_fond(move || client.incidents(), Message::IncidentsRecus);
            }
            _ => {}
        }
        Task::none()
    }

    fn agir(&mut self, action: Action, cible: String) -> Task<Message> {
        let client = self.client.clone();
        en_fond(
            move || match action {
                Action::Verification => client.verifier_maj(),
                Action::Relance => client.relancer(&cible),
                Action::Acquittement => cible
                    .parse()
                    .map_err(|_| String::new())
                    .and_then(|id| client.acquitter(id)),
            },
            move |resultat| Message::ActionTerminee(action, resultat),
        )
    }

    fn action_terminee(&mut self, action: Action, resultat: Result<(), String>) -> Task<Message> {
        if resultat.is_err() {
            return self.afficher_toast("Action impossible : serveur injoignable".into());
        }
        match action {
            Action::Verification => Task::batch([
                self.afficher_toast("Vérification lancée".into()),
                apres(Duration::from_secs(4), Message::Rafraichir),
            ]),
            Action::Relance => Task::batch([
                self.afficher_toast("Contrôles relancés".into()),
                apres(Duration::from_secs(3), Message::Recharger),
            ]),
            Action::Acquittement => Task::batch([
                self.afficher_toast("Incident acquitté".into()),
                self.rafraichir(),
            ]),
        }
    }

    fn afficher_toast(&mut self, texte: String) -> Task<Message> {
        let id = self.toast.as_ref().map_or(0, |(id, _)| id + 1);
        self.toast = Some((id, texte));
        apres(DUREE_TOAST, Message::FinToast(id))
    }

    /// Le serveur ne repond plus (sauf pendant une installation, ou c'est attendu).
    fn hors_ligne(&self) -> bool {
        self.erreur && self.maj_envoyee.is_none()
    }

    fn installation_en_cours(&self) -> bool {
        self.maj_envoyee.is_some()
            || self.etat.as_ref().is_some_and(|e| {
                matches!(
                    e.maj.statut,
                    StatutMaj::Installation | StatutMaj::Redemarrage
                )
            })
    }

    fn peut_etre_en_veille(&mut self) {
        let Some(etat) = &self.etat else { return };
        let minutes = etat.veille_minutes;
        if minutes > 0
            && self.page != Page::Veille
            && !self.installation_en_cours()
            && self.derniere_action.elapsed() > Duration::from_secs(minutes as u64 * 60)
        {
            self.confirmer_maj = None;
            self.page = Page::Veille;
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let Some(etat) = &self.etat else {
            return commun::demarrage(self.erreur);
        };
        let contexte = pages::Contexte {
            etat,
            maintenant: format::maintenant(),
            service: self.service.as_ref().map(|(d, _)| d),
            incidents: self.incidents.as_ref().map(|(i, _)| i),
            filtre: self.filtre,
            injoignable_depuis: self.hors_ligne().then_some(self.recu),
        };
        let mut page = pages::page(&self.page, &contexte);
        if self.hors_ligne() {
            // au-dessus de la page plutot que par-dessus : le titre et l'horloge restent lisibles
            page = column![
                commun::hors_ligne(self.recu, RAFRAICHISSEMENT.as_secs()),
                page
            ]
            .into();
        }
        let mut couches = vec![page];
        if self.installation_en_cours() {
            let version = self
                .maj_envoyee
                .clone()
                .or_else(|| etat.maj.disponible.as_ref().map(|d| d.version.clone()));
            couches.push(pages::maj::installation(version.unwrap_or_default()));
        } else if let Some(version) = &self.confirmer_maj {
            couches.push(pages::maj::confirmation(version, &etat.maj.actuelle));
        }
        if let Some((_, texte)) = &self.toast {
            couches.push(commun::toast(texte));
        }
        stack(couches).into()
    }
}

/// Force le rechargement au prochain rafraichissement, sans effacer l'affichage.
fn perimer<T>(donnees: &mut Option<(T, Instant)>) {
    if let Some((_, charge)) = donnees {
        *charge = charge.checked_sub(FRAICHEUR * 2).unwrap_or(*charge);
    }
}
