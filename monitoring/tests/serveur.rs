//! Le vrai serveur demarre sur une config minimale, son API repond avec le contrat attendu, et il
//! reprend l'historique (data.db) laisse par la version precedente.

use std::fs::{self, File};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const BINAIRE: &str = env!("CARGO_BIN_EXE_monitoring");

struct Serveur {
    processus: Child,
    dossier: PathBuf,
    url: String,
    port: u16,
}

impl Drop for Serveur {
    fn drop(&mut self) {
        let _ = self.processus.kill();
        let _ = self.processus.wait();
    }
}

impl Serveur {
    fn demarrer(nom: &str, preparer_base: impl FnOnce(&Path)) -> Serveur {
        let dossier =
            std::env::temp_dir().join(format!("monitoring-serveur-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dossier);
        fs::create_dir_all(&dossier).unwrap();
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let config = format!(
            "[general]\nport = {port}\nintervalle = 5\n\n[[service]]\nnom = \"Local\"\ntag = \"Test\"\ntcp = \"127.0.0.1:{port}\"\n"
        );
        fs::write(dossier.join("config.toml"), config).unwrap();
        preparer_base(&dossier.join("data.db"));
        let journal = File::create(dossier.join("serveur.log")).unwrap();
        let processus = Command::new(BINAIRE)
            .arg("serveur")
            .env("MONITORING_CONFIG", dossier.join("config.toml"))
            .env("MONITORING_DB", dossier.join("data.db"))
            .stdout(journal.try_clone().unwrap())
            .stderr(journal)
            .spawn()
            .unwrap();
        Serveur {
            processus,
            url: format!("http://127.0.0.1:{port}"),
            dossier,
            port,
        }
    }

    fn journal(&self) -> String {
        fs::read_to_string(self.dossier.join("serveur.log")).unwrap_or_default()
    }

    /// Attend que le premier controle du service local soit passe.
    fn attendre_service_en_ligne(&mut self) -> Value {
        let fin = Instant::now() + Duration::from_secs(30);
        while Instant::now() < fin {
            assert!(
                self.processus.try_wait().unwrap().is_none(),
                "le serveur s'est arrete :\n{}",
                self.journal()
            );
            if let Ok((200, etat)) = requete(&format!("{}/api/etat", self.url), None)
                && etat["services"][0]["etat"] == "up"
            {
                return etat;
            }
            thread::sleep(Duration::from_millis(300));
        }
        panic!(
            "le controle TCP local n'est pas passe :\n{}",
            self.journal()
        );
    }
}

/// GET, ou POST si un corps est donne ; (code, JSON).
fn requete(url: &str, corps: Option<&str>) -> Result<(u16, Value), ureq::Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .http_status_as_error(false)
        .proxy(None)
        .build()
        .into();
    let mut reponse = match corps {
        None => agent.get(url).call()?,
        Some(c) => agent
            .post(url)
            .header("Content-Type", "application/json")
            .send(c)?,
    };
    let code = reponse.status().as_u16();
    let valeur = serde_json::from_reader(reponse.body_mut().as_reader()).unwrap_or(Value::Null);
    Ok((code, valeur))
}

fn maintenant() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn le_serveur_demarre_et_son_api_repond() {
    let mut s = Serveur::demarrer("api", |_| {});
    let etat = s.attendre_service_en_ligne();
    let service = &etat["services"][0];
    assert_eq!(service["id"], "local");
    assert_eq!(service["tag"], "Test");
    assert_eq!(
        service["nodes"][0]["info"],
        format!("TCP {} ouvert", s.port)
    );
    assert_eq!(service["nodes"][0]["type"], "tcp");
    assert_eq!(service["spark"].as_array().unwrap().len(), 72);
    assert_eq!(etat["intervalle"], 5);
    assert_eq!(etat["tailscale"]["api"]["configuree"], false);
    assert_eq!(etat["maj"]["actuelle"], "dev");
    assert_eq!(etat["maj"]["statut"], "desactive");

    let (code, detail) = requete(&format!("{}/api/service/local", s.url), None).unwrap();
    assert_eq!(code, 200);
    assert_eq!(detail["jours"].as_array().unwrap().len(), 90);
    assert!(detail["courbes"]["local/Local"].is_array());
    assert_eq!(
        requete(&format!("{}/api/service/inconnu", s.url), None)
            .unwrap()
            .0,
        404
    );

    let (code, incidents) = requete(&format!("{}/api/incidents", s.url), None).unwrap();
    assert_eq!((code, &incidents["bilan"]["nombre"]), (200, &json!(0)));

    let (code, reponse) = requete(
        &format!("{}/api/maj/installer", s.url),
        Some(r#"{"version": "9.9.9"}"#),
    )
    .unwrap();
    assert_eq!(
        (code, &reponse["erreur"]),
        (409, &json!("Aucune mise à jour prête pour cette version"))
    );
    assert_eq!(
        requete(&format!("{}/api/maj/installer", s.url), Some("pas du json"))
            .unwrap()
            .0,
        400
    );
    assert_eq!(
        requete(&format!("{}/api/controler/local", s.url), Some("")).unwrap(),
        (200, json!({"ok": true}))
    );
    assert_eq!(
        requete(&format!("{}/api/inexistant", s.url), None)
            .unwrap()
            .0,
        404
    );

    let sonde = Command::new(BINAIRE)
        .arg("sonde")
        .env("MONITORING_CONFIG", s.dossier.join("config.toml"))
        .status();
    assert!(sonde.unwrap().success());
}

#[test]
fn l_historique_de_la_version_precedente_est_repris() {
    let il_y_a_10_jours = maintenant() - 10 * 86_400;
    let mut s = Serveur::demarrer("historique", |chemin| {
        // schema et contenu tels que les ecrivait la version Python
        let base = rusqlite::Connection::open(chemin).unwrap();
        base.execute_batch(
            "CREATE TABLE mesures (ts INTEGER NOT NULL, node TEXT NOT NULL, ok INTEGER NOT NULL, ms REAL);
             CREATE INDEX mesures_ts ON mesures(ts);
             CREATE TABLE incidents (id INTEGER PRIMARY KEY AUTOINCREMENT, service TEXT NOT NULL, nom TEXT NOT NULL,
                 niveau TEXT NOT NULL, message TEXT NOT NULL, debut INTEGER NOT NULL, fin INTEGER,
                 acquitte INTEGER NOT NULL DEFAULT 0, panne_s INTEGER NOT NULL DEFAULT 0, panne_depuis INTEGER);
             CREATE TABLE meta (cle TEXT PRIMARY KEY, valeur TEXT);",
        )
        .unwrap();
        let debut_panne = maintenant() - 3600;
        base.execute(
            "INSERT INTO meta VALUES ('debut:local', ?1)",
            [il_y_a_10_jours.to_string()],
        )
        .unwrap();
        base.execute(
            "INSERT INTO incidents (service, nom, niveau, message, debut, panne_depuis) VALUES
             ('local', 'Local', 'down', 'Ne répond plus : Connexion refusée', ?1, ?1),
             ('ancien', 'Ancien', 'warn', 'retire de la config', ?1, NULL)",
            [debut_panne],
        )
        .unwrap();
    });
    let etat = s.attendre_service_en_ligne();
    // une heure de panne sur 10 jours de suivi
    let dispo = etat["services"][0]["dispo30"].as_f64().unwrap();
    assert!((99.5..99.7).contains(&dispo), "dispo30 = {dispo}");

    let (_, incidents) = requete(&format!("{}/api/incidents", s.url), None).unwrap();
    let liste = incidents["incidents"].as_array().unwrap();
    assert_eq!(liste.len(), 2);
    for i in liste {
        assert!(i["fin"].is_i64(), "incident non clos : {i}");
    }
    let local = liste.iter().find(|i| i["service"] == "local").unwrap();
    assert!(local["panne_s"].as_i64().unwrap() >= 3600);
    assert!(etat["incidents_ouverts"].as_array().unwrap().is_empty());
}
