//! Mises a jour depuis les releases GitHub.
//!
//! La CI publie une release par push sur main : `monitoring-<version>.tar.gz` + `SHA256SUMS`
//! (format decrit dans .github/workflows/ci.yml). Le serveur cherche periodiquement la derniere
//! release, la telecharge, controle son empreinte et son contenu, puis attend qu'on appuie sur
//! « Mettre a jour » (avec confirmation). L'installation garde config.toml et data.db, sauvegarde
//! la version en place dans .maj/precedente, verifie que le nouveau binaire s'execute, remplace
//! les fichiers puis relance l'appli ; lancer.sh remet la version precedente si la nouvelle ne
//! demarre pas.

use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, File, Permissions};
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use chrono::DateTime;
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::api::{DerniereInstallation, Maj, StatutMaj, VersionDisponible};
use crate::chemins::Chemins;
use crate::config::Config;
use crate::https::{self, tronquer};
use crate::moniteur::maintenant;

/// Taille maximale d'une archive ; l'installateur des versions Python 1.0.x a la meme limite.
const TAILLE_MAX: u64 = 30 * 1024 * 1024;
/// Fichiers de l'installation remplaces par une mise a jour (config.toml et data.db n'y sont pas).
const FICHIERS: [&str; 4] = ["monitoring", "lancer.sh", "config.example.toml", "VERSION"];
const EXECUTABLES: [&str; 2] = ["monitoring", "lancer.sh"];
/// Indispensables dans une archive.
const REQUIS: [&str; 3] = ["monitoring", "lancer.sh", "VERSION"];
const DELAI_RESEAU: Duration = Duration::from_secs(30);

type Resultat<T> = Result<T, Box<dyn Error>>;

pub struct MisesAJour {
    depot: String,
    periode: Duration,
    chemins: Chemins,
    etat: Mutex<Maj>,
    /// Demande de verification immediate (« Verifier maintenant »).
    reveil: (Mutex<bool>, Condvar),
}

enum Recherche {
    AJour,
    Prete(VersionDisponible),
}

#[derive(Deserialize)]
struct Release {
    #[serde(default)]
    tag_name: String,
    published_at: Option<String>,
    body: Option<String>,
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<FichierRelease>,
}

#[derive(Deserialize)]
struct FichierRelease {
    name: String,
    browser_download_url: String,
}

