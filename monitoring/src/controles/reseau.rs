//! Par ou part le trafic de la tablette : tunnel Tailscale (VPN Android) ou acces direct.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

/// Adresse locale que le systeme utiliserait pour joindre `ip` (aucun paquet envoye).
fn adresse_source(ip: &str) -> Option<Ipv4Addr> {
    let cible: Ipv4Addr = ip.parse().ok()?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((cible, 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(a) => Some(a),
        IpAddr::V6(_) => None,
    }
}

/// Plage CGNAT 100.64.0.0/10 utilisee par Tailscale.
fn est_ip_tailscale(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 100 && (o[1] & 0xC0) == 64
}

/// Vrai si le trafic vers `ip` part dans le tunnel Tailscale (route de sous-reseau ou exit node).
pub fn passe_par_tailscale(ip: &str) -> bool {
    adresse_source(ip).is_some_and(est_ip_tailscale)
}

/// Adresse Tailscale de la tablette si le tunnel Tailscale est actif.
pub fn tablette_sur_tailnet() -> Option<Ipv4Addr> {
    adresse_source("100.100.100.100").filter(|a| est_ip_tailscale(*a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plage_tailscale() {
        for (ip, attendu) in [
            ("100.64.0.1", true),
            ("100.127.255.254", true),
            ("100.128.0.1", false),
            ("10.0.0.1", false),
        ] {
            assert_eq!(est_ip_tailscale(ip.parse().unwrap()), attendu, "{ip}");
        }
    }
}
