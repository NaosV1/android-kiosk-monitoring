//! Ping ICMP (IPv4) sans droits root : socket ICMP « datagramme » du noyau, autorisee par
//! `net.ipv4.ping_group_range`. Un echo par seconde jusqu'a la premiere reponse ou la fin du delai.

use std::collections::HashMap;
use std::io::{self, Read};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use super::{Echec, Resultat, chemin, dns, duree, verifier_route};
use crate::config::ConfigNode;
use crate::https;

static NUMEROS: AtomicU16 = AtomicU16::new(1);

pub(super) fn controler(node: &ConfigNode, hote: &str, delai: f64) -> Resultat {
    let resolution = match &node.resolveur {
        Some(serveur) => dns::resoudre_par(hote, serveur, delai).map(|ips| ips[0]),
        None => dns::premiere_ipv4(hote),
    };
    let ip = match resolution {
        Ok(ip) => ip,
        Err(Echec(m)) => return Resultat::echec(m),
    };
    let ip_texte = ip.to_string();
    if let Err(Echec(m)) = verifier_route(node, &ip_texte) {
        return Resultat::echec(m);
    }
    let socket = match Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4)) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            return Resultat::echec("Ping non autorisé sur ce système");
        }
        Err(e) => return Resultat::echec(https::raison_io(&e, delai)),
    };
    let voie = chemin(node, Some(hote), &ip_texte);
    match echanger(&socket, ip, duree(delai)) {
        Ok(Some((ms, 0))) => Resultat::reussi(ms, format!("Répond au ping{voie}")),
        Ok(Some((ms, perdus))) => {
            let s = if perdus > 1 { "s" } else { "" };
            Resultat::reussi(ms, format!("Répond au ping ({perdus} perdu{s}){voie}"))
        }
        Ok(None) => Resultat::echec(format!(
            "Aucune réponse au ping en {} s{voie}",
            https::secondes(delai)
        )),
        Err(e) => Resultat::echec(https::raison_io(&e, delai)),
    }
}

/// Temps de reponse (ms) et nombre d'echos restes sans reponse ; `None` si aucune reponse.
fn echanger(socket: &Socket, ip: Ipv4Addr, delai: Duration) -> io::Result<Option<(f64, usize)>> {
    let destination = SockAddr::from(SocketAddrV4::new(ip, 0));
    let mut envois: HashMap<u16, Instant> = HashMap::new();
    let fin = Instant::now() + delai;
    let mut prochain = Instant::now();
    let mut tampon = [0u8; 1024];
    loop {
        let maintenant = Instant::now();
        if maintenant >= fin {
            return Ok(None);
        }
        if maintenant >= prochain {
            let numero = NUMEROS.fetch_add(1, Ordering::Relaxed);
            socket.send_to(&echo(numero), &destination)?;
            envois.insert(numero, maintenant);
            prochain = maintenant + Duration::from_secs(1);
        }
        let attente = prochain.min(fin).saturating_duration_since(Instant::now());
        socket.set_read_timeout(Some(attente.max(Duration::from_millis(10))))?;
        let n = match (&*socket).read(&mut tampon) {
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(e) => return Err(e),
        };
        if let Some(numero) = numero_de_reponse(&tampon[..n])
            && let Some(envoi) = envois.get(&numero)
        {
            return Ok(Some((
                envoi.elapsed().as_secs_f64() * 1000.0,
                envois.len() - 1,
            )));
        }
    }
}

/// Demande d'echo ICMP. Linux remplace l'identifiant (et recalcule la somme de controle).
fn echo(numero: u16) -> Vec<u8> {
    let mut paquet = vec![8, 0, 0, 0, 0, 0];
    paquet.extend_from_slice(&numero.to_be_bytes());
    paquet.extend_from_slice(b"tablette-monitoring");
    let somme = somme_de_controle(&paquet);
    paquet[2..4].copy_from_slice(&somme.to_be_bytes());
    paquet
}

fn somme_de_controle(octets: &[u8]) -> u16 {
    let mut somme: u32 = octets
        .chunks(2)
        .map(|m| u32::from(u16::from_be_bytes([m[0], m.get(1).copied().unwrap_or(0)])))
        .sum();
    while somme > 0xFFFF {
        somme = (somme & 0xFFFF) + (somme >> 16);
    }
    !(somme as u16)
}

/// Numero d'une reponse d'echo. Linux livre le message ICMP seul ; macOS le fait preceder
/// de l'en-tete IPv4 (premier octet 0x4_, jamais un type ICMP de reponse).
fn numero_de_reponse(paquet: &[u8]) -> Option<u16> {
    let icmp = if paquet.first().is_some_and(|o| o >> 4 == 4) {
        paquet.get(usize::from(paquet[0] & 0x0F) * 4..)?
    } else {
        paquet
    };
    (icmp.len() >= 8 && icmp[0] == 0).then(|| u16::from_be_bytes([icmp[6], icmp[7]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_somme_de_controle_d_un_echo_est_valide() {
        let paquet = echo(0x1234);
        assert_eq!(somme_de_controle(&paquet), 0);
        assert_eq!(&paquet[6..8], &[0x12, 0x34]);
    }

    #[test]
    fn lecture_des_reponses() {
        let reponse = [0, 0, 0xAB, 0xCD, 0, 7, 0x01, 0x02];
        assert_eq!(numero_de_reponse(&reponse), Some(0x0102));
        let mut avec_ip = vec![0x45; 20];
        avec_ip.extend_from_slice(&reponse);
        assert_eq!(numero_de_reponse(&avec_ip), Some(0x0102));
        assert_eq!(numero_de_reponse(&[3, 1, 0, 0, 0, 0, 0, 1]), None);
        assert_eq!(numero_de_reponse(&[0, 0, 0]), None);
    }
}