impl MisesAJour {
    pub fn new(conf: &Config, chemins: Chemins) -> MisesAJour {
        // message laisse par lancer.sh si une nouvelle version n'a pas demarre
        let retour = fs::read_to_string(chemins.maj.join("retour-arriere"))
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        let derniere = fs::read_to_string(chemins.maj.join("derniere-installation.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
        let depot = conf.maj.depot.clone();
        let etat = Maj {
            actuelle: version_locale(&chemins.version),
            depot: depot.clone(),
            heures: conf.maj.heures,
            statut: if depot.is_empty() {
                StatutMaj::Desactive
            } else {
                StatutMaj::Inconnu
            },
            verifie: None,
            en_cours: false,
            erreur: retour,
            disponible: None,
            derniere_installation: derniere,
        };
        MisesAJour {
            depot,
            periode: Duration::from_secs_f64(conf.maj.heures * 3600.0),
            chemins,
            etat: Mutex::new(etat),
            reveil: (Mutex::new(false), Condvar::new()),
        }
    }

    fn verrou(&self) -> MutexGuard<'_, Maj> {
        self.etat.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn resume(&self) -> Maj {
        self.verrou().clone()
    }

    /// Ne rend jamais la main (sauf mises a jour desactivees).
    pub fn boucle(&self) {
        if self.depot.is_empty() {
            return;
        }
        self.attendre(Duration::from_secs(60)); // laisse le reseau se mettre en place au demarrage
        loop {
            self.verifier();
            self.attendre(self.periode);
        }
    }

    pub fn verifier_maintenant(&self) {
        let (demande, condition) = &self.reveil;
        *demande.lock().unwrap_or_else(PoisonError::into_inner) = true;
        condition.notify_all();
    }

    fn attendre(&self, duree: Duration) {
        let (demande, condition) = &self.reveil;
        let garde = demande.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut garde, _) = condition
            .wait_timeout_while(garde, duree, |d| !*d)
            .unwrap_or_else(PoisonError::into_inner);
        *garde = false;
    }

    /// Cherche la derniere release ; si elle est plus recente, la telecharge et la verifie.
    fn verifier(&self) {
        {
            let mut etat = self.verrou();
            if matches!(
                etat.statut,
                StatutMaj::Installation | StatutMaj::Redemarrage
            ) {
                return;
            }
            etat.en_cours = true;
        }
        let recherche = self.chercher();
        let mut etat = self.verrou();
        match recherche {
            Ok(Recherche::AJour) => {
                etat.statut = StatutMaj::AJour;
                etat.disponible = None;
                etat.erreur = None;
            }
            Ok(Recherche::Prete(disponible)) => {
                if etat.statut != StatutMaj::Installation {
                    etat.statut = StatutMaj::Prete;
                    etat.disponible = Some(disponible);
                    etat.erreur = None;
                }
            }
            Err(message) => {
                etat.erreur = Some(tronquer(&message, 200));
                if etat.disponible.is_none() || etat.statut == StatutMaj::Telechargement {
                    etat.statut = StatutMaj::Erreur;
                }
            }
        }
        etat.en_cours = false;
        etat.verifie = Some(maintenant());
    }

    fn chercher(&self) -> Result<Recherche, String> {
        let agent = https::agent(DELAI_RESEAU, true, None);
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.depot
        );
        let mut reponse = agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "tablette-monitoring")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|e| {
                format!(
                    "GitHub injoignable ({})",
                    https::raison(&e, DELAI_RESEAU.as_secs_f64())
                )
            })?;
        match reponse.status().as_u16() {
            404 => return Ok(Recherche::AJour),
            200..=299 => {}
            code => return Err(format!("GitHub : erreur HTTP {code}")),
        }
        let release: Release = serde_json::from_reader(reponse.body_mut().as_reader())
            .map_err(|e| format!("GitHub : réponse illisible ({e})"))?;

        let version = release.tag_name.trim_start_matches(['v', 'V']).to_string();
        if cle_version(&version).is_empty() {
            return Err(format!(
                "Release sans numéro de version lisible ({})",
                tronquer(&release.tag_name, 30)
            ));
        }
        if cle_version(&version) <= cle_version(&self.verrou().actuelle) {
            return Ok(Recherche::AJour);
        }
        let disponible = VersionDisponible {
            version: version.clone(),
            publiee: release
                .published_at
                .as_deref()
                .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
                .map(|d| d.timestamp()),
            notes: tronquer(release.body.as_deref().unwrap_or(""), 4000),
            url: release.html_url.clone(),
        };
        let nom = format!("monitoring-{version}.tar.gz");
        let lien = |fichier: &str| {
            release
                .assets
                .iter()
                .find(|a| a.name == fichier)
                .map(|a| a.browser_download_url.as_str())
        };
        let (Some(lien_archive), Some(lien_sommes)) = (lien(&nom), lien("SHA256SUMS")) else {
            return Err(format!(
                "Release v{version} incomplète (archive ou SHA256SUMS manquant)"
            ));
        };
        let sommes = telecharger_texte(&agent, lien_sommes)?;
        let attendu = empreinte_dans(&sommes, &nom)
            .ok_or_else(|| format!("Empreinte de {nom} absente de SHA256SUMS"))?;

        let dossier = &self.chemins.maj;
        fs::create_dir_all(dossier).map_err(|e| format!("Dossier {} : {e}", dossier.display()))?;
        let archive = dossier.join(&nom);
        if sha256(&archive).ok().as_deref() != Some(attendu.as_str()) {
            {
                let mut etat = self.verrou();
                etat.statut = StatutMaj::Telechargement;
                etat.disponible = Some(disponible.clone());
            }
            telecharger(&agent, lien_archive, &archive)?;
            if sha256(&archive).ok().as_deref() != Some(attendu.as_str()) {
                let _ = fs::remove_file(&archive);
                return Err("Archive téléchargée corrompue (empreinte différente)".into());
            }
            journal!("mise a jour v{version} telechargee et verifiee");
        }
        controler_archive(&archive, &version)?;
        supprimer_autres_archives(dossier, &nom);
        Ok(Recherche::Prete(disponible))
    }

    /// Lance l'installation de `version`, qui doit etre exactement celle proposee a l'ecran.
    pub fn installer(self: &Arc<Self>, version: &str) -> Result<(), String> {
        {
            let mut etat = self.verrou();
            let prete = etat.statut == StatutMaj::Prete
                && etat
                    .disponible
                    .as_ref()
                    .is_some_and(|d| d.version == version);
            if !prete {
                return Err("Aucune mise à jour prête pour cette version".into());
            }
            etat.statut = StatutMaj::Installation;
        }
        let moi = Arc::clone(self);
        let version = version.to_string();
        thread::spawn(move || moi.installer_version(&version));
        Ok(())
    }

    fn installer_version(&self, version: &str) {
        let ancienne = self.verrou().actuelle.clone();
        let extrait = self.chemins.maj.join(format!("extrait-{version}"));
        let precedente = self.chemins.maj.join("precedente");
        let mut sauvegarde_faite = false;
        let resultat = self.remplacer_installation(
            version,
            &ancienne,
            &extrait,
            &precedente,
            &mut sauvegarde_faite,
        );
        let _ = fs::remove_dir_all(&extrait);
        match resultat {
            Ok(()) => {
                journal!("mise a jour installee : {ancienne} -> {version}, relance de l'appli");
                let relancee = relancer_appli();
                let mut etat = self.verrou();
                etat.statut = StatutMaj::Redemarrage;
                if !relancee {
                    etat.erreur =
                        Some("Installée : relance l'appli à la main (kiosk-restart)".into());
                }
            }
            Err(e) => {
                journal!("echec de la mise a jour : {e}");
                if sauvegarde_faite && let Err(e) = self.restaurer(&precedente) {
                    journal!("restauration impossible : {e}");
                }
                let mut etat = self.verrou();
                etat.statut = StatutMaj::Prete;
                etat.erreur = Some(tronquer(&format!("Installation impossible : {e}"), 200));
            }
        }
    }

    fn remplacer_installation(
        &self,
        version: &str,
        ancienne: &str,
        extrait: &Path,
        precedente: &Path,
        sauvegarde_faite: &mut bool,
    ) -> Resultat<()> {
        let dossier = &self.chemins.dossier;
        let maj = &self.chemins.maj;
        let archive = maj.join(format!("monitoring-{version}.tar.gz"));
        controler_archive(&archive, version)?;
        let _ = fs::remove_dir_all(extrait);
        extraire(&archive, version, extrait)?;
        if fs::read_to_string(extrait.join("VERSION"))?.trim() != version {
            return Err("VERSION de l'archive incohérente".into());
        }
        verifier_binaire(&extrait.join("monitoring"), version)?;

        // sauvegarde de la version en place, pour le retour arriere de lancer.sh
        let _ = fs::remove_dir_all(precedente);
        fs::create_dir_all(precedente)?;
        for nom in FICHIERS {
            let source = dossier.join(nom);
            if source.is_file() {
                fs::copy(&source, precedente.join(nom))?;
            }
        }
        fs::write(precedente.join("VERSION"), format!("{ancienne}\n"))?;
        *sauvegarde_faite = true;
        fs::write(maj.join("a-confirmer"), format!("{version}\n"))?;

        for nom in FICHIERS {
            let neuf = extrait.join(nom);
            if neuf.is_file() {
                remplacer(&neuf, &dossier.join(nom))?;
            }
        }
        let installation = DerniereInstallation {
            version: version.into(),
            depuis: ancienne.into(),
            quand: maintenant(),
        };
        fs::write(
            maj.join("derniere-installation.json"),
            serde_json::to_string(&installation)?,
        )?;
        let _ = fs::remove_file(maj.join("retour-arriere"));
        Ok(())
    }

    /// Remet la version sauvegardee telle qu'elle etait (installation interrompue).
    fn restaurer(&self, precedente: &Path) -> Resultat<()> {
        for nom in FICHIERS {
            let sauvegarde = precedente.join(nom);
            if sauvegarde.is_file() {
                remplacer(&sauvegarde, &self.chemins.dossier.join(nom))?;
            }
        }
        let _ = fs::remove_file(self.chemins.maj.join("a-confirmer"));
        Ok(())
    }
}

