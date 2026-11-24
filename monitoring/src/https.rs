//! Requetes HTTP(S) sortantes : controles, API Tailscale et GitHub.
//!
//! Les certificats sont verifies avec les autorites du systeme (/etc/ssl/certs), comme le
//! faisait la version Python : une autorite interne ajoutee a Debian reste reconnue.

use std::io;
use std::net::ToSocketAddrs;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use ureq::Agent;
use ureq::http::Uri;
use ureq::tls::{Certificate, RootCerts, TlsConfig};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

/// Joindre `hote` a une adresse imposee (comme `curl --resolve`) : le nom reste utilise
/// pour l'en-tete Host, le SNI et la verification du certificat.
#[derive(Clone, Debug)]
pub struct AdresseImposee {
    pub hote: String,
    pub adresse: String,
}

pub fn agent(delai: Duration, verifier_certificat: bool, adresse: Option<AdresseImposee>) -> Agent {
    let tls = TlsConfig::builder()
        .root_certs(racines_systeme())
        .disable_verification(!verifier_certificat)
        .build();
    let config = Agent::config_builder()
        .timeout_global(Some(delai))
        .http_status_as_error(false)
        .tls_config(tls)
        .build();
    match adresse {
        None => Agent::with_parts(config, DefaultConnector::default(), ResolveurSysteme),
        Some(a) => Agent::with_parts(config, DefaultConnector::default(), ResolveurImpose(a)),
    }
}

/// Texte court pour l'ecran ; `delai` sert au message de depassement.
pub fn raison(e: &ureq::Error, delai: f64) -> String {
    match e {
        ureq::Error::Timeout(_) => message_delai(delai),
        ureq::Error::HostNotFound => "Nom introuvable (DNS)".into(),
        ureq::Error::Io(e) => raison_io(e, delai),
        ureq::Error::Rustls(e) => raison_tls(e),
        ureq::Error::Tls(_) => "Erreur TLS".into(),
        autre => tronquer(&autre.to_string(), 110),
    }
}

fn raison_tls(e: &rustls::Error) -> String {
    match e {
        rustls::Error::InvalidCertificate(_) => "Certificat TLS invalide".into(),
        _ => "Erreur TLS".into(),
    }
}

pub fn raison_io(e: &io::Error, delai: f64) -> String {
    // pendant l'echange, les erreurs TLS arrivent enveloppees dans une erreur d'entree-sortie
    if let Some(tls) = e.get_ref().and_then(|i| i.downcast_ref::<rustls::Error>()) {
        return raison_tls(tls);
    }
    match e.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => message_delai(delai),
        io::ErrorKind::ConnectionRefused => "Connexion refusée".into(),
        io::ErrorKind::ConnectionReset => "Connexion coupée".into(),
        io::ErrorKind::NetworkUnreachable | io::ErrorKind::HostUnreachable => {
            "Réseau injoignable".into()
        }
        _ => tronquer(&e.to_string(), 110),
    }
}

pub fn message_delai(delai: f64) -> String {
    format!("Délai dépassé après {} s", secondes(delai))
}

/// `10` -> "10", `2.5` -> "2,5".
pub fn secondes(valeur: f64) -> String {
    valeur.to_string().replace('.', ",")
}

/// Les `n` premiers caracteres.
pub fn tronquer(texte: &str, n: usize) -> String {
    texte.chars().take(n).collect()
}

/// Encodage `%XX` d'un segment de chemin d'url (comme `encodeURIComponent`).
pub fn encoder_segment(segment: &str) -> String {
    segment
        .bytes()
        .map(|o| match o {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                char::from(o).to_string()
            }
            _ => format!("%{o:02X}"),
        })
        .collect()
}

/// Lit au plus `limite` octets du debut du corps.
pub fn lire_debut(
    reponse: &mut ureq::http::Response<ureq::Body>,
    limite: u64,
) -> io::Result<Vec<u8>> {
    use io::Read;
    let mut corps = Vec::new();
    reponse
        .body_mut()
        .as_reader()
        .take(limite)
        .read_to_end(&mut corps)?;
    Ok(corps)
}

