//! lancer.sh : installation du binaire lors du passage depuis la version Python, et retour
//! automatique a la version precedente quand une nouvelle version ne demarre pas.
//!
//! Le binaire est remplace par un script factice dont le comportement depend de fichiers
//! temoins (`serveur-ko`, `ecran-ko`) poses dans le dossier d'installation.

use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const LANCEUR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/lancer.sh");

/// Binaire factice de la version `version`.
fn binaire_factice(version: &str) -> String {
    format!(
        r#"#!/bin/bash
cd "$(dirname "$0")"
case "$1" in
  --version) echo {version} ;;
  serveur) [ -f serveur-ko ] && exit 1; touch serveur-pret; exec sleep 60 ;;
  sonde) [ -f serveur-pret ] ;;
  ecran) if [ -f ecran-ko ]; then echo "Ecran arrete : bibliotheque absente" >&2; exit 1; fi; exec sleep 60 ;;
esac
"#
    )
}

fn ecrire(chemin: &Path, contenu: &str, executable: bool) {
    fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    fs::write(chemin, contenu).unwrap();
    let mode = if executable { 0o755 } else { 0o644 };
    fs::set_permissions(chemin, fs::Permissions::from_mode(mode)).unwrap();
}

/// Installation telle que la laisse l'installateur Python 1.0.x juste apres avoir installe la
/// release Rust 2.0.5 : nouveaux lancer.sh et VERSION, binaire seulement dans l'archive.
fn installation_apres_maj_python(nom: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("monitoring-lanceur-{nom}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(d.join(".maj")).unwrap();
    fs::copy(LANCEUR, d.join("lancer.sh")).unwrap();
    ecrire(&d.join("VERSION"), "2.0.5\n", false);
    ecrire(&d.join("serveur.py"), "", true);
    ecrire(&d.join("web/index.html"), "", false);
    ecrire(&d.join(".maj/a-confirmer"), "2.0.5\n", false);
    let p = d.join(".maj/precedente");
    ecrire(&p.join("serveur.py"), "print('version python')\n", true);
    ecrire(
        &p.join("lancer.sh"),
        "#!/bin/bash\n# lanceur python\n",
        true,
    );
    ecrire(&p.join("VERSION"), "1.0.3\n", false);
    ecrire(&p.join("config.example.toml"), "# exemple 1.0.3\n", false);
    ecrire(&p.join("web/index.html"), "<!doctype html>", false);

    let archive = File::create(d.join(".maj/monitoring-2.0.5.tar.gz")).unwrap();
    let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
        archive,
        flate2::Compression::fast(),
    ));
    let binaire = binaire_factice("2.0.5");
    let mut entete = tar::Header::new_gnu();
    entete.set_size(binaire.len() as u64);
    entete.set_mode(0o755);
    entete.set_cksum();
    tar.append_data(
        &mut entete,
        "monitoring-2.0.5/monitoring",
        binaire.as_bytes(),
    )
    .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    d
}

/// Lance lancer.sh ; le laisse tourner au plus `duree`, puis arrete tout son groupe de processus.
fn lancer(dossier: &Path, duree: Duration) -> Option<ExitStatus> {
    let mut processus = Command::new("bash")
        .arg(dossier.join("lancer.sh"))
        .process_group(0)
        .spawn()
        .unwrap();
    let fin = Instant::now() + duree;
    while Instant::now() < fin {
        if let Some(statut) = processus.try_wait().unwrap() {
            return Some(statut);
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _ = Command::new("kill")
        .arg("--")
        .arg(format!("-{}", processus.id()))
        .status();
    let _ = processus.wait();
    None
}

fn lire(chemin: PathBuf) -> String {
    fs::read_to_string(chemin).unwrap_or_default()
}

fn version_du_binaire(dossier: &Path) -> String {
    let sortie = Command::new(dossier.join("monitoring"))
        .arg("--version")
        .output()
        .unwrap();
    String::from_utf8_lossy(&sortie.stdout).trim().to_string()
}

#[test]
fn passage_depuis_la_version_python() {
    let d = installation_apres_maj_python("passage");
    // la nouvelle version doit tenir 15 s avant d'etre confirmee
    assert_eq!(
        lancer(&d, Duration::from_secs(19)),
        None,
        "lancer.sh s'est arrete :\n{}",
        lire(d.join("serveur.log"))
    );
    assert_eq!(version_du_binaire(&d), "2.0.5");
    assert!(!d.join(".maj/a-confirmer").exists());
    assert!(!d.join("serveur.py").exists() && !d.join("web").exists());
    assert_eq!(lire(d.join(".maj/precedente/VERSION")), "1.0.3\n");
    assert!(lire(d.join("serveur.log")).contains("version 2.0.5 demarree"));
}

#[test]
fn retour_a_la_version_python_si_l_ecran_ne_demarre_pas() {
    let d = installation_apres_maj_python("ecran");
    ecrire(&d.join("ecran-ko"), "", false);
    let statut = lancer(&d, Duration::from_secs(15)).expect("lancer.sh aurait du s'arreter");
    assert!(!statut.success());
    assert_eq!(lire(d.join("VERSION")), "1.0.3\n");
    assert_eq!(lire(d.join("serveur.py")), "print('version python')\n");
    assert_eq!(lire(d.join("lancer.sh")), "#!/bin/bash\n# lanceur python\n");
    assert_eq!(lire(d.join("web/index.html")), "<!doctype html>");
    assert!(!d.join(".maj/a-confirmer").exists());
    let raison = lire(d.join(".maj/retour-arriere"));
    assert!(
        raison.contains(
            "la version 2.0.5 n'a pas démarré (l'écran s'est arrêté : bibliotheque absente)"
        ),
        "{raison}"
    );
}

#[test]
fn retour_a_la_version_rust_precedente_si_le_serveur_ne_repond_pas() {
    let d = installation_apres_maj_python("serveur");
    // la version precedente est cette fois une version Rust
    let p = d.join(".maj/precedente");
    fs::remove_file(p.join("serveur.py")).unwrap();
    fs::remove_dir_all(p.join("web")).unwrap();
    ecrire(&p.join("monitoring"), &binaire_factice("2.0.4"), true);
    ecrire(&p.join("VERSION"), "2.0.4\n", false);
    ecrire(&d.join("serveur-ko"), "", false);
    let statut = lancer(&d, Duration::from_secs(45)).expect("lancer.sh aurait du s'arreter");
    assert!(!statut.success());
    assert_eq!(version_du_binaire(&d), "2.0.4");
    assert_eq!(lire(d.join("VERSION")), "2.0.4\n");
    assert!(lire(d.join(".maj/retour-arriere")).contains("le serveur ne répond pas"));
}