/// Version installee (fichier VERSION ecrit par la CI), `dev` pour une installation manuelle.
fn version_locale(chemin: &Path) -> String {
    fs::read_to_string(chemin)
        .map(|t| t.trim().to_string())
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "dev".into())
}

/// `"v1.0.12"` -> `[1, 0, 12]` ; version illisible (`dev`) -> `[]`, plus ancienne que toutes.
fn cle_version(version: &str) -> Vec<u64> {
    let v = version.trim().trim_start_matches(['v', 'V']);
    let parties: Option<Vec<u64>> = v
        .split('.')
        .map(|p| {
            (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse().ok())
                .flatten()
        })
        .collect();
    parties.unwrap_or_default()
}

/// Empreinte de `nom` dans un fichier SHA256SUMS (`<empreinte>  [*]<nom>` par ligne).
fn empreinte_dans(sommes: &str, nom: &str) -> Option<String> {
    sommes.lines().find_map(
        |ligne| match ligne.split_whitespace().collect::<Vec<_>>().as_slice() {
            [empreinte, fichier] if fichier.trim_start_matches('*') == nom => {
                Some(empreinte.to_lowercase())
            }
            _ => None,
        },
    )
}

fn sha256(chemin: &Path) -> io::Result<String> {
    let mut fichier = File::open(chemin)?;
    let mut calcul = Sha256::new();
    let mut bloc = vec![0; 1 << 16];
    loop {
        let n = fichier.read(&mut bloc)?;
        if n == 0 {
            break;
        }
        calcul.update(&bloc[..n]);
    }
    Ok(calcul
        .finalize()
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect())
}

