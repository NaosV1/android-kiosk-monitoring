//! Resolution de noms : controle DNS par le resolveur de la tablette (/etc/resolv.conf), et
//! interrogation directe d'un serveur DNS precis (`resolveur`, ex. le DNS interne d'un reseau
//! prive joint par Tailscale).

use std::collections::BTreeSet;
use std::hash::{BuildHasher, RandomState};
use std::io;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use dns_lookup::{AddrFamily, AddrInfoHints, LookupError, LookupErrorKind, SockType};

use super::{Echec, Resultat, duree, ms_depuis};
use crate::config::ConfigNode;
use crate::https;

pub(super) fn controler(node: &ConfigNode, hote: &str, delai: f64) -> Resultat {
    let debut = Instant::now();
    if let Some(serveur) = &node.resolveur {
        return match resoudre_par(hote, serveur, delai) {
            Ok(ips) => Resultat::reussi(
                ms_depuis(debut),
                resolu(&ips.iter().map(Ipv4Addr::to_string).collect::<Vec<_>>()),
            ),
            Err(Echec(m)) => Resultat::echec(m),
        };
    }
    // getaddrinfo n'a pas de delai : la resolution tourne dans un fil qu'on cesse d'attendre
    let (envoi, reception) = mpsc::channel();
    let nom = hote.to_string();
    thread::spawn(move || {
        let _ = envoi.send(resoudre_systeme(&nom));
    });
    let reponse = reception.recv_timeout(duree(delai));
    let ms = ms_depuis(debut);
    match reponse {
        Err(_) => Resultat::echec(https::message_delai(delai)),
        Ok(Err(e)) => Resultat::echec(match e.kind() {
            LookupErrorKind::NoName | LookupErrorKind::NoData => {
                "Nom introuvable (NXDOMAIN)".into()
            }
            LookupErrorKind::Again => "Serveur DNS injoignable".into(),
            _ => https::tronquer(&format!("Échec de résolution ({e})"), 110),
        }),
        Ok(Ok(ips)) if ips.is_empty() => Resultat::echec("Aucune adresse IPv4"),
        Ok(Ok(ips)) => Resultat::reussi(ms, resolu(&ips.into_iter().collect::<Vec<_>>())),
    }
}

fn resolu(ips: &[String]) -> String {
    match ips.len() {
        1 => format!("Résolu en {}", ips[0]),
        n => format!("Résolu en {} (+{})", ips[0], n - 1),
    }
}

/// Adresses IPv4 par le resolveur du systeme, triees.
fn resoudre_systeme(nom: &str) -> Result<BTreeSet<String>, LookupError> {
    Ok(ipv4_du_systeme(nom)?.map(|ip| ip.to_string()).collect())
}

fn ipv4_du_systeme(nom: &str) -> Result<impl Iterator<Item = Ipv4Addr>, LookupError> {
    let indications = AddrInfoHints {
        flags: 0,
        address: AddrFamily::Inet.into(),
        socktype: SockType::Stream.into(),
        protocol: 0,
    };
    let adresses = dns_lookup::getaddrinfo(Some(nom), None, Some(indications))?;
    Ok(adresses.filter_map(|a| match a.ok()?.sockaddr.ip() {
        IpAddr::V4(ip) => Some(ip),
        IpAddr::V6(_) => None,
    }))
}

/// Premiere adresse IPv4 de `nom` donnee par le resolveur du systeme.
pub(super) fn premiere_ipv4(nom: &str) -> Result<Ipv4Addr, Echec> {
    ipv4_du_systeme(nom)
        .ok()
        .and_then(|mut ips| ips.next())
        .ok_or_else(|| Echec("Nom introuvable (DNS)".into()))
}

/// Adresses IPv4 de `nom` demandees directement au serveur DNS `serveur` (UDP, port 53).
pub fn resoudre_par(nom: &str, serveur: &str, delai: f64) -> Result<Vec<Ipv4Addr>, Echec> {
    if let Ok(ip) = nom.parse::<Ipv4Addr>() {
        return Ok(vec![ip]);
    }
    let identifiant = (RandomState::new().hash_one(Instant::now()) & 0xFFFF) as u16;
    let requete = requete_a(nom, identifiant)?;
    let injoignable = |e: io::Error| {
        Echec(format!(
            "DNS {serveur} injoignable ({})",
            https::raison_io(&e, delai)
        ))
    };
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(injoignable)?;
    socket
        .set_read_timeout(Some(Duration::from_secs_f64((delai / 2.0).clamp(0.5, 3.0))))
        .map_err(injoignable)?;
    let mut tampon = [0u8; 4096];
    // une relance si le paquet UDP se perd
    for _ in 0..2 {
        socket
            .send_to(&requete, (serveur, 53))
            .map_err(injoignable)?;
        loop {
            match socket.recv(&mut tampon) {
                Ok(n) if n >= 12 && tampon[..2] == identifiant.to_be_bytes() => {
                    return lire_reponse(&tampon[..n], nom, serveur);
                }
                Ok(_) => continue,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    break;
                }
                Err(e) => return Err(injoignable(e)),
            }
        }
    }
    Err(Echec(format!("DNS {serveur} ne répond pas")))
}

