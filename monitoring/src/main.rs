//! Monitoring de la tablette : un seul binaire, lance deux fois par `lancer.sh`.
//!
//! ```text
//! monitoring serveur      controles, historique, mises a jour et API JSON (127.0.0.1:8080)
//! monitoring ecran [URL]  interface tactile plein ecran, cliente de cette API
//! monitoring sonde        code de sortie 0 si le serveur repond (attente du demarrage)
//! monitoring --version    version compilee (controlee avant chaque installation)
//! ```

/// Ligne horodatee dans le journal (`serveur.log`).
macro_rules! journal {
    ($($arg:tt)*) => {
        println!("{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), format_args!($($arg)*))
    };
}

mod api;
mod chemins;
mod config;
mod controles;
mod ecran;
mod https;
mod maj;
mod moniteur;
mod serveur_http;
mod tailscale;

use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chemins::Chemins;
use config::Config;

/// Version de la release, fixee par la CI a la compilation (`dev` sinon).
const VERSION: &str = match option_env!("MONITORING_VERSION") {
    Some(v) => v,
    None => "dev",
};

const USAGE: &str = "usage : monitoring serveur | ecran [URL] | sonde | --version";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments.as_slice() {
        ["--version"] => {
            println!("{VERSION}");
            ExitCode::SUCCESS
        }
        ["serveur"] => serveur(),
        ["sonde"] => sonde(),
        ["ecran"] => ecran(None),
        ["ecran", url] => ecran(Some(url.to_string())),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn charger() -> Result<(Chemins, Config), String> {
    let chemins = Chemins::depuis_environnement()
        .map_err(|e| format!("Dossier de l'installation introuvable : {e}"))?;
    let conf = Config::lire(&chemins.config).map_err(|e| e.to_string())?;
    Ok((chemins, conf))
}

fn serveur() -> ExitCode {
    let (chemins, conf) = match charger() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let moniteur = match moniteur::Moniteur::ouvrir(&conf, &chemins.base) {
        Ok(m) => Arc::new(m),
        Err(e) => {
            eprintln!("Historique {} inutilisable : {e}", chemins.base.display());
            return ExitCode::FAILURE;
        }
    };
    journal!(
        "{} services, {} nodes, Tailscale actif={} api={}",
        conf.services.len(),
        conf.services.iter().map(|s| s.nodes.len()).sum::<usize>(),
        conf.tailscale.actif,
        if conf.tailscale.cle_api.is_empty() {
            "non"
        } else {
            "oui"
        }
    );
    let maj = Arc::new(maj::MisesAJour::new(&conf, chemins.clone()));
    let depot = if conf.maj.depot.is_empty() {
        "desactivees"
    } else {
        conf.maj.depot.as_str()
    };
    journal!("version {}, mises a jour : {depot}", maj.resume().actuelle);

    let m = Arc::clone(&moniteur);
    thread::spawn(move || m.boucle_controles());
    let m = Arc::clone(&moniteur);
    thread::spawn(move || tailscale::boucle(&m));
    let u = Arc::clone(&maj);
    thread::spawn(move || u.boucle());

    if let Err(e) = serveur_http::servir(&conf, moniteur, maj) {
        eprintln!("Ecoute sur {}:{} impossible : {e}", conf.ecoute, conf.port);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Attendue par `lancer.sh` : le serveur repond-il deja ?
fn sonde() -> ExitCode {
    let Ok((_, conf)) = charger() else {
        return ExitCode::FAILURE;
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .proxy(None)
        .build()
        .into();
    match agent.get(format!("{}/api/etat", conf.url_locale())).call() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn ecran(url: Option<String>) -> ExitCode {
    // La derniere ligne d'ecran.log est lisible telle quelle : lancer.sh la cite (page Mise a jour)
    // si l'ecran d'une nouvelle version s'arrete.
    std::panic::set_hook(Box::new(|info| {
        let message = message_de_panique(info.payload()).replace('\n', " ");
        // seule bibliotheque de l'ecran absente de l'installation d'origine (voir README)
        let conseil = if message.contains("libxkbcommon-x11") {
            " (apt install libxkbcommon-x11-0)"
        } else {
            ""
        };
        if let Some(lieu) = info.location() {
            eprintln!("panique dans {}:{}", lieu.file(), lieu.line());
        }
        eprintln!("Ecran arrete : {message}{conseil}");
    }));
    let url = match url {
        Some(u) => u.trim_end_matches('/').to_string(),
        None => match charger() {
            Ok((_, conf)) => conf.url_locale(),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        },
    };
    match ecran::lancer(url) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Ecran arrete : {e}");
            ExitCode::FAILURE
        }
    }
}

fn message_de_panique(panique: &(dyn std::any::Any + Send)) -> String {
    panique
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panique.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panique".into())
}
