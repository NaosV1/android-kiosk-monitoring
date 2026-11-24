//! Historique SQLite (`data.db`) : mesures recentes, incidents et debut du suivi de chaque service.
//! Meme schema que la version Python : l'historique est conserve d'une version a l'autre.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::api::{Incident, Niveau};

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS mesures (ts INTEGER NOT NULL, node TEXT NOT NULL,
        ok INTEGER NOT NULL, ms REAL);
    CREATE INDEX IF NOT EXISTS mesures_ts ON mesures(ts);
    CREATE TABLE IF NOT EXISTS incidents (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        service TEXT NOT NULL, nom TEXT NOT NULL,
        niveau TEXT NOT NULL, message TEXT NOT NULL,
        debut INTEGER NOT NULL, fin INTEGER,
        acquitte INTEGER NOT NULL DEFAULT 0,
        panne_s INTEGER NOT NULL DEFAULT 0, panne_depuis INTEGER);
    CREATE TABLE IF NOT EXISTS meta (cle TEXT PRIMARY KEY, valeur TEXT);
";

const JOUR: i64 = 86_400;

/// Moyenne des temps de reponse d'une node sur une tranche de temps.
pub struct Moyenne {
    pub node: String,
    pub tranche: i64,
    /// `None` si aucun controle n'a reussi dans la tranche.
    pub ms: Option<f64>,
}

pub struct Base(Connection);

impl Base {
    pub fn ouvrir(chemin: &Path) -> rusqlite::Result<Base> {
        let connexion = Connection::open(chemin)?;
        connexion.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
        connexion.execute_batch(SCHEMA)?;
        Ok(Base(connexion))
    }

    /// Debut du suivi d'un service (fixe au premier demarrage qui le voit) : un service ajoute
    /// plus tard ne compte pas comme disponible avant d'avoir ete controle.
    pub fn debut_suivi(&self, service: &str, maintenant: i64) -> rusqlite::Result<i64> {
        let cle = format!("debut:{service}");
        self.0.execute(
            "INSERT OR IGNORE INTO meta VALUES (?1, ?2)",
            params![cle, maintenant.to_string()],
        )?;
        let valeur: String =
            self.0
                .query_row("SELECT valeur FROM meta WHERE cle = ?1", [&cle], |l| {
                    l.get(0)
                })?;
        Ok(valeur.parse().unwrap_or(maintenant))
    }

    /// (id, service) des incidents encore ouverts.
    pub fn incidents_ouverts(&self) -> rusqlite::Result<Vec<(i64, String)>> {
        let mut requete = self
            .0
            .prepare("SELECT id, service FROM incidents WHERE fin IS NULL")?;
        requete
            .query_map([], |l| Ok((l.get(0)?, l.get(1)?)))?
            .collect()
    }

    pub fn ouvrir_incident(
        &self,
        service: &str,
        nom: &str,
        niveau: Niveau,
        message: &str,
        maintenant: i64,
    ) -> rusqlite::Result<i64> {
        let panne_depuis = (niveau == Niveau::Down).then_some(maintenant);
        self.0.execute(
            "INSERT INTO incidents (service, nom, niveau, message, debut, panne_depuis) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![service, nom, niveau.texte(), message, maintenant, panne_depuis],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    /// Incident toujours en cours : met a jour le message et le temps de panne totale.
    /// Le niveau ne redescend jamais (un incident passe par `down` le reste).
    pub fn prolonger_incident(
        &self,
        id: i64,
        niveau: Niveau,
        message: &str,
        maintenant: i64,
    ) -> rusqlite::Result<()> {
        let (niveau_actuel, mut panne_s, mut panne_depuis): (String, i64, Option<i64>) =
            self.0.query_row(
                "SELECT niveau, panne_s, panne_depuis FROM incidents WHERE id = ?1",
                [id],
                |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?)),
            )?;
        let mut niveau_retenu = niveau_actuel;
        if niveau == Niveau::Down {
            niveau_retenu = Niveau::Down.texte().into();
            panne_depuis = panne_depuis.or(Some(maintenant));
        } else if let Some(depuis) = panne_depuis.take() {
            panne_s += maintenant - depuis;
        }
        self.0.execute(
            "UPDATE incidents SET niveau = ?1, message = ?2, panne_s = ?3, panne_depuis = ?4 WHERE id = ?5",
            params![niveau_retenu, message, panne_s, panne_depuis, id],
        )?;
        Ok(())
    }

    pub fn fermer_incident(&self, id: i64, maintenant: i64) -> rusqlite::Result<()> {
        let ligne: Option<(i64, Option<i64>)> = self
            .0
            .query_row(
                "SELECT panne_s, panne_depuis FROM incidents WHERE id = ?1",
                [id],
                |l| Ok((l.get(0)?, l.get(1)?)),
            )
            .optional()?;
        let Some((panne_s, panne_depuis)) = ligne else {
            return Ok(());
        };
        let panne_s = panne_s + panne_depuis.map_or(0, |d| maintenant - d);
        self.0.execute(
            "UPDATE incidents SET fin = ?1, panne_s = ?2, panne_depuis = NULL WHERE id = ?3",
            params![maintenant, panne_s, id],
        )?;
        Ok(())
    }

