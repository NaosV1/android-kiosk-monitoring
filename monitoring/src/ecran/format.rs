//! Mise en forme des durees, dates et mesures, a la francaise.

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

const JOURS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

/// Espace fine insecable, separateur des milliers en francais.
const ESPACE_FINE: char = '\u{202F}';

pub fn maintenant() -> i64 {
    Local::now().timestamp()
}

pub fn local(ts: i64) -> DateTime<Local> {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(Local::now)
}

/// `42 s`, `5 min`, `2 h 05`, `3 j`.
pub fn duree(secondes: f64) -> String {
    let s = secondes.round().max(0.0) as i64;
    if s < 60 {
        return format!("{s} s");
    }
    let m = s / 60;
    if m < 60 {
        return format!("{m} min");
    }
    let (h, r) = (m / 60, m % 60);
    if h < 24 {
        return if r > 0 {
            format!("{h} h {r:02}")
        } else {
            format!("{h} h")
        };
    }
    format!("{} j", h / 24)
}

/// `à l'instant`, `il y a 5 min`, ou `jamais` sans horodatage.
pub fn il_y_a(ts: Option<i64>, maintenant: i64) -> String {
    match ts {
        Some(ts) if ts != 0 => {
            if maintenant - ts < 45 {
                "à l'instant".into()
            } else {
                format!("il y a {}", duree((maintenant - ts) as f64))
            }
        }
        _ => "jamais".into(),
    }
}

pub fn heure<Tz: TimeZone>(date: &DateTime<Tz>) -> String {
    format!("{:02}:{:02}", date.hour(), date.minute())
}

/// `lundi 24 novembre`
pub fn jour_long<Tz: TimeZone>(date: &DateTime<Tz>) -> String {
    format!(
        "{} {} {}",
        JOURS[date.weekday().num_days_from_monday() as usize],
        date.day(),
        MOIS[date.month0() as usize]
    )
}

/// Heure seule le jour meme, sinon `24/11 08:30`.
pub fn quand<Tz: TimeZone>(date: &DateTime<Tz>, aujourd_hui: &DateTime<Tz>) -> String {
    if date.date_naive() == aujourd_hui.date_naive() {
        heure(date)
    } else {
        format!("{:02}/{:02} {}", date.day(), date.month(), heure(date))
    }
}

pub fn majuscule_initiale(texte: &str) -> String {
    let mut lettres = texte.chars();
    match lettres.next() {
        Some(premiere) => premiere.to_uppercase().chain(lettres).collect(),
        None => String::new(),
    }
}

/// `99,95 %`, `100 %` au-dela de 99,995, `—` sans valeur.
pub fn pct(valeur: Option<f64>) -> String {
    match valeur {
        None => "—".into(),
        Some(v) if v >= 99.995 => "100 %".into(),
        Some(v) => format!("{} %", format!("{v:.2}").replace('.', ",")),
    }
}

/// `1 234 ms` (arrondi), `—` sans valeur.
pub fn ms(valeur: Option<f64>) -> String {
    match valeur {
        None => "—".into(),
        Some(v) => format!("{} ms", entier(v.round() as i64)),
    }
}

/// Entier avec separateur des milliers.
pub fn entier(n: i64) -> String {
    let chiffres = n.unsigned_abs().to_string();
    let mut sortie = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            sortie.push(ESPACE_FINE);
        }
        sortie.push(c);
    }
    if n < 0 { format!("-{sortie}") } else { sortie }
}

/// Nombre sans decimale inutile, virgule decimale : `6`, `1,5`.
pub fn nombre(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        entier(v as i64)
    } else {
        v.to_string().replace('.', ",")
    }
}

/// `1 node`, `3 nodes`.
pub fn pluriel(n: usize, mot: &str) -> String {
    format!("{n} {mot}{}", if n > 1 { "s" } else { "" })
}

/// `v1.2.3`, ou `dev` pour une installation hors release.
pub fn nom_version(version: &str) -> String {
    if version.is_empty() || version == "dev" {
        "dev".into()
    } else {
        format!("v{version}")
    }
}

/// Au plus `n` caracteres, termine par « … » s'il a fallu couper (iced coupe net sinon).
pub fn abreger(texte: &str, n: usize) -> String {
    if texte.chars().count() <= n {
        return texte.to_string();
    }
    let debut: String = texte.chars().take(n.saturating_sub(1)).collect();
    format!("{}…", debut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Utc};

    #[test]
    fn durees() {
        assert_eq!(duree(-3.0), "0 s");
        assert_eq!(duree(59.4), "59 s");
        assert_eq!(duree(60.0), "1 min");
        assert_eq!(duree(3599.0), "59 min");
        assert_eq!(duree(3600.0), "1 h");
        assert_eq!(duree(2.0 * 3600.0 + 5.0 * 60.0), "2 h 05");
        assert_eq!(duree(3.0 * 86400.0 + 7200.0), "3 j");
    }

    #[test]
    fn il_y_a_relatif() {
        assert_eq!(il_y_a(None, 1000), "jamais");
        assert_eq!(il_y_a(Some(0), 1000), "jamais");
        assert_eq!(il_y_a(Some(970), 1000), "à l'instant");
        assert_eq!(il_y_a(Some(700), 1000), "il y a 5 min");
    }

    #[test]
    fn dates() {
        let paris = FixedOffset::east_opt(3600).unwrap();
        let d = paris.with_ymd_and_hms(2026, 11, 24, 8, 5, 0).unwrap();
        assert_eq!(heure(&d), "08:05");
        assert_eq!(jour_long(&d), "mardi 24 novembre");
        assert_eq!(quand(&d, &d), "08:05");
        let lendemain = paris.with_ymd_and_hms(2026, 11, 25, 0, 1, 0).unwrap();
        assert_eq!(quand(&d, &lendemain), "24/11 08:05");
        let aout = Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap();
        assert_eq!(jour_long(&aout), "dimanche 2 août");
        assert_eq!(majuscule_initiale("hier · lundi"), "Hier · lundi");
        assert_eq!(majuscule_initiale("été"), "Été");
    }

    #[test]
    fn mesures() {
        assert_eq!(pct(None), "—");
        assert_eq!(pct(Some(99.996)), "100 %");
        assert_eq!(pct(Some(99.5)), "99,50 %");
        assert_eq!(ms(Some(12.4)), "12 ms");
        assert_eq!(ms(Some(1234.6)), "1\u{202F}235 ms");
        assert_eq!(entier(1_234_567), "1\u{202F}234\u{202F}567");
        assert_eq!(entier(-950), "-950");
        assert_eq!(nombre(6.0), "6");
        assert_eq!(nombre(1.5), "1,5");
        assert_eq!(pluriel(1, "node"), "1 node");
        assert_eq!(pluriel(2, "node"), "2 nodes");
        assert_eq!(nom_version("dev"), "dev");
        assert_eq!(nom_version("2.0.4"), "v2.0.4");
    }

    #[test]
    fn abreviation() {
        assert_eq!(abreger("Base de données", 20), "Base de données");
        assert_eq!(abreger("Service très long nommé", 16), "Service très lo…");
        assert_eq!(abreger("Service très long", 9), "Service…");
    }
}
