//! Emplacement des fichiers de l'installation : tout est a cote du binaire
//! (`/root/monitoring/` sur la tablette).

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Chemins {
    /// Dossier de l'installation (celui du binaire).
    pub dossier: PathBuf,
    /// `config.toml`, ou `$MONITORING_CONFIG`.
    pub config: PathBuf,
    /// Historique SQLite `data.db`, ou `$MONITORING_DB`.
    pub base: PathBuf,
    /// Archives telechargees, sauvegarde de la version precedente, marqueurs de mise a jour.
    pub maj: PathBuf,
    /// Version installee, ecrite dans chaque release par la CI (absent = installation manuelle).
    pub version: PathBuf,
}

impl Chemins {
    pub fn depuis_environnement() -> std::io::Result<Chemins> {
        let binaire = std::env::current_exe()?.canonicalize()?;
        let dossier = binaire.parent().map(Path::to_path_buf).unwrap_or_default();
        let variable = |nom: &str| std::env::var_os(nom).map(PathBuf::from);
        Ok(Chemins {
            config: variable("MONITORING_CONFIG").unwrap_or_else(|| dossier.join("config.toml")),
            base: variable("MONITORING_DB").unwrap_or_else(|| dossier.join("data.db")),
            maj: dossier.join(".maj"),
            version: dossier.join("VERSION"),
            dossier,
        })
    }
}