    pub fn acquitter(&self, id: i64) -> rusqlite::Result<()> {
        self.0
            .execute("UPDATE incidents SET acquitte = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn ajouter_mesure(
        &self,
        ts: i64,
        node: &str,
        ok: bool,
        ms: Option<f64>,
    ) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO mesures VALUES (?1, ?2, ?3, ?4)",
            params![ts, node, ok, ms],
        )?;
        Ok(())
    }

    /// Les courbes couvrent 24 h ; les incidents clos sont gardes 120 jours.
    pub fn menage(&self, maintenant: i64) -> rusqlite::Result<()> {
        self.0.execute(
            "DELETE FROM mesures WHERE ts < ?1",
            [maintenant - JOUR - 7200],
        )?;
        self.0.execute(
            "DELETE FROM incidents WHERE fin IS NOT NULL AND fin < ?1",
            [maintenant - 120 * JOUR],
        )?;
        Ok(())
    }

    /// Moyenne des temps de reponse de chaque node par tranche de `largeur` secondes depuis `debut`.
    pub fn moyennes(&self, debut: i64, largeur: i64) -> rusqlite::Result<Vec<Moyenne>> {
        let mut requete = self.0.prepare(
            "SELECT node, (ts - ?1) / ?2, AVG(ms), SUM(ok) FROM mesures WHERE ts >= ?1 GROUP BY node, (ts - ?1) / ?2",
        )?;
        requete
            .query_map(params![debut, largeur], |l| {
                let reussis: i64 = l.get(3)?;
                Ok(Moyenne {
                    node: l.get(0)?,
                    tranche: l.get(1)?,
                    ms: if reussis > 0 { l.get(2)? } else { None },
                })
            })?
            .collect()
    }

    /// Incidents qui chevauchent [debut, fin[, du plus recent au plus ancien.
    pub fn incidents_entre(
        &self,
        debut: i64,
        fin: i64,
        service: Option<&str>,
    ) -> rusqlite::Result<Vec<Incident>> {
        let mut requete = self.0.prepare(
            "SELECT id, service, nom, niveau, message, debut, fin, acquitte, panne_s, panne_depuis FROM incidents
             WHERE debut < ?1 AND (fin IS NULL OR fin > ?2) AND (?3 IS NULL OR service = ?3) ORDER BY debut DESC",
        )?;
        requete
            .query_map(params![fin, debut, service], |l| {
                let niveau: String = l.get(3)?;
                Ok(Incident {
                    id: l.get(0)?,
                    service: l.get(1)?,
                    nom: l.get(2)?,
                    niveau: Niveau::depuis_texte(&niveau).unwrap_or(Niveau::Warn),
                    message: l.get(4)?,
                    debut: l.get(5)?,
                    fin: l.get(6)?,
                    acquitte: l.get(7)?,
                    panne_s: l.get(8)?,
                    panne_depuis: l.get(9)?,
                })
            })?
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Base {
        Base::ouvrir(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn duree_de_panne_d_un_incident() {
        let b = base();
        let id = b
            .ouvrir_incident("api", "API", Niveau::Warn, "lent", 1000)
            .unwrap();
        b.prolonger_incident(id, Niveau::Down, "hors ligne", 1100)
            .unwrap();
        b.prolonger_incident(id, Niveau::Warn, "une node revient", 1160)
            .unwrap();
        b.prolonger_incident(id, Niveau::Down, "hors ligne", 1200)
            .unwrap();
        b.fermer_incident(id, 1230).unwrap();
        let i = &b.incidents_entre(0, 2000, Some("api")).unwrap()[0];
        assert_eq!(
            (i.niveau, i.panne_s, i.panne_depuis, i.fin),
            (Niveau::Down, 90, None, Some(1230))
        );
        assert_eq!(i.message, "hors ligne");
        assert!(
            b.incidents_entre(0, 2000, Some("autre"))
                .unwrap()
                .is_empty()
        );
        assert!(b.incidents_ouverts().unwrap().is_empty());
    }

    #[test]
    fn le_debut_du_suivi_est_fixe_au_premier_passage() {
        let b = base();
        assert_eq!(b.debut_suivi("api", 100).unwrap(), 100);
        assert_eq!(b.debut_suivi("api", 500).unwrap(), 100);
    }

    #[test]
    fn moyennes_par_tranche() {
        let b = base();
        b.ajouter_mesure(1000, "api/a", true, Some(10.0)).unwrap();
        b.ajouter_mesure(1100, "api/a", true, Some(30.0)).unwrap();
        b.ajouter_mesure(2300, "api/a", false, None).unwrap();
        let mut m = b.moyennes(1000, 1200).unwrap();
        m.sort_by_key(|m| m.tranche);
        assert_eq!(
            m.iter().map(|m| (m.tranche, m.ms)).collect::<Vec<_>>(),
            [(0, Some(20.0)), (1, None)]
        );
    }
}