/// Requete DNS de type A, recursion demandee.
fn requete_a(nom: &str, identifiant: u16) -> Result<Vec<u8>, Echec> {
    let invalide = || {
        Echec(format!(
            "Nom invalide pour le DNS : {}",
            https::tronquer(nom, 60)
        ))
    };
    let mut requete = Vec::with_capacity(nom.len() + 18);
    requete.extend_from_slice(&identifiant.to_be_bytes());
    requete.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for etiquette in nom.trim_end_matches('.').split('.') {
        if etiquette.is_empty() || etiquette.len() > 63 || !etiquette.is_ascii() {
            return Err(invalide());
        }
        requete.push(etiquette.len() as u8);
        requete.extend_from_slice(etiquette.to_ascii_lowercase().as_bytes());
    }
    requete.extend_from_slice(&[0, 0, 1, 0, 1]);
    Ok(requete)
}

fn lire_reponse(reponse: &[u8], nom: &str, serveur: &str) -> Result<Vec<Ipv4Addr>, Echec> {
    match reponse[3] & 0x0F {
        0 => {}
        3 => return Err(Echec(format!("{nom} inconnu du DNS {serveur}"))),
        code => {
            let libelle = match code {
                2 => "SERVFAIL".to_string(),
                5 => "REFUSED".to_string(),
                autre => autre.to_string(),
            };
            return Err(Echec(format!("DNS {serveur} : erreur {libelle}")));
        }
    }
    let illisible = || Echec(format!("DNS {serveur} : réponse illisible"));
    let octet = |i: usize| reponse.get(i).copied().ok_or_else(illisible);
    let nombre = u16::from_be_bytes([octet(6)?, octet(7)?]);
    let mut i = sauter_nom(reponse, 12).ok_or_else(illisible)? + 4;
    let mut ips = Vec::new();
    for _ in 0..nombre {
        i = sauter_nom(reponse, i).ok_or_else(illisible)?;
        let entete = reponse.get(i..i + 10).ok_or_else(illisible)?;
        let type_ = u16::from_be_bytes([entete[0], entete[1]]);
        let longueur = usize::from(u16::from_be_bytes([entete[8], entete[9]]));
        i += 10;
        let donnees = reponse.get(i..i + longueur).ok_or_else(illisible)?;
        if type_ == 1 && longueur == 4 {
            ips.push(Ipv4Addr::new(
                donnees[0], donnees[1], donnees[2], donnees[3],
            ));
        }
        i += longueur;
    }
    if ips.is_empty() {
        return Err(Echec(format!(
            "{nom} : aucune adresse IPv4 (DNS {serveur})"
        )));
    }
    Ok(ips)
}

/// Position qui suit le nom (etiquettes, eventuellement terminees par un pointeur) commencant en `i`.
fn sauter_nom(message: &[u8], mut i: usize) -> Option<usize> {
    loop {
        let longueur = *message.get(i)?;
        match longueur {
            0 => return Some(i + 1),
            l if l & 0xC0 == 0xC0 => return Some(i + 2),
            l => i += usize::from(l) + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reponse(rcode: u8, reponses: &[(u16, &[u8])]) -> Vec<u8> {
        let mut m = requete_a("intranet.exemple.fr", 0x4242).unwrap();
        m[2] = 0x81;
        m[3] = 0x80 | rcode;
        m[7] = reponses.len() as u8;
        for (type_, donnees) in reponses {
            m.extend_from_slice(&[0xC0, 12]);
            m.extend_from_slice(&type_.to_be_bytes());
            m.extend_from_slice(&[0, 1, 0, 0, 0, 60]);
            m.extend_from_slice(&(donnees.len() as u16).to_be_bytes());
            m.extend_from_slice(donnees);
        }
        m
    }

    #[test]
    fn requete_bien_formee() {
        let r = requete_a("Intranet.Exemple.fr.", 0x0102).unwrap();
        assert_eq!(&r[..4], &[0x01, 0x02, 0x01, 0x00]);
        assert_eq!(
            &r[12..],
            b"\x08intranet\x07exemple\x02fr\x00\x00\x01\x00\x01"
        );
        assert!(requete_a("a..b", 1).is_err());
        assert!(requete_a("été.fr", 1).is_err());
    }

    #[test]
    fn lecture_des_adresses() {
        let cname: &[u8] = &[3, b'w', b'w', b'w', 0xC0, 12];
        let m = reponse(0, &[(5, cname), (1, &[10, 0, 0, 7]), (1, &[10, 0, 0, 8])]);
        let ips = lire_reponse(&m, "intranet.exemple.fr", "10.0.0.53").unwrap();
        assert_eq!(
            ips,
            [Ipv4Addr::new(10, 0, 0, 7), Ipv4Addr::new(10, 0, 0, 8)]
        );
    }

    #[test]
    fn erreurs_du_serveur() {
        let nom = "intranet.exemple.fr";
        let message = |m: &[u8]| lire_reponse(m, nom, "10.0.0.53").unwrap_err().0;
        assert_eq!(
            message(&reponse(3, &[])),
            "intranet.exemple.fr inconnu du DNS 10.0.0.53"
        );
        assert_eq!(message(&reponse(2, &[])), "DNS 10.0.0.53 : erreur SERVFAIL");
        assert_eq!(
            message(&reponse(0, &[(28, &[0; 16])])),
            "intranet.exemple.fr : aucune adresse IPv4 (DNS 10.0.0.53)"
        );
        let mut tronquee = reponse(0, &[(1, &[10, 0, 0, 7])]);
        tronquee.truncate(tronquee.len() - 2);
        assert_eq!(message(&tronquee), "DNS 10.0.0.53 : réponse illisible");
    }
}