fn telecharger_texte(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    let mut reponse = requete_fichier(agent, url)?;
    let mut texte = String::new();
    reponse
        .body_mut()
        .as_reader()
        .take(65_536)
        .read_to_string(&mut texte)
        .map_err(|e| {
            format!(
                "Téléchargement impossible ({})",
                https::raison_io(&e, DELAI_RESEAU.as_secs_f64())
            )
        })?;
    Ok(texte)
}

fn telecharger(agent: &ureq::Agent, url: &str, destination: &Path) -> Result<(), String> {
    let partiel = destination.with_extension("gz.part");
    let mut reponse = requete_fichier(agent, url)?;
    let copie = File::create(&partiel).and_then(|mut fichier| {
        io::copy(
            &mut reponse.body_mut().as_reader().take(TAILLE_MAX + 1),
            &mut fichier,
        )
    });
    let taille = copie.map_err(|e| {
        let _ = fs::remove_file(&partiel);
        format!(
            "Téléchargement impossible ({})",
            https::raison_io(&e, DELAI_RESEAU.as_secs_f64())
        )
    })?;
    if taille > TAILLE_MAX {
        let _ = fs::remove_file(&partiel);
        return Err("Archive trop grosse".into());
    }
    fs::rename(&partiel, destination).map_err(|e| format!("Téléchargement impossible ({e})"))
}