fn racines_systeme() -> RootCerts {
    static RACINES: OnceLock<Arc<Vec<Certificate<'static>>>> = OnceLock::new();
    let certificats = RACINES.get_or_init(|| {
        let trouves = rustls_native_certs::load_native_certs();
        for e in &trouves.errors {
            journal!("certificats du systeme : {e}");
        }
        Arc::new(
            trouves
                .certs
                .iter()
                .map(|c| Certificate::from_der(c.as_ref()).to_owned())
                .collect(),
        )
    });
    RootCerts::Specific(Arc::clone(certificats))
}

/// Resolution par le systeme, ou un nom introuvable reste une erreur de nom : le resolveur
/// d'ureq le rend comme une erreur d'entree-sortie generique (« failed to lookup address
/// information »), qui s'afficherait telle quelle au lieu de « Nom introuvable (DNS) ».
#[derive(Debug)]
struct ResolveurSysteme;

impl Resolver for ResolveurSysteme {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        match DefaultResolver::default().resolve(uri, config, timeout) {
            Err(ureq::Error::Io(_)) => Err(ureq::Error::HostNotFound),
            autre => autre,
        }
    }
}

#[derive(Debug)]
struct ResolveurImpose(AdresseImposee);

impl Resolver for ResolveurImpose {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let hote = uri
            .host()
            .unwrap_or("")
            .trim_matches(|c| c == '[' || c == ']');
        if !hote.eq_ignore_ascii_case(&self.0.hote) {
            // redirection vers un autre nom : resolution normale
            return ResolveurSysteme.resolve(uri, config, timeout);
        }
        let port = uri
            .port_u16()
            .unwrap_or(if uri.scheme_str() == Some("http") {
                80
            } else {
                443
            });
        let adresses = (self.0.adresse.as_str(), port)
            .to_socket_addrs()
            .map_err(|_| ureq::Error::HostNotFound)?;
        let mut resultat = self.empty();
        for a in adresses {
            if resultat.try_push(a).is_err() {
                break;
            }
        }
        if resultat.is_empty() {
            return Err(ureq::Error::HostNotFound);
        }
        Ok(resultat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodage_des_segments() {
        assert_eq!(encoder_segment("base-de-donnees"), "base-de-donnees");
        assert_eq!(encoder_segment("moi@exemple.fr"), "moi%40exemple.fr");
        assert_eq!(encoder_segment("a b/é"), "a%20b%2F%C3%A9");
    }

    #[test]
    fn un_nom_introuvable_est_signale_comme_tel() {
        // domaine reserve (RFC 6761) : jamais resolu, avec ou sans reseau
        let erreur = agent(Duration::from_secs(5), true, None)
            .get("https://nexiste-pas.invalid/")
            .call()
            .unwrap_err();
        assert_eq!(raison(&erreur, 5.0), "Nom introuvable (DNS)");
    }

    #[test]
    fn messages_d_erreur() {
        let certificat = rustls::Error::InvalidCertificate(rustls::CertificateError::UnknownIssuer);
        let enveloppee = io::Error::new(io::ErrorKind::InvalidData, certificat.clone());
        assert_eq!(raison_io(&enveloppee, 10.0), "Certificat TLS invalide");
        assert_eq!(
            raison(&ureq::Error::Rustls(certificat), 10.0),
            "Certificat TLS invalide"
        );
        let tls = io::Error::new(io::ErrorKind::InvalidData, rustls::Error::DecryptError);
        assert_eq!(raison_io(&tls, 10.0), "Erreur TLS");
        let refus = io::Error::from(io::ErrorKind::ConnectionRefused);
        assert_eq!(raison(&ureq::Error::Io(refus), 10.0), "Connexion refusée");
        assert_eq!(
            raison(&ureq::Error::HostNotFound, 2.5),
            "Nom introuvable (DNS)"
        );
        assert_eq!(message_delai(2.5), "Délai dépassé après 2,5 s");
        assert_eq!(message_delai(10.0), "Délai dépassé après 10 s");
    }
}
