use anyhow::Result;
use rusqlite::Connection;
use std::{fs, path::Path, sync::Mutex};

pub struct Db(pub Mutex<Connection>);

pub fn init(path: impl AsRef<Path>) -> Result<Db> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        PRAGMA foreign_keys=ON;

        CREATE TABLE IF NOT EXISTS skills (
            id              TEXT PRIMARY KEY,
            name            TEXT NOT NULL UNIQUE,
            mean            REAL NOT NULL DEFAULT 50.0,
            variance        REAL NOT NULL DEFAULT 400.0,
            process_noise   REAL NOT NULL DEFAULT 1.5,
            position        INTEGER NOT NULL,
            created_at      INTEGER NOT NULL,
            last_updated    INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS observations (
            id                  TEXT PRIMARY KEY,
            skill_id            TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            score               REAL NOT NULL,
            measurement_noise   REAL NOT NULL,
            source              TEXT NOT NULL,
            note                TEXT,
            resulting_mean      REAL NOT NULL,
            resulting_variance  REAL NOT NULL,
            created_at          INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_obs_skill ON observations(skill_id);
        "#,
    )?;

    // Seed default vertices matching the dossier hexagon if table is empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0))?;
    if count == 0 {
        let defaults = [
            "Intelligence",
            "Creativity",
            "Willingness to Act",
            "Motivation",
            "Social Life",
            "Acting Skills",
        ];
        let now = chrono::Utc::now().timestamp();
        for (i, name) in defaults.iter().enumerate() {
            conn.execute(
                "INSERT INTO skills (id, name, mean, variance, process_noise, position, created_at, last_updated)
                 VALUES (?1, ?2, 50.0, 400.0, 1.5, ?3, ?4, ?4)",
                rusqlite::params![uuid::Uuid::new_v4().to_string(), name, i as i64, now],
            )?;
        }
    }

    Ok(Db(Mutex::new(conn)))
}