fn requete_fichier(
    agent: &ureq::Agent,
    url: &str,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let reponse = agent
        .get(url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", "tablette-monitoring")
        .call()
        .map_err(|e| {
            format!(
                "Téléchargement impossible ({})",
                https::raison(&e, DELAI_RESEAU.as_secs_f64())
            )
        })?;
    match reponse.status().as_u16() {
        200..=299 => Ok(reponse),
        code => Err(format!("GitHub : erreur HTTP {code}")),
    }
}

fn supprimer_autres_archives(dossier: &Path, garder: &str) {
    let Ok(entrees) = fs::read_dir(dossier) else {
        return;
    };
    for entree in entrees.flatten() {
        let nom = entree.file_name().to_string_lossy().into_owned();
        if nom.starts_with("monitoring-") && nom.contains(".tar.gz") && nom != garder {
            let _ = fs::remove_file(entree.path());
        }
    }
}

/// Chemin d'une entree de l'archive sous son dossier racine (`monitoring-<version>/...`).
/// Refuse tout ce qui sortirait de ce dossier.
fn chemin_dans_archive(chemin: &Path, racine: &str) -> Option<String> {
    let mut parties = chemin.components().filter(|c| *c != Component::CurDir);
    if parties.next()? != Component::Normal(racine.as_ref()) {
        return None;
    }
    let mut reste = Vec::new();
    for partie in parties {
        match partie {
            Component::Normal(p) => reste.push(p.to_str()?.to_string()),
            _ => return None,
        }
    }
    Some(reste.join("/"))
}

/// Refuse toute archive qui sortirait de son dossier, contiendrait des liens, ou serait incomplete.
fn controler_archive(archive: &Path, version: &str) -> Result<(), String> {
    let racine = format!("monitoring-{version}");
    let illisible = |e: io::Error| format!("Archive illisible ({e})");
    let mut lecteur = tar::Archive::new(GzDecoder::new(File::open(archive).map_err(illisible)?));
    let mut contenu = HashSet::new();
    for entree in lecteur.entries().map_err(illisible)? {
        let entree = entree.map_err(illisible)?;
        let chemin = entree.path().map_err(illisible)?.into_owned();
        let Some(relatif) = chemin_dans_archive(&chemin, &racine) else {
            return Err(format!(
                "Archive refusée : chemin inattendu ({})",
                tronquer(&chemin.to_string_lossy(), 60)
            ));
        };
        let type_entree = entree.header().entry_type();
        if !(type_entree.is_file() || type_entree.is_dir()) {
            return Err("Archive refusée : lien ou fichier spécial".into());
        }
        contenu.insert(relatif);
    }
    let manquants: Vec<&str> = REQUIS
        .into_iter()
        .filter(|f| !contenu.contains(*f))
        .collect();
    if !manquants.is_empty() {
        return Err(format!(
            "Archive incomplète : {} manquant",
            manquants.join(", ")
        ));
    }
    Ok(())
}

/// Extrait les fichiers de l'installation dans `destination` (deja controlee par `controler_archive`).
fn extraire(archive: &Path, version: &str, destination: &Path) -> Resultat<()> {
    let racine = format!("monitoring-{version}");
    fs::create_dir_all(destination)?;
    let mut lecteur = tar::Archive::new(GzDecoder::new(File::open(archive)?));
    for entree in lecteur.entries()? {
        let mut entree = entree?;
        let relatif = chemin_dans_archive(&entree.path()?, &racine);
        if let Some(nom) = relatif.filter(|r| FICHIERS.contains(&r.as_str()))
            && entree.header().entry_type().is_file()
        {
            entree.unpack(destination.join(nom))?;
        }
    }
    Ok(())
}

/// Le nouveau binaire doit s'executer sur cette tablette (architecture, bibliotheques) et
/// annoncer la version attendue, avant de toucher a l'installation en place.
fn verifier_binaire(binaire: &Path, version: &str) -> Resultat<()> {
    fs::set_permissions(binaire, Permissions::from_mode(0o755))?;
    let mut processus = Command::new(binaire)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("le nouveau binaire ne s'exécute pas sur cette tablette ({e})"))?;
    let fin = Instant::now() + Duration::from_secs(10);
    while processus.try_wait()?.is_none() {
        if Instant::now() >= fin {
            let _ = processus.kill();
            return Err("le nouveau binaire ne répond pas".into());
        }
        thread::sleep(Duration::from_millis(100));
    }
    let (mut annoncee, mut erreurs) = (String::new(), String::new());
    if let Some(sortie) = processus.stdout.as_mut() {
        let _ = sortie.read_to_string(&mut annoncee);
    }
    if annoncee.trim() != version {
        if let Some(sortie) = processus.stderr.as_mut() {
            let _ = sortie.read_to_string(&mut erreurs);
        }
        let detail = erreurs
            .lines()
            .next()
            .unwrap_or(annoncee.trim())
            .to_string();
        return Err(format!(
            "le nouveau binaire ne démarre pas ({})",
            tronquer(&detail, 120)
        )
        .into());
    }
    Ok(())
}

/// Copie `source` a la place de `destination` d'un seul coup (renommage atomique) : un binaire
/// en cours d'execution peut etre remplace.
fn remplacer(source: &Path, destination: &Path) -> io::Result<()> {
    let nom = destination
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let provisoire = destination.with_file_name(format!(".{nom}.nouveau"));
    fs::copy(source, &provisoire)?;
    let mode = if EXECUTABLES.contains(&nom.as_str()) {
        0o755
    } else {
        0o644
    };
    fs::set_permissions(&provisoire, Permissions::from_mode(mode))?;
    fs::rename(&provisoire, destination)
}

/// Relance l'appli (serveur + ecran) par kiosk-restart, depuis un processus detache qui survit
/// a l'arret du groupe de processus de l'appli.
fn relancer_appli() -> bool {
    let commande = chercher_commande("kiosk-restart")
        .unwrap_or_else(|| PathBuf::from("/usr/local/bin/kiosk-restart"));
    if !commande.is_file() {
        return false;
    }
    Command::new("sh")
        .args(["-c", "sleep 2; exec \"$0\""])
        .arg(&commande)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

fn chercher_commande(nom: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(nom))
        .find(|c| c.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_temporaire(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("monitoring-test-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Archive avec des entrees (nom brut, type, contenu).
    fn archive(dossier: &Path, entrees: &[(&str, tar::EntryType, &[u8])]) -> PathBuf {
        let chemin = dossier.join("archive.tar.gz");
        let gz = flate2::write::GzEncoder::new(
            File::create(&chemin).unwrap(),
            flate2::Compression::fast(),
        );
        let mut b = tar::Builder::new(gz);
        for (nom, type_entree, contenu) in entrees {
            let mut h = tar::Header::new_gnu();
            h.as_gnu_mut().unwrap().name[..nom.len()].copy_from_slice(nom.as_bytes());
            h.set_entry_type(*type_entree);
            h.set_size(contenu.len() as u64);
            h.set_mode(0o644);
            h.set_cksum();
            b.append(&h, *contenu).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
        chemin
    }

    const F: tar::EntryType = tar::EntryType::Regular;

    fn complete(
        extra: (&'static str, tar::EntryType, &'static [u8]),
    ) -> Vec<(&'static str, tar::EntryType, &'static [u8])> {
        vec![
            ("monitoring-2.0.1/", tar::EntryType::Directory, b""),
            ("monitoring-2.0.1/monitoring", F, b"binaire"),
            ("monitoring-2.0.1/lancer.sh", F, b"#!/bin/bash"),
            ("monitoring-2.0.1/VERSION", F, b"2.0.1\n"),
            extra,
        ]
    }

    #[test]
    fn comparaison_des_versions() {
        assert_eq!(cle_version("v1.0.12"), [1, 0, 12]);
        assert!(cle_version("dev").is_empty());
        assert!(cle_version("1.0.").is_empty());
        assert!(cle_version("1.0.10") > cle_version("1.0.9"));
        assert!(cle_version("1.0.9") > cle_version("dev"));
        assert!(cle_version("1.1.3") > cle_version("1.0.40"));
        assert!(cle_version("2.0.1") > cle_version("1.0.3"));
    }

    #[test]
    fn empreintes() {
        let sommes = "abc  autre.tar.gz\nDEF  *monitoring-2.0.1.tar.gz\n";
        assert_eq!(
            empreinte_dans(sommes, "monitoring-2.0.1.tar.gz").as_deref(),
            Some("def")
        );
        assert_eq!(empreinte_dans(sommes, "monitoring-2.0.2.tar.gz"), None);
        let d = dossier_temporaire("sha");
        fs::write(d.join("f"), b"abc").unwrap();
        assert_eq!(
            sha256(&d.join("f")).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn archive_valide_puis_extraite() {
        let d = dossier_temporaire("valide");
        let a = archive(&d, &complete(("monitoring-2.0.1/web/index.html", F, b"")));
        controler_archive(&a, "2.0.1").unwrap();
        extraire(&a, "2.0.1", &d.join("extrait")).unwrap();
        assert_eq!(
            fs::read_to_string(d.join("extrait/VERSION")).unwrap(),
            "2.0.1\n"
        );
        assert!(!d.join("extrait/web").exists());
    }

    #[test]
    fn archives_refusees() {
        let cas = [
            (
                ("monitoring-2.0.1/../../etc/passwd", F, b"" as &[u8]),
                "chemin inattendu",
            ),
            (("/etc/passwd", F, b""), "chemin inattendu"),
            (("autre/fichier", F, b""), "chemin inattendu"),
            (
                ("monitoring-2.0.1/lien", tar::EntryType::Symlink, b""),
                "lien ou fichier spécial",
            ),
        ];
        for (i, (extra, attendu)) in cas.into_iter().enumerate() {
            let d = dossier_temporaire(&format!("refus{i}"));
            let a = archive(&d, &complete(extra));
            let e = controler_archive(&a, "2.0.1").unwrap_err();
            assert!(e.contains(attendu), "{e}");
        }
        let d = dossier_temporaire("incomplete");
        let a = archive(&d, &[("monitoring-2.0.1/VERSION", F, b"2.0.1")]);
        assert_eq!(
            controler_archive(&a, "2.0.1").unwrap_err(),
            "Archive incomplète : monitoring, lancer.sh manquant"
        );
    }

    /// Installation 2.0.5 en place, et release 2.0.6 telechargee avec ce binaire.
    fn installation_avec_release(nom: &str, binaire: &'static [u8]) -> Arc<MisesAJour> {
        let d = dossier_temporaire(nom);
        for (fichier, contenu) in [
            ("monitoring", "#!/bin/sh\necho 2.0.5\n"),
            ("lancer.sh", "#!/bin/bash\n# 2.0.5\n"),
            ("VERSION", "2.0.5\n"),
            ("config.toml", "# config reelle\n"),
        ] {
            fs::write(d.join(fichier), contenu).unwrap();
        }
        fs::create_dir_all(d.join(".maj")).unwrap();
        let a = archive(
            &d,
            &[
                ("monitoring-2.0.6/monitoring", F, binaire),
                ("monitoring-2.0.6/lancer.sh", F, b"#!/bin/bash\n# 2.0.6\n"),
                (
                    "monitoring-2.0.6/config.example.toml",
                    F,
                    b"# exemple 2.0.6\n",
                ),
                ("monitoring-2.0.6/VERSION", F, b"2.0.6\n"),
                ("monitoring-2.0.6/serveur.py", F, b""),
                ("monitoring-2.0.6/web/index.html", F, b""),
            ],
        );
        fs::rename(a, d.join(".maj/monitoring-2.0.6.tar.gz")).unwrap();
        let chemins = Chemins {
            config: d.join("config.toml"),
            base: d.join("data.db"),
            maj: d.join(".maj"),
            version: d.join("VERSION"),
            dossier: d,
        };
        let conf = Config::depuis_texte("[mise_a_jour]\ndepot = \"proprietaire/depot\"").unwrap();
        let maj = Arc::new(MisesAJour::new(&conf, chemins));
        {
            let mut etat = maj.verrou();
            etat.statut = StatutMaj::Prete;
            etat.disponible = Some(VersionDisponible {
                version: "2.0.6".into(),
                publiee: None,
                notes: String::new(),
                url: None,
            });
        }
        maj
    }

    #[test]
    fn installation_d_une_nouvelle_version() {
        let maj = installation_avec_release("installation", b"#!/bin/sh\necho 2.0.6\n");
        let d = maj.chemins.dossier.clone();
        assert_eq!(maj.resume().actuelle, "2.0.5");
        maj.installer_version("2.0.6");

        let lire = |chemin: &str| fs::read_to_string(d.join(chemin)).unwrap();
        assert_eq!(lire("VERSION"), "2.0.6\n");
        assert_eq!(lire("monitoring"), "#!/bin/sh\necho 2.0.6\n");
        assert_eq!(
            fs::metadata(d.join("monitoring"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        assert_eq!(lire("config.example.toml"), "# exemple 2.0.6\n");
        assert_eq!(lire("config.toml"), "# config reelle\n");
        assert!(!d.join("serveur.py").exists() && !d.join("web").exists());
        assert_eq!(lire(".maj/precedente/VERSION"), "2.0.5\n");
        assert_eq!(
            lire(".maj/precedente/monitoring"),
            "#!/bin/sh\necho 2.0.5\n"
        );
        assert_eq!(lire(".maj/a-confirmer"), "2.0.6\n");
        assert!(!d.join(".maj/extrait-2.0.6").exists());
        let installation: DerniereInstallation =
            serde_json::from_str(&lire(".maj/derniere-installation.json")).unwrap();
        assert_eq!(
            (installation.version.as_str(), installation.depuis.as_str()),
            ("2.0.6", "2.0.5")
        );
        // pas de kiosk-restart hors de la tablette
        let etat = maj.resume();
        assert_eq!(etat.statut, StatutMaj::Redemarrage);
        assert_eq!(
            etat.erreur.as_deref(),
            Some("Installée : relance l'appli à la main (kiosk-restart)")
        );
    }

    #[test]
    fn un_binaire_qui_ne_demarre_pas_n_est_pas_installe() {
        let maj = installation_avec_release("binaire-ko", b"#!/bin/sh\necho 2.0.7\n");
        let d = maj.chemins.dossier.clone();
        maj.installer_version("2.0.6");

        assert_eq!(fs::read_to_string(d.join("VERSION")).unwrap(), "2.0.5\n");
        assert_eq!(
            fs::read_to_string(d.join("monitoring")).unwrap(),
            "#!/bin/sh\necho 2.0.5\n"
        );
        assert!(!d.join(".maj/a-confirmer").exists() && !d.join(".maj/precedente").exists());
        let etat = maj.resume();
        assert_eq!(etat.statut, StatutMaj::Prete);
        let erreur = etat.erreur.unwrap();
        assert!(
            erreur.starts_with("Installation impossible : le nouveau binaire ne démarre pas"),
            "{erreur}"
        );
    }
}
